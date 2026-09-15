use embassy_stm32::gpio::Output;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

pub(crate) const B: u8 = 0b0001;
pub(crate) const G: u8 = 0b0010;
pub(crate) const R: u8 = 0b0100;
//pub(crate) const N: u8 = 0b1000;
pub(crate) const AND_MASK: u8 = 0b00010000;
pub(crate) const OR_MASK: u8 = 0b00100000;
pub(crate) const XOR_MASK: u8 = 0b00110000;

pub(crate) static LED_CHANNEL: Channel<ThreadModeRawMutex, u8, 4> = Channel::new();

#[embassy_executor::task]
pub(crate) async fn led(
    mut red: Output<'static>,
    mut green: Output<'static>,
    mut blue: Output<'static>,
) {
    let mut data = 0b0000;
    loop {
        let cmd = LED_CHANNEL.receive().await;

        match cmd & 0b11110000 {
            AND_MASK => {
                data &= cmd & 0b1111;
            }
            OR_MASK => {
                data |= cmd & 0b1111;
            }
            XOR_MASK => {
                data ^= cmd & 0b1111;
            }
            _ => continue,
        }

        if data & R != 0 {
            red.set_high();
        } else {
            red.set_low();
        }
        if data & G != 0 {
            green.set_high();
        } else {
            green.set_low();
        }
        if data & B != 0 {
            blue.set_high();
        } else {
            blue.set_low();
        }
    }
}
