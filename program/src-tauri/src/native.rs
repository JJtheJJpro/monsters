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
    use udev::MonitorBuilder;

    let mut monitor = MonitorBuilder::new()
        .unwrap()
        .match_subsystem("usb")
        .unwrap()
        .listen()
        .unwrap();
    //println!("Listening for USB device changes...");

    for event in monitor.iter() {
        break;
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn monitor_usb_macos() {
    use core_foundation::runloop::CFRunLoopRun;
    use iokit_sys::{
        io_iterator_t, kIOMasterPortDefault, kIOMatchedNotification, IOIteratorNext,
        IOObjectRelease, IOServiceAddMatchingNotification, IOServiceMatching,
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