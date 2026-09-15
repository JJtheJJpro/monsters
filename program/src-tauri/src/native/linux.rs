pub(crate) fn monitor_usb() {
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