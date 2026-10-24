#[cfg(target_os = "android")]
mod android_port;

use std::{
    sync::{Arc, RwLock},
    thread::JoinHandle,
    time::Duration,
};

use crossbeam_channel::Sender;
use monster_lib::TcpControlCmd;
use monsters_codes as codes;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

#[derive(Debug)]
enum NetSender {
    Unauthorized(Sender<[u8; 64]>),
    Authorized(Sender<u8>),
}

#[derive(Debug)]
struct RawAppData {
    sp: Option<Sender<u8>>,
    net: Option<NetSender>,
    stop_net: Option<Sender<TcpControlCmd>>,
    net_thread: Option<JoinHandle<()>>,
}
impl RawAppData {
    fn send_byte(&self, b: u8) -> bool {
        if let Some(net) = &self.net {
            if let NetSender::Authorized(net) = net {
                net.send(b).unwrap();
                true
            } else {
                false
            }
        } else if let Some(sp) = &self.sp {
            sp.send(b).unwrap();
            true
        } else {
            false
        }
    }
}
fn send_byte(state: &AppData, b: u8) -> bool {
    state.write().unwrap().send_byte(b)
}
type AppData = Arc<RwLock<RawAppData>>;

#[tauri::command]
fn frontend_loaded(app: AppHandle) {
    let state = app.state::<AppData>().inner();
    let app_clone = app.clone();
    let app_read_clone = app.clone();
    let app_on_error = app.clone();

    let state_on_connect = state.clone();
    let state_on_disconnect = state.clone();
    //let state_on_read = state.clone();

    let on_connect = move |tx| {
        state_on_connect.write().unwrap().sp = Some(tx);
        println!("send CONNECTED");
        app_clone.emit("cmd", codes::CONNECTED).unwrap();
    };
    let on_disconnect_status = move |connecting| {
        state_on_disconnect.write().unwrap().sp = None;
        if connecting {
            println!("send CONNECTING");
            app.emit("cmd", codes::CONNECTING).unwrap();
        } else {
            println!("send DISCONNECTED");
            app.emit("cmd", codes::DISCONNECTED).unwrap();
        }
    };
    let on_read = move |b| {
        app_read_clone.emit("cmd", b).unwrap();
    };
    let on_error = move |s: String| {
        use std::fs::File;
        use std::io::Write;

        let log_dir = match app_on_error.path().app_log_dir() {
            Ok(v) if let Ok(_) = std::fs::create_dir_all(&v) => v,
            _ => return,
        };
        let log_file = log_dir.join("monst_err.log");

        if let Ok(mut file) = File::options()
            .append(true)
            .create(true)
            .open(log_file)
        {
            let curtime = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let _ = writeln!(
                file,
                "[{}] {}",
                curtime,
                s.replace("\r", "\\r").replace("\n", "\\n")
            );
        }
    };

    #[cfg(not(target_os = "android"))]
    monster_lib::start_serialport_thread(on_connect, on_disconnect_status, on_read, on_error);
    #[cfg(target_os = "android")]
    android_port::start_serialport_thread(on_connect, on_disconnect_status, on_read, on_error);
}

#[tauri::command]
async fn net_on(app: AppHandle) -> bool {
    let state = app.state::<AppData>().inner();
    let (stop_tx, stop_rx) = crossbeam_channel::unbounded();
    state.write().unwrap().stop_net = Some(stop_tx);

    let app_on_password_needed = app.clone();
    let app_on_connect = app.clone();
    let app_on_read = app.clone();
    let app_on_disconnect_status = app.clone();
    let state_on_password_needed = state.clone();
    let state_on_connect = state.clone();
    let state_on_disconnect = state.clone();

    let on_password_needed = move |tx| {
        if let Some(tx) = tx {
            state_on_password_needed.write().unwrap().net = Some(NetSender::Unauthorized(tx));
        }
        app_on_password_needed
            .emit("cmd", codes::UNAUTHORIZED)
            .unwrap();
    };
    let on_connect = move |tx| {
        state_on_connect.write().unwrap().net = Some(NetSender::Authorized(tx));
        println!("net send CONNECTED");
        app_on_connect.emit("cmd", codes::CONNECTED).unwrap();
    };
    let on_disconnect_status = move |connecting| {
        state_on_disconnect.write().unwrap().net = None;
        if connecting {
            println!("net send CONNECTING");
            app_on_disconnect_status
                .emit("cmd", codes::CONNECTING)
                .unwrap();
        } else {
            println!("net send DISCONNECTED");
            app_on_disconnect_status
                .emit("cmd", codes::DISCONNECTED)
                .unwrap();
        }
    };
    let on_read = move |b| {
        app_on_read.emit("cmd", b).unwrap();
    };

    state.write().unwrap().net_thread = Some(monster_lib::start_tcp_thread(
        "192.168.1.55:8080",
        Duration::from_secs(3),
        stop_rx,
        on_password_needed,
        on_connect,
        on_disconnect_status,
        on_read,
        |_| {},
    ));

    true
}

#[tauri::command]
async fn net_auth(pass: String, app: AppHandle) -> bool {
    let state = app.state::<AppData>().inner();

    if let Some(NetSender::Unauthorized(tx)) = &state.write().unwrap().net {
        let sha256pswd = {
            let mut hasher = Sha256::new();
            hasher.update(pass);
            hasher
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };

        if let Err(e) = tx.send(sha256pswd.as_bytes().try_into().unwrap()) {
            eprintln!("{e}");
            return false;
        }
    }

    true
}

#[tauri::command]
async fn net_off(app: AppHandle) -> bool {
    let state = app.state::<AppData>().inner();

    if let Some(net) = &mut state.write().unwrap().net {
        if let NetSender::Authorized(net) = net {
            net.send(codes::RESET).unwrap();
        }
    }
    state.write().unwrap().net = None;
    if let Some(stop_net) = &state.read().unwrap().stop_net {
        stop_net.send(TcpControlCmd::Stop).unwrap();
    }
    state.write().unwrap().stop_net = None;

    let handle_to_join = {
        let guard = &mut state.write().unwrap().net_thread;
        guard.take()
    };
    if let Some(net_thread) = handle_to_join {
        println!("joining thread");
        net_thread.join().unwrap();
        println!("thread joined");
    }

    app.emit("cmd", codes::RESET).unwrap();
    if state.read().unwrap().sp.is_some() {
        app.emit("cmd", codes::CONNECTED).unwrap();
    } else {
        app.emit("cmd", codes::DISCONNECTED).unwrap();
    }

    true
}

#[tauri::command]
async fn cmd(b: u8, app: AppHandle) -> bool {
    let state = app.state::<AppData>().inner();

    match b {
        b if b < 0b10000000 => send_byte(state, b),

        codes::SEND_DATA_OFF => send_byte(state, b),
        codes::SEND_DATA_ON => send_byte(state, b),
        codes::AUTHORIZED => send_byte(state, b),

        codes::RESET => send_byte(state, b),

        _ => true,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(RawAppData {
            sp: None,
            net: None,
            stop_net: None,
            net_thread: None,
        })))
        //.setup(|app| Ok(()))
        .on_window_event(|window, ev| match ev {
            WindowEvent::CloseRequested { .. } => {
                let arcclone = window.app_handle().state::<AppData>().clone();
                if let Some(sp) = &arcclone.read().unwrap().sp {
                    sp.send(codes::RESET).unwrap();
                }
                if let Some(net) = &mut arcclone.write().unwrap().net {
                    if let NetSender::Authorized(net) = net {
                        net.send(codes::RESET).unwrap();
                    }
                }
                drop(arcclone);
            }
            _ => {}
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            frontend_loaded,
            cmd,
            net_on,
            net_auth,
            net_off
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
