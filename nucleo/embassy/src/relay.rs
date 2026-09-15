use crate::codes;
use embassy_stm32::gpio::{Level, OutputOpenDrain};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};

pub(crate) static RELAY_CHANNEL: Channel<ThreadModeRawMutex, u8, 8> = Channel::new();

#[embassy_executor::task]
pub(crate) async fn relay(mut relays: [OutputOpenDrain<'static>; 36]) {
    loop {
        let cmd = RELAY_CHANNEL.receive().await;

        if cmd == codes::RESET {
            for relay in &mut relays {
                relay.set_high();
            }
            continue;
        }

        let mode = if (cmd >> 6) != 0 {
            Level::Low
        } else {
            Level::High
        };
        let monst = cmd & 0b00111111;

        relays[monst as usize].set_level(mode);

        //LED_CHANNEL.send(0b0001).await;

        //for relay in relays {
        //    if cmd == 1 {
        //        LED_CHANNEL.send(0b0001).await;
        //        relay.set_high();
        //    } else {
        //        LED_CHANNEL.send(0b0001).await;
        //        relay.set_low();
        //    }
        //}
    }
}
