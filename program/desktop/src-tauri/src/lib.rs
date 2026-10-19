use std::sync::{Arc, RwLock};

use crossbeam_channel::Sender;
use monster_lib::TcpControlCmd;
use monsters_codes as codes;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

#[derive(Debug)]
enum NetSender {
    Unauthorized(Sender<[u8; 32]>),
    Authorized(Sender<u8>),
}

#[derive(Debug)]
struct RawAppData {
    sp: Option<Sender<u8>>,
    net: Option<NetSender>,
    stop_net: Option<Sender<TcpControlCmd>>,
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

        let mut file = File::options()
            .append(true)
            .create(true)
            .open("./monst_err.log")
            .unwrap();
        let curtime = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        writeln!(file, "[{}] {}", curtime, s.replace("\r", "\\r").replace("\n", "\\n")).unwrap();
    };

    monster_lib::start_serialport_thread(on_connect, on_disconnect_status, on_read, on_error);
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
        .invoke_handler(tauri::generate_handler![frontend_loaded, cmd])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
