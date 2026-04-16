use serial2::SerialPort;
use std::{
    net::TcpStream,
    sync::{
        Arc, RwLock,
        mpsc::{self, Sender},
    },
    thread,
    time::Duration,
};
use tauri::{Emitter, Manager, State, WindowEvent};

mod native;

struct AppData {
    sp: Option<SerialPort>,
    tx: Option<Sender<Vec<u8>>>,
    ts: Option<TcpStream>,
}

pub fn monitor_usb() -> String {
    loop {
        let list = serial_enumerator::get_serial_list();
        for port in list {
            if let Some(pinfo) = port.usb_info {
                if pinfo.vid == "0483" && pinfo.pid == "374b" {
                    return port.name;
                }
            }
        }

        #[cfg(target_os = "windows")]
        native::monitor_usb_windows();
        #[cfg(target_os = "linux")]
        native::monitor_usb_linux();
        #[cfg(target_os = "macos")]
        native::monitor_usb_macos();
    }
}

#[tauri::command]
fn write(v: u8, state: State<Arc<RwLock<AppData>>>) {
    if let Some(sp) = &state.read().unwrap().sp {
        sp.write(&[v]).unwrap();
    }
}

#[tauri::command]
fn frontend_loaded(state: State<Arc<RwLock<AppData>>>) {
    let t_state = state.inner().clone();

    thread::spawn(move || {
        loop {
            let name = monitor_usb();
            let sp = match SerialPort::open(name, 9600) {
                Ok(mut v) => {
                    if let Err(e) = v.set_read_timeout(Duration::MAX) {
                        eprintln!("SetReadTimeout: {e}");
                        break;
                    }
                    if let Err(e) = v.set_write_timeout(Duration::MAX) {
                        eprintln!("SetWriteTimeout: {e}");
                        break;
                    }
                    v
                }
                Err(e) => {
                    eprintln!("{e}");
                    break;
                }
            };

            t_state.write().unwrap().sp = Some(sp.try_clone().unwrap());

            loop {
                let b = {
                    let mut b = [0];
                    if let Err(_) = sp.read_exact(&mut b) {
                        if let Some(tx) = &t_state.read().unwrap().tx {
                            tx.send(vec![0b10000000]).unwrap();
                        }
                        break;
                    }
                    b[0]
                };
                match b {
                    b if b < 0b10000000 => {
                        if let Some(tx) = &t_state.read().unwrap().tx {
                            tx.send(vec![b]).unwrap();
                        }
                    }
                    0b11111111 => {
                        if let Some(tx) = &t_state.read().unwrap().tx {
                            tx.send(vec![0b11111111]).unwrap();
                        }
                    }
                    _ => {}
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let (tx, rx) = mpsc::channel();

            let t_app = app.handle();
            let t_app2 = t_app.clone();

            t_app.manage(Arc::new(RwLock::new(AppData {
                sp: None,
                tx: Some(tx),
                ts: None,
            })));

            thread::spawn(move || {
                loop {
                    for msg in rx.iter() {
                        match msg[0] {
                            0b11111111 => {
                                t_app2.emit("global", [0xFF]).unwrap();
                            }
                            b if b < 0b10000000 => {
                                t_app2.emit("global", [b]).unwrap();
                            }
                            _ => {}
                        }
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, ev| match ev {
            WindowEvent::CloseRequested { .. } => {
                let arcclone = window.app_handle().state::<Arc<RwLock<AppData>>>().clone();
                if let Some(sp) = &arcclone.read().unwrap().sp {
                    sp.write(&[0b10000000]).unwrap();
                }
                drop(arcclone);
            }
            _ => {}
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![frontend_loaded])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
