use std::{
    sync::{
        mpsc::{self, Sender as EventSender},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::Duration,
};

use crossbeam_channel::{unbounded, Sender as ByteSender};
use jni::{
    objects::{GlobalRef, JObject, JString, JValue},
    sys::{jboolean, jbyte, jobject, jstring, JNIEnv as RawJNIEnv},
    JNIEnv, JavaVM,
};

enum BackendEvent {
    Connecting,
    Connected(bool),
    Disconnected,
    Read(u8),
    Error(String),
}

struct Backend {
    vm: Arc<JavaVM>,
    instance: GlobalRef,
}

static BACKEND: OnceLock<Mutex<Option<Backend>>> = OnceLock::new();
static EVENTS: OnceLock<EventSender<BackendEvent>> = OnceLock::new();

fn backend() -> Result<(Arc<JavaVM>, GlobalRef), String> {
    BACKEND
        .get()
        .and_then(|backend| backend.lock().ok())
        .and_then(|backend| {
            backend
                .as_ref()
                .map(|backend| (Arc::clone(&backend.vm), backend.instance.clone()))
        })
        .ok_or_else(|| "Android serial backend has not been initialized".to_owned())
}

fn invoke_backend(method: &str, signature: &str, args: &[JValue<'_, '_>]) -> Result<(), String> {
    let (vm, instance) = backend()?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| format!("Could not attach to Android runtime: {error}"))?;

    match env.call_method(instance.as_obj(), method, signature, args) {
        Ok(_) => Ok(()),
        Err(error) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            Err(format!("Android serial {method} failed: {error}"))
        }
    }
}

fn write_byte(byte: u8) -> Result<(), String> {
    invoke_backend("write", "(B)V", &[JValue::Byte(byte as jbyte)])
}

fn send_event(event: BackendEvent) {
    if let Some(events) = EVENTS.get() {
        if events.send(event).is_err() {
            eprintln!("Android serial event receiver has stopped");
        }
    } else {
        eprintln!("Android serial event received before the listener started");
    }
}

pub fn start_serialport_thread<
    FConnect: Fn(ByteSender<u8>) + Send + Sync + 'static,
    FDisconnectStatus: Fn(bool) + Send + Sync + 'static,
    FRead: Fn(u8) + Send + Sync + 'static,
    FError: Fn(String) + Send + Sync + 'static,
>(
    on_connect: FConnect,
    on_disconnect_status: FDisconnectStatus,
    on_read: FRead,
    on_error: FError,
) {
    let (event_tx, event_rx) = mpsc::channel();
    if EVENTS.set(event_tx).is_err() {
        return;
    }

    thread::spawn(move || {
        on_disconnect_status(false);

        loop {
            match invoke_backend("open", "()V", &[]) {
                Ok(()) => break,
                Err(error) => {
                    on_error(error);
                    thread::sleep(Duration::from_secs(1));
                }
            }
        }

        let mut writer: Option<thread::JoinHandle<()>> = None;
        let mut error_message: Option<Vec<u8>> = None;

        while let Ok(event) = event_rx.recv() {
            match event {
                BackendEvent::Connecting => on_disconnect_status(true),
                BackendEvent::Connected(error_mode) => {
                    if !error_mode && writer.is_none() {
                        let (byte_tx, byte_rx) = unbounded();
                        on_connect(byte_tx);
                        writer = Some(thread::spawn(move || {
                            while let Ok(byte) = byte_rx.recv() {
                                if let Err(error) = write_byte(byte) {
                                    send_event(BackendEvent::Error(error));
                                    send_event(BackendEvent::Disconnected);
                                    break;
                                }
                            }
                        }));
                    }
                }
                BackendEvent::Disconnected => {
                    on_disconnect_status(false);
                    if let Some(writer_thread) = writer.take() {
                        let _ = writer_thread.join();
                    }
                    error_message = None;
                }
                BackendEvent::Read(byte) => {
                    if let Some(message) = &mut error_message {
                        if byte.is_ascii() {
                            message.push(byte);
                        } else if byte == monsters_codes::RESET {
                            on_error(String::from_utf8_lossy(message).into_owned());
                            error_message = None;
                            if let Err(error) = write_byte(monsters_codes::RESET) {
                                on_error(error);
                            }
                        }
                    } else if byte == monsters_codes::ERROR {
                        error_message = Some(Vec::new());
                    } else {
                        on_read(byte);
                    }
                }
                BackendEvent::Error(error) => on_error(error),
            }
        }
    });
}

fn with_env<R>(raw_env: *mut RawJNIEnv, f: impl FnOnce(&mut JNIEnv<'_>) -> R) -> Option<R> {
    let mut env = match unsafe { JNIEnv::from_raw(raw_env) } {
        Ok(env) => env,
        Err(error) => {
            eprintln!("Could not access Android JNI environment: {error}");
            return None;
        }
    };
    Some(f(&mut env))
}

#[no_mangle]
pub extern "system" fn Java_com_jjthejjpro_monsters_1app_AndroidSerialPort_nativeInitialize(
    raw_env: *mut RawJNIEnv,
    instance: jobject,
) {
    with_env(raw_env, |env| {
        let vm = match env.get_java_vm() {
            Ok(vm) => vm,
            Err(error) => {
                eprintln!("Could not get Android Java VM: {error}");
                return;
            }
        };
        let instance = match env.new_global_ref(unsafe { JObject::from_raw(instance) }) {
            Ok(instance) => instance,
            Err(error) => {
                eprintln!("Could not retain Android serial backend: {error}");
                return;
            }
        };

        let backend = BACKEND.get_or_init(|| Mutex::new(None));
        match backend.lock() {
            Ok(mut backend) => {
                *backend = Some(Backend {
                    vm: Arc::new(vm),
                    instance,
                })
            }
            Err(error) => eprintln!("Android serial backend state is poisoned: {error}"),
        }
    });
}

#[no_mangle]
pub extern "system" fn Java_com_jjthejjpro_monsters_1app_AndroidSerialPort_nativeOnConnecting(
    _env: *mut RawJNIEnv,
    _instance: jobject,
) {
    send_event(BackendEvent::Connecting);
}

#[no_mangle]
pub extern "system" fn Java_com_jjthejjpro_monsters_1app_AndroidSerialPort_nativeOnConnected(
    _env: *mut RawJNIEnv,
    _instance: jobject,
    error_mode: jboolean,
) {
    send_event(BackendEvent::Connected(error_mode != 0));
}

#[no_mangle]
pub extern "system" fn Java_com_jjthejjpro_monsters_1app_AndroidSerialPort_nativeOnDisconnected(
    _env: *mut RawJNIEnv,
    _instance: jobject,
) {
    send_event(BackendEvent::Disconnected);
}

#[no_mangle]
pub extern "system" fn Java_com_jjthejjpro_monsters_1app_AndroidSerialPort_nativeOnRead(
    _env: *mut RawJNIEnv,
    _instance: jobject,
    byte: jbyte,
) {
    send_event(BackendEvent::Read(byte as u8));
}

#[no_mangle]
pub extern "system" fn Java_com_jjthejjpro_monsters_1app_AndroidSerialPort_nativeOnError(
    raw_env: *mut RawJNIEnv,
    _instance: jobject,
    message: jstring,
) {
    with_env(raw_env, |env| {
        let message = unsafe { JString::from_raw(message) };
        match env.get_string(&message) {
            Ok(message) => send_event(BackendEvent::Error(message.into())),
            Err(error) => eprintln!("Could not read Android serial error: {error}"),
        };
    });
}
