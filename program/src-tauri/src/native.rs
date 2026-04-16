#[cfg(target_os = "windows")]
pub(crate) fn monitor_usb_windows() {
    use std::collections::HashMap;
    use wmi::{COMLibrary, Variant, WMIConnection};

    let com = COMLibrary::new().unwrap();
    let wmi_con = WMIConnection::new(com).unwrap();

    //println!("Listening for USB device changes...");

    const QUERY: &'static str = r#"SELECT * FROM Win32_DeviceChangeEvent WHERE EventType=2"#;
    let iter = wmi_con
        .raw_notification::<HashMap<String, Variant>>(QUERY)
        .unwrap();
    for evt in iter {
        match evt {
            Ok(_event) => break, //println!("DeviceChangeEvent: {:?}", event),
            Err(e) => {
                // Fallback: attempt to parse as generic JSON
                let val = wmi_con
                    .raw_query(QUERY)
                    .unwrap()
                    .into_iter()
                    .collect::<HashMap<String, Variant>>()
                    .clone();
                println!("Raw Event JSON: {:?}", val);
                eprintln!("WMI error mapping: {}", e);
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn monitor_usb_linux() {
    use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
    use std::os::fd::AsFd;
    use udev::{EventType, MonitorBuilder};

    let monitor = MonitorBuilder::new()
        .expect("Failed to create udev monitor")
        .match_subsystem("usb")
        .expect("Failed to add filter")
        .listen()
        .expect("Failed to listen");

    //println!("Listening for USB device changes...");

    let poll_fd = PollFd::new(monitor.as_fd(), PollFlags::POLLIN);
    let mut fds = [poll_fd];

    loop {
        match poll(&mut fds, PollTimeout::MAX) {
            Ok(_) => {
                for event in monitor.iter() {
                    // Only process Bind events
                    if event.event_type() == EventType::Bind {
                        // You can also inspect properties like Vendor ID/Product ID here
                        if let Some(vid) = event.device().property_value("ID_VENDOR_ID")
                            && let Some(pid) = event.device().property_value("ID_MODEL_ID")
                        {
                            if let Some(vid) = vid.to_str()
                                && let Some(pid) = pid.to_str()
                            {
                                if vid == "0483" && pid == "374b" {
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => eprintln!("Poll error: {:?}", e),
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn monitor_usb_macos() {
    use core_foundation::runloop::CFRunLoopRun;
    use iokit_sys::{
        IOIteratorNext, IOObjectRelease, IOServiceAddMatchingNotification, IOServiceMatching,
        io_iterator_t, kIOMasterPortDefault, kIOMatchedNotification,
    };

    extern "C" fn callback(_refcon: *mut std::ffi::c_void, iterator: io_iterator_t) {
        unsafe {
            loop {
                let service = IOIteratorNext(iterator);
                if service == 0 {
                    break;
                }
                println!("USB device event on macOS: service=0x{:x}", service);
                IOObjectRelease(service);
            }
        }
    }

    unsafe {
        let matching = IOServiceMatching(b"IOUSBDevice\0".as_ptr() as *const _);
        let mut notify: io_iterator_t = 0;
        IOServiceAddMatchingNotification(
            kIOMasterPortDefault,
            kIOMatchedNotification,
            matching,
            callback,
            ptr::null_mut(),
            &mut notify,
        );
        // Prime notifications
        callback(ptr::null_mut(), notify);
        //println!("Listening for USB device changes...");
        CFRunLoopRun();
    }
}
