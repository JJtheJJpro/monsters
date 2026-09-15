use std::{io::{Write, stdout}, thread, time::Duration};

//fn exit(err: Box<dyn Error>) -> ! {
//    eprintln!("{err}");
//    process::exit(1);
//}

fn utf8_to_str_escape_lossless(input: &[u8]) -> String {
    input.iter()
        .flat_map(|&b| std::ascii::escape_default(b))
        .map(|c| c as char)
        .collect()
}

fn main() {
    //let mut port = serial2::SerialPort::open("/dev/ttyACM0", 115200).unwrap();
    let mut port = serial2::SerialPort::open("COM9", 115200).unwrap();
    port.set_read_timeout(Duration::MAX).unwrap();
    port.set_write_timeout(Duration::MAX).unwrap();

    port.write(&[0b01000000]).unwrap();
    thread::sleep(Duration::from_millis(500));
    port.write(&[0b00000000]).unwrap();

    loop {
        let mut b = [0];
        port.read_exact(&mut b).unwrap();
        print!("{}", str::from_utf8(&b).unwrap());
        stdout().flush().unwrap();
    }
}
