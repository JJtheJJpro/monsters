cfg_select! {
    target_os = "linux" => {
        mod linux;
        use linux as os;
    }
    target_os = "macos" => {
        mod macos;
        use macos as os;
    }
    target_os = "windows" => {
        mod windows;
        use windows as os;
    }
}

pub(crate) fn monitor_usb() -> String {
    loop {
        match serialport::available_ports() {
            Ok(s_ports) => {
                for s_port in s_ports {
                    match s_port.port_type {
                        serialport::SerialPortType::UsbPort(usbport) => {
                            if usbport.vid == 0x4A4A && usbport.pid == 0x4D4E {
                                return s_port.port_name;
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
        os::monitor_usb();
    }
}
