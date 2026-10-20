use std::collections::HashSet;
use std::time::Duration;

pub(crate) fn monitor_usb() {
    // Preferred: block on kernel uevents (no polling, instant wake-up).
    if wait_for_uevent() {
        return;
    }
    // Fallback: the socket was denied (e.g. SELinux) or failed; poll sysfs.
    wait_for_sysfs_change();
}

/// Blocks until the kernel announces a new USB device.
/// Returns false if the netlink socket is unavailable or breaks.
fn wait_for_uevent() -> bool {
    use std::mem;

    const NETLINK_KOBJECT_UEVENT: libc::c_int = 15;

    unsafe {
        let fd = libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
            NETLINK_KOBJECT_UEVENT,
        );
        if fd < 0 {
            return false;
        }

        let mut addr: libc::sockaddr_nl = mem::zeroed();
        addr.nl_family = libc::AF_NETLINK as libc::sa_family_t;
        addr.nl_groups = 1; // kernel uevent multicast group

        let rc = libc::bind(
            fd,
            &addr as *const _ as *const libc::sockaddr,
            mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t,
        );
        if rc < 0 {
            libc::close(fd);
            return false;
        }

        let mut buf = [0u8; 8192];
        loop {
            let mut from: libc::sockaddr_nl = mem::zeroed();
            let mut from_len = mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t;

            let n = libc::recvfrom(
                fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
                0,
                &mut from as *mut _ as *mut libc::sockaddr,
                &mut from_len,
            );

            if n < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                libc::close(fd);
                return false;
            }

            // Only trust messages that come from the kernel (pid 0).
            if from.nl_pid != 0 {
                continue;
            }

            if is_usb_device_added(&buf[..n as usize]) {
                libc::close(fd);
                return true;
            }
        }
    }
}

/// A uevent is a series of NUL-separated "KEY=VALUE" strings.
fn is_usb_device_added(msg: &[u8]) -> bool {
    let (mut add, mut usb, mut device) = (false, false, false);
    for field in msg.split(|b| *b == 0) {
        match field {
            b"ACTION=add" => add = true,
            b"SUBSYSTEM=usb" => usb = true,
            b"DEVTYPE=usb_device" => device = true, // ignore per-interface events
            _ => {}
        }
    }
    add && usb && device
}

fn list_usb_devices() -> HashSet<String> {
    std::fs::read_dir("/sys/bus/usb/devices")
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn wait_for_sysfs_change() {
    let known = list_usb_devices();
    loop {
        std::thread::sleep(Duration::from_millis(500));
        if list_usb_devices().iter().any(|d| !known.contains(d)) {
            return;
        }
    }
}