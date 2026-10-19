pub(crate) fn monitor_usb() {
    use std::collections::HashMap;
    use wmi::{Variant, WMIConnection};

    let wmi_con = WMIConnection::new().unwrap();

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
