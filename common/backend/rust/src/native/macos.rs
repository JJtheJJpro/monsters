pub(crate) fn monitor_usb() {
    use std::cell::Cell;

    use core_foundation::runloop::{
        kCFRunLoopDefaultMode, CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRemoveSource,
        CFRunLoopRun, CFRunLoopStop,
    };
    use io_kit_sys::{
        kIOMasterPortDefault, IOIteratorNext, IONotificationPortCreate, IONotificationPortDestroy,
        IONotificationPortGetRunLoopSource, IOObjectRelease, IOServiceAddMatchingNotification,
        IOServiceMatching,
    };
    use io_kit_sys::{keys::kIOMatchedNotification, types::io_iterator_t};

    unsafe extern "C" fn callback(refcon: *mut std::ffi::c_void, iterator: io_iterator_t) {
        let mut found_device = false;
        unsafe {
            loop {
                let service = IOIteratorNext(iterator);
                if service == 0 {
                    break;
                }
                println!("USB device event on macOS: service=0x{:x}", service);
                IOObjectRelease(service);
                found_device = true;
            }
        }

        if found_device && unsafe { &*refcon.cast::<Cell<bool>>() }.get() {
            unsafe {
                CFRunLoopStop(CFRunLoopGetCurrent());
            }
        }
    }

    unsafe {
        let notify_port = IONotificationPortCreate(kIOMasterPortDefault);
        if notify_port.is_null() {
            panic!("Failed to create macOS USB notification port");
        }

        let matching = IOServiceMatching(b"IOSerialBSDClient\0".as_ptr().cast());
        if matching.is_null() {
            IONotificationPortDestroy(notify_port);
            panic!("Failed to create macOS USB matching dictionary");
        }

        let waiting_for_insert = Cell::new(false);
        let mut notify: io_iterator_t = 0;
        let result = IOServiceAddMatchingNotification(
            notify_port,
            kIOMatchedNotification.cast_mut(),
            matching,
            callback,
            (&waiting_for_insert as *const Cell<bool>).cast_mut().cast(),
            &mut notify,
        );
        if result != 0 {
            IONotificationPortDestroy(notify_port);
            panic!("Failed to register macOS USB notification: {result:#x}");
        }

        let run_loop_source = IONotificationPortGetRunLoopSource(notify_port);
        if run_loop_source.is_null() {
            IOObjectRelease(notify);
            IONotificationPortDestroy(notify_port);
            panic!("Failed to get macOS USB notification run loop source");
        }

        callback(
            (&waiting_for_insert as *const Cell<bool>).cast_mut().cast(),
            notify,
        );
        waiting_for_insert.set(true);
        CFRunLoopAddSource(
            CFRunLoopGetCurrent(),
            run_loop_source,
            kCFRunLoopDefaultMode,
        );
        if crate::sp::available_monster_port().is_none() {
            CFRunLoopRun();
        }
        CFRunLoopRemoveSource(
            CFRunLoopGetCurrent(),
            run_loop_source,
            kCFRunLoopDefaultMode,
        );
        IOObjectRelease(notify);
        IONotificationPortDestroy(notify_port);
    }
}
