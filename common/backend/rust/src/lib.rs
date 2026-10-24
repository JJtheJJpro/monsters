mod native;
mod sp;

use crossbeam_channel::{Receiver, Sender, unbounded};
use monsters_codes as codes;
use serial2::SerialPort;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    io::{self, ErrorKind, Read, Write},
    net::{Shutdown, TcpStream, ToSocketAddrs},
    sync::Arc,
    thread,
    time::Duration,
};

/// Starts running the infinite loop, handling all things for the serial port and exposing certain functions.
///
/// The writer is given when the `on_connect` callback is called.
///
/// When `on_disconnect` is called, the previously given writer will no longer work; it must be dropped.
pub fn start_serialport_thread<
    FConnect: Fn(Sender<u8>) + Send + Sync + 'static,
    FDisconnectStatus: Fn(bool) + Send + Sync + 'static,
    FRead: Fn(u8) + Send + Sync + 'static,
    FError: Fn(String) + Send + Sync + 'static,
>(
    on_connect: FConnect,
    on_disconnect_status: FDisconnectStatus,
    on_read: FRead,
    on_error: FError,
) {
    let on_read = Arc::new(on_read);

    thread::spawn(move || {
        loop {
            on_disconnect_status(false);
            let (name, err) = sp::monitor_usb();
            on_disconnect_status(true);
            let (tx, rx) = unbounded();

            let sp = match SerialPort::open(name, 115200) {
                Ok(mut v) => {
                    // Set both timeouts to MAX (blocking indefinitely)
                    if let Err(e) = v.set_read_timeout(Duration::MAX) {
                        eprintln!("SetReadTimeout: {e}");
                        continue;
                    } else if let Err(e) = v.set_write_timeout(Duration::MAX) {
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

            // Clone/split the underlying file descriptor/handle for concurrent I/O
            let sp_reader = match sp.try_clone() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Clone error: {e}");
                    continue;
                }
            };
            let sp_writer = sp;

            if !err {
                on_connect(tx);
            } else {
                drop(tx);
            }

            // By design, reading will fail first, which should be on the main thread.
            // The callback function calls, then the app will be notified to drop the tx, promptly ending the writer thread.
            // It is possible to do the other way around, but that would require function cloning. Nope.
            let on_read_clone = Arc::clone(&on_read);
            let write_handle = thread::spawn(move || {
                while let Ok(byte) = rx.recv() {
                    if let Err(e) = sp_writer.write_all(&[byte]) {
                        eprintln!("Write error: {e}");
                        break;
                    }
                }
                drop(rx);
            });

            loop {
                let mut b = [0u8; 1];
                match sp_reader.read_exact(&mut b) {
                    Ok(()) => {
                        if b[0] == codes::ERROR {
                            let mut b = [0u8; 1];
                            let mut buf = Vec::new();
                            while let Ok(()) = sp_reader.read_exact(&mut b) {
                                if b[0].is_ascii() {
                                    buf.push(b[0]);
                                } else {
                                    if b[0] == codes::RESET {
                                        break;
                                    }
                                }
                            }
                            on_error(String::from_utf8(buf).unwrap());
                            let _ = sp_reader.write_all(&[codes::RESET]);
                        } else {
                            on_read_clone(b[0]);
                        }
                    }
                    Err(e) if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) => {
                        continue;
                    }
                    Err(e) => {
                        eprintln!("Read error: {e}");
                        break;
                    }
                }
            }

            on_disconnect_status(false);

            // Wait for reader thread to finish tear-down on disconnect
            if !err {
                let _ = write_handle.join();
            }
        }
    });
}

/// Control messages sent from the application to manage TCP lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpControlCmd {
    Stop,
}

/// Starts running the infinite loop, handling all things for the tcp stream and exposing certain functions.
///
/// If a password is needed, the `on_password_needed` callback is called.
/// It will be called consecutive times afterwards if the given password is incorrect each time.
/// Once the given password is correct, the sender given from the `on_password_needed` should be dropped; use the `on_connect` sender.
///
/// The writer is given when the `on_connect` callback is called.
///
/// When `on_disconnect_status` is called, the previously given writer will no longer work; it must be dropped.
pub fn start_tcp_thread<
    A: ToSocketAddrs + Send + Sync + 'static,
    FNeedsPswd: Fn(Option<Sender<[u8; 64]>>) + Send + Sync + 'static,
    FConnect: Fn(Sender<u8>) + Send + Sync + 'static,
    FDisconnectStatus: Fn(bool) + Send + Sync + 'static,
    FRead: Fn(u8) + Send + Sync + 'static,
    FError: Fn(String) + Send + Sync + 'static,
>(
    ip: A,
    timeout: Duration,
    stop_rx: Receiver<TcpControlCmd>, // Control channel to trigger disconnect
    on_password_needed: FNeedsPswd,
    on_connect: FConnect,
    on_disconnect_status: FDisconnectStatus,
    on_read: FRead,
    _on_error: FError, // this is a maybe.  the only concern here is that the wifi module takes a full like 5 seconds to connect...
) -> thread::JoinHandle<()> {
    let on_read = Arc::new(on_read);
    let ip = Arc::new(ip);

    thread::spawn(move || {
        'tcp_lifecycle: loop {
            on_disconnect_status(false);
            // 1. Check if stop signal was sent before trying to connect
            if let Ok(TcpControlCmd::Stop) = stop_rx.try_recv() {
                break 'tcp_lifecycle;
            }

            let (connect_tx, connect_rx) = crossbeam_channel::bounded(1);
            let (connect_stop_tx, connect_stop_rx) = crossbeam_channel::bounded(1);
            let connect_ip = Arc::clone(&ip);
            let connect_handle = thread::spawn(move || {
                let result = tcp_connect_nonblocking(connect_ip.as_ref(), &connect_stop_rx);
                let _ = connect_tx.send(result);
            });

            let connect_result = crossbeam_channel::select_biased! {
                recv(stop_rx) -> cmd => {
                    if matches!(cmd, Ok(TcpControlCmd::Stop)) || cmd.is_err() {
                        let _ = connect_stop_tx.try_send(());
                        drop(connect_handle);
                        break 'tcp_lifecycle;
                    }
                    continue 'tcp_lifecycle;
                }
                recv(connect_rx) -> result => result.unwrap_or_else(|_| {
                    Err(io::Error::new(
                        ErrorKind::Other,
                        "TCP connection worker exited without a result",
                    ))
                }),
            };

            let mut tcp = match connect_result {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("TcpStream Connect Error: {e}");
                    let _ = connect_handle.join();
                    // Sleep with interrupt check
                    if thread_sleep_or_stop(&stop_rx, timeout) {
                        break 'tcp_lifecycle;
                    }
                    continue;
                }
            };
            let _ = connect_handle.join();

            if let Err(e) = tcp.set_nodelay(true) {
                eprintln!("TcpStream Set NoDelay: {e}");
                if thread_sleep_or_stop(&stop_rx, timeout) {
                    break 'tcp_lifecycle;
                }
                continue;
            }

            on_disconnect_status(true);

            let _ = tcp.set_read_timeout(None);
            let _ = tcp.set_write_timeout(None);

            // --- Handshake Phase ---
            if let Err(e) = tcp.write_all(&[codes::AUTHORIZED]) {
                eprintln!("TcpStream Write Handshake: {e}");
                if thread_sleep_or_stop(&stop_rx, timeout) {
                    break 'tcp_lifecycle;
                }
                continue;
            }

            let mut auth_byte = [0u8; 1];
            if let Err(e) = tcp.read_exact(&mut auth_byte) {
                eprintln!("TcpStream Read Handshake: {e}");
                if thread_sleep_or_stop(&stop_rx, timeout) {
                    break 'tcp_lifecycle;
                }
                continue;
            }

            // --- Password Phase ---
            if auth_byte[0] == codes::UNAUTHORIZED {
                let (pswd_tx, pswd_rx) = unbounded::<[u8; 64]>();
                on_password_needed(Some(pswd_tx));
                while auth_byte[0] == codes::UNAUTHORIZED {
                    // Wait for password OR stop command
                    crossbeam_channel::select! {
                        recv(stop_rx) -> msg => {
                            if matches!(msg, Ok(TcpControlCmd::Stop)) {
                                on_password_needed(None);
                                break 'tcp_lifecycle;
                            }
                        }
                        recv(pswd_rx) -> pswd => {
                            match pswd {
                                Ok(pswd_bytes) => {
                                    if let Err(e) = tcp.write_all(&pswd_bytes) {
                                        eprintln!("TcpStream Write Password: {e}");
                                        on_password_needed(None);
                                        if thread_sleep_or_stop(&stop_rx, timeout) { break 'tcp_lifecycle; }
                                        continue;
                                    }
                                    if let Err(e) = tcp.read_exact(&mut auth_byte) {
                                        eprintln!("TcpStream Read Auth Response: {e}");
                                        on_password_needed(None);
                                        if thread_sleep_or_stop(&stop_rx, timeout) { break 'tcp_lifecycle; }
                                        continue;
                                    }
                                    if auth_byte[0] == codes::UNAUTHORIZED {
                                        eprintln!("Auth failed");
                                        on_password_needed(None);
                                        if thread_sleep_or_stop(&stop_rx, timeout) { break 'tcp_lifecycle; }
                                    } else if auth_byte[0] == codes::AUTHORIZED {
                                        break;
                                    }
                                }
                                Err(_) => {
                                    on_password_needed(None);
                                    if thread_sleep_or_stop(&stop_rx, timeout) { break 'tcp_lifecycle; }
                                    continue;
                                }
                            }
                        }
                    }
                }
            }

            // --- Connected Phase ---
            let mut reader_stream = match tcp.try_clone() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("TcpStream try_clone error: {e}");
                    if thread_sleep_or_stop(&stop_rx, timeout) {
                        break 'tcp_lifecycle;
                    }
                    continue;
                }
            };
            let writer_stream = Arc::new(tcp);
            let writer_stream_clone = Arc::clone(&writer_stream);

            let (tx, rx) = unbounded::<u8>();
            on_connect(tx);

            // Spawn dedicated reader thread
            let on_read_clone = Arc::clone(&on_read);
            let read_handle = thread::spawn(move || {
                loop {
                    let mut b = [0u8; 1];
                    match reader_stream.read_exact(&mut b) {
                        Ok(()) => on_read_clone(b[0]),
                        Err(e)
                            if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
                        {
                            continue;
                        }
                        Err(e) => {
                            eprintln!("TcpStream Read Error: {e}");
                            break;
                        }
                    }
                }
            });

            // Loop waiting for outbound data OR a Stop command
            loop {
                crossbeam_channel::select! {
                    // Outbound byte to write
                    recv(rx) -> byte => {
                        match byte {
                            Ok(b) => {
                                if let Err(e) = (&*writer_stream_clone).write_all(&[b]) {
                                    eprintln!("TcpStream Write Error: {e}");
                                    break;
                                }
                            }
                            Err(_) => break, // Outbound tx dropped
                        }
                    }
                    // Command from application to stop connection
                    recv(stop_rx) -> cmd => {
                        if matches!(cmd, Ok(TcpControlCmd::Stop)) {
                            // Shut down socket immediately to force read_exact in reader thread to fail
                            let _ = writer_stream_clone.shutdown(Shutdown::Both);
                            let _ = read_handle.join();
                            drop(rx);
                            on_disconnect_status(false);
                            break 'tcp_lifecycle; // Fully exit thread loop
                        }
                    }
                }
            }

            // Clean teardown if network read/write broke unexpectedly
            let _ = writer_stream_clone.shutdown(Shutdown::Both);
            let _ = read_handle.join();

            drop(rx);
            on_disconnect_status(false);

            if thread_sleep_or_stop(&stop_rx, timeout) {
                break 'tcp_lifecycle;
            }
        }
    })
}

fn tcp_connect_nonblocking<A: ToSocketAddrs>(
    address: &A,
    stop_rx: &Receiver<()>,
) -> io::Result<TcpStream> {
    let mut last_error = None;

    for address in address.to_socket_addrs()? {
        if !matches!(
            stop_rx.try_recv(),
            Err(crossbeam_channel::TryRecvError::Empty)
        ) {
            return Err(io::Error::new(
                ErrorKind::Interrupted,
                "connection cancelled",
            ));
        }

        let socket = match Socket::new(
            Domain::for_address(address),
            Type::STREAM,
            Some(Protocol::TCP),
        ) {
            Ok(socket) => socket,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };

        if let Err(error) = socket.set_nonblocking(true) {
            last_error = Some(error);
            continue;
        }

        match socket.connect(&address.into()) {
            Ok(()) => return socket_to_tcp_stream(socket),
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        }

        loop {
            crossbeam_channel::select! {
                recv(stop_rx) -> _ => {
                    return Err(io::Error::new(ErrorKind::Interrupted, "connection cancelled"));
                }
                default(Duration::from_millis(25)) => {}
            }

            if let Some(error) = socket.take_error()? {
                last_error = Some(error);
                break;
            }

            match socket.peer_addr() {
                Ok(_) => return socket_to_tcp_stream(socket),
                Err(error) if error.kind() == ErrorKind::NotConnected => {}
                Err(error) => {
                    last_error = Some(error);
                    break;
                }
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| io::Error::new(ErrorKind::InvalidInput, "no socket addresses resolved")))
}

fn socket_to_tcp_stream(socket: Socket) -> io::Result<TcpStream> {
    let stream = TcpStream::from(socket);
    stream.set_nonblocking(false)?;
    Ok(stream)
}

/// Helper function to sleep safely while listening for a Stop command
fn thread_sleep_or_stop(stop_rx: &Receiver<TcpControlCmd>, duration: Duration) -> bool {
    crossbeam_channel::select! {
        recv(stop_rx) -> msg => matches!(msg, Ok(TcpControlCmd::Stop)),
        default(duration) => false,
    }
}
