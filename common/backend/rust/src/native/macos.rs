pub(crate) fn monitor_usb() {
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
