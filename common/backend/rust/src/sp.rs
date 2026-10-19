/// Monitors for the USB serial port connection with the monster box.  This blocks until the monster box is found.
///
/// Returns the name for the serial port (e.g. COM4 or /dev/ttyACM0).
/// 
/// The second tuple value indicates if the controller encountered an error.
pub fn monitor_usb() -> (String, bool) {
    loop {
        match serialport::available_ports() {
            Ok(s_ports) => {
                for s_port in s_ports {
                    match s_port.port_type {
                        serialport::SerialPortType::UsbPort(usbport) => {
                            if usbport.vid == 0x4A4A && usbport.pid == 0x4D4E {
                                return (s_port.port_name, false);
                            } else if usbport.vid == 0x4A4A && usbport.pid == 0x4D4D {
                                return (s_port.port_name, true);
                            }
                        }
                        _ => continue,
                    }
                }
            }
            Err(e) => {
                eprintln!("{:?}: {}", e.kind, e.description);
                panic!("{e}");
            }
        }
        crate::native::os::monitor_usb();
    }
}
