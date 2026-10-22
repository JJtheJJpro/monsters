/// Monitors for the USB serial port connection with the monster box.  This blocks until the monster box is found.
///
/// Returns the name for the serial port (e.g. COM4 or /dev/ttyACM0).
///
/// The second tuple value indicates if the controller encountered an error.
pub fn monitor_usb() -> (String, bool) {
    loop {
        if let Some(port) = available_monster_port() {
            return port;
        }
        crate::native::os::monitor_usb();
    }
}

pub(super) fn available_monster_port() -> Option<(String, bool)> {
    #[cfg(not(any(unix, windows)))]
    compile_error!("Serialport support only valid for windows and unix targets.");

    match serialport::available_ports() {
        Ok(ports) => {
            for port in ports {
                match port.port_type {
                    serialport::SerialPortType::UsbPort(usbport) => {
                        if usbport.vid == 0x4A4A && usbport.pid == 0x4D4E {
                            return Some((port.port_name, false));
                        } else if usbport.vid == 0x4A4A && usbport.pid == 0x4D4D {
                            return Some((port.port_name, true));
                        }
                        //eprintln!(
                        //    "Ignoring USB serial port {} with VID:PID {:04x}:{:04x}",
                        //    port.port_name, usbport.vid, usbport.pid
                        //);
                    }
                    _ => {}
                    //port_type => eprintln!(
                    //    "Ignoring serial port {} with type {:?}",
                    //    port.port_name, port_type
                    //),
                }
            }
            None
        }
        Err(_) => {
            // SAFETY: compiler error covers non-unix and non-windows machines.
            unsafe { core::hint::unreachable_unchecked() }
            //eprintln!("{:?}: {}", e.kind, e.description);
            //panic!("{e}");
        }
    }
}
