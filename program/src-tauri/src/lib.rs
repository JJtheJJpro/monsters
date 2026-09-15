use crate::native::monitor_usb;
use serde_repr::{Deserialize_repr, Serialize_repr};
use serial2::SerialPort;
use std::{
    io::Write,
    net::TcpStream,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use tauri::{Manager, State, WindowEvent};

pub mod codes;
pub mod native;

// We'll keep it just in case...
#[repr(u8)]
#[derive(Serialize_repr, Deserialize_repr, Debug)]
enum Cmd {
    P0,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P9,
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    A8,
    A9,
    X0,
    X1,
    X2,
    X3,
    X4,
    X5,
    X6,
    X7,
    X8,
    X9,

    G,
    D,

    Unknown,
}

#[derive(Debug)]
struct RawAppData {
    sp: Option<SerialPort>,
    net: Option<TcpStream>,
}
type AppData = Arc<RwLock<RawAppData>>;

#[tauri::command]
fn frontend_loaded(state: State<AppData>) {
    let t_state = state.inner().clone();
    thread::spawn(move || loop {
        let name = monitor_usb();
        println!("{name}");
        let sp = match SerialPort::open(name, 115200) {
            Ok(mut v) => {
                if let Err(e) = v.set_read_timeout(Duration::MAX) {
                    eprintln!("SetReadTimeout: {e}");
                    continue;
                }
                if let Err(e) = v.set_write_timeout(Duration::MAX) {
                    eprintln!("SetWriteTimeout: {e}");
                    continue;
                }
                v
            }
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };
        t_state.write().unwrap().sp = Some(sp.try_clone().unwrap());
        loop {
            let b = {
                let mut b = [0];
                if let Err(_) = sp.read_exact(&mut b) {
                    t_state.write().unwrap().sp = None;
                    break;
                }
                b[0]
            };
        }
    });
}

#[tauri::command]
fn key_action(down: bool, cmd: u8, state: State<AppData>) -> bool {
    let byte = cmd | if down { 0b01000000 } else { 0 };
    let mut app = state.inner().write().unwrap();

    if let Some(sp) = &app.sp {
        sp.write(&[byte]).unwrap();
        println!("true");
        true
    } else if let Some(net) = &mut app.net {
        net.write(&[byte]).unwrap();
        println!("true");
        true
    } else {
        println!("false");
        false
    }

    //let t_state = state.inner().clone();
    //thread::spawn(move || {
    //    if let Some(sp) = &t_state.read().unwrap().sp {
    //        sp.write(&[cmd | if down { 0b01000000 } else { 0 }])
    //            .unwrap();
    //    }
    //    if let Some(net) = &mut t_state.write().unwrap().net {
    //        net.write(&[cmd | if down { 0b01000000 } else { 0 }])
    //            .unwrap();
    //    }
    //});
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(RawAppData {
            sp: None,
            net: None,
        })))
        .setup(|app| Ok(()))
        .on_window_event(|window, ev| match ev {
            WindowEvent::CloseRequested { .. } => {
                let arcclone = window.app_handle().state::<AppData>().clone();
                if let Some(sp) = &arcclone.read().unwrap().sp {
                    sp.write(&[codes::RESET]).unwrap();
                }
                if let Some(net) = &mut arcclone.write().unwrap().net {
                    net.write(&[codes::RESET]).unwrap();
                }
                drop(arcclone);
            }
            _ => {}
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![key_action, frontend_loaded])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
