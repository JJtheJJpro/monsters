use std::{io::{Write, stdout}, time::Duration};

//fn exit(err: Box<dyn Error>) -> ! {
//    eprintln!("{err}");
//    process::exit(1);
//}

fn main() {
    let mut port = serial2::SerialPort::open("COM6", 115200).unwrap();
    port.set_read_timeout(Duration::MAX).unwrap();
    port.set_write_timeout(Duration::MAX).unwrap();

    loop {
        let mut b = [0];
        port.read_exact(&mut b).unwrap();
        print!("{}", str::from_utf8(&b).unwrap());
        stdout().flush().unwrap();
    }
}
