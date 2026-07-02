#![no_std]
#![no_main]

mod codes;
pub mod wifi_driver;

use codes::*;
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    exti::ExtiInput,
    gpio::{Level, Output, OutputOpenDrain, Pull, Speed},
    usart::{Config, Uart},
};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use embedded_io_async::Write;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    EXTI15_10 => embassy_stm32::exti::InterruptHandler<embassy_stm32::interrupt::typelevel::EXTI15_10>;
    USART2 => embassy_stm32::usart::InterruptHandler<embassy_stm32::peripherals::USART2>;
    //USART2 => embassy_stm32::usart::BufferedInterruptHandler<embassy_stm32::peripherals::USART2>;
    USART3 => embassy_stm32::usart::InterruptHandler<embassy_stm32::peripherals::USART3>;
    DMA1_STREAM1 => embassy_stm32::dma::InterruptHandler<embassy_stm32::peripherals::DMA1_CH1>;
    DMA1_STREAM3 => embassy_stm32::dma::InterruptHandler<embassy_stm32::peripherals::DMA1_CH3>;
    DMA1_STREAM5 => embassy_stm32::dma::InterruptHandler<embassy_stm32::peripherals::DMA1_CH5>;
    DMA1_STREAM6 => embassy_stm32::dma::InterruptHandler<embassy_stm32::peripherals::DMA1_CH6>;
});

static LED_CHANNEL: Channel<ThreadModeRawMutex, u8, 4> = Channel::new();
const B: u8 = 0b0001;
const G: u8 = 0b0010;
const R: u8 = 0b0100;
//const N: u8 = 0b1000;
const AND_MASK: u8 = 0b00010000;
const OR_MASK: u8 = 0b00100000;
const XOR_MASK: u8 = 0b00110000;
#[embassy_executor::task]
async fn led_task(mut red: Output<'static>, mut green: Output<'static>, mut blue: Output<'static>) {
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

static RELAY_CHANNEL: Channel<ThreadModeRawMutex, u8, 8> = Channel::new();
#[embassy_executor::task]
async fn relay(mut relays: [OutputOpenDrain<'static>; 36]) {
    loop {
        let cmd = RELAY_CHANNEL.receive().await;

        if cmd == 0b11111111 {
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

#[embassy_executor::task]
async fn button_loop(mut btn: ExtiInput<'static, embassy_stm32::mode::Async>) {
    loop {
        btn.wait_for_high().await;
        LED_CHANNEL.send(XOR_MASK | B).await;
        btn.wait_for_low().await;
        LED_CHANNEL.send(XOR_MASK | B).await;
    }
}

async fn test() {
    for i in 0..36 {
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    spawner.spawn(
        led_task(
            Output::new(p.PB14, Level::Low, Speed::VeryHigh),
            Output::new(p.PB0, Level::Low, Speed::VeryHigh),
            Output::new(p.PB7, Level::Low, Speed::VeryHigh),
        )
        .unwrap(),
    );

    let btn: ExtiInput<'_, embassy_stm32::mode::Async> =
        ExtiInput::new(p.PC13, p.EXTI13, Pull::Down, Irqs);
    spawner.spawn(button_loop(btn).unwrap());

    let mut config = Config::default();
    config.baudrate = 115200;

    let mut usb = Uart::new(p.USART3, p.PD9, p.PD8, p.DMA1_CH3, p.DMA1_CH1, Irqs, config).unwrap();

    spawner.spawn(
        wifi_driver::wifi(
            Uart::new(p.USART2, p.PD6, p.PD5, p.DMA1_CH6, p.DMA1_CH5, Irqs, config).unwrap(),
        )
        .unwrap(),
    );

    if !wifi_driver::test_startup().await {
        LED_CHANNEL.send(XOR_MASK | R).await;
        usb.write_all(b"AT TEST FAIL").await.unwrap();
        return;
    }
    
    info!("{}", wifi_driver::current_wifi_mode().await);

    // relay map
    let relays = [
        // P - like reading a book
        // A - On left (A01 and A02 found above P)
        // X - On right (X01 and X02 found above P)
        OutputOpenDrain::new(p.PA15, Level::High, Speed::Low), // P01
        OutputOpenDrain::new(p.PA5, Level::High, Speed::Low),  // P02
        OutputOpenDrain::new(p.PC7, Level::High, Speed::Low),  // P03
        OutputOpenDrain::new(p.PA6, Level::High, Speed::Low),  // P04
        OutputOpenDrain::new(p.PB5, Level::High, Speed::Low),  // P05
        OutputOpenDrain::new(p.PA7, Level::High, Speed::Low),  // P06
        OutputOpenDrain::new(p.PB3, Level::High, Speed::Low),  // P07
        OutputOpenDrain::new(p.PD14, Level::High, Speed::Low), // P08
        OutputOpenDrain::new(p.PA4, Level::High, Speed::Low),  // P09
        OutputOpenDrain::new(p.PD15, Level::High, Speed::Low), // P10
        OutputOpenDrain::new(p.PB4, Level::High, Speed::Low),  // P11
        OutputOpenDrain::new(p.PF12, Level::High, Speed::Low), // P12
        OutputOpenDrain::new(p.PC6, Level::High, Speed::Low),  // A01
        OutputOpenDrain::new(p.PB15, Level::High, Speed::Low), // A02
        OutputOpenDrain::new(p.PB1, Level::High, Speed::Low),  // A03
        OutputOpenDrain::new(p.PC2, Level::High, Speed::Low),  // A04
        OutputOpenDrain::new(p.PF4, Level::High, Speed::Low),  // A05
        OutputOpenDrain::new(p.PB6, Level::High, Speed::Low),  // A06
        OutputOpenDrain::new(p.PB2, Level::High, Speed::Low),  // A07
        OutputOpenDrain::new(p.PD13, Level::High, Speed::Low), // A08
        OutputOpenDrain::new(p.PD11, Level::High, Speed::Low), // A09
        OutputOpenDrain::new(p.PE2, Level::High, Speed::Low),  // A10
        OutputOpenDrain::new(p.PA0, Level::High, Speed::Low),  // A11
        OutputOpenDrain::new(p.PE0, Level::High, Speed::Low),  // A12
        OutputOpenDrain::new(p.PB8, Level::High, Speed::Low),  // X01
        OutputOpenDrain::new(p.PB9, Level::High, Speed::Low),  // X02
        OutputOpenDrain::new(p.PF14, Level::High, Speed::Low), // X03
        OutputOpenDrain::new(p.PE13, Level::High, Speed::Low), // X04
        OutputOpenDrain::new(p.PF15, Level::High, Speed::Low), // X05
        OutputOpenDrain::new(p.PG14, Level::High, Speed::Low), // X06
        OutputOpenDrain::new(p.PG9, Level::High, Speed::Low),  // X07
        OutputOpenDrain::new(p.PE7, Level::High, Speed::Low),  // X08
        OutputOpenDrain::new(p.PE10, Level::High, Speed::Low), // X09
        OutputOpenDrain::new(p.PE12, Level::High, Speed::Low), // X10
        OutputOpenDrain::new(p.PE15, Level::High, Speed::Low), // X11
        OutputOpenDrain::new(p.PB11, Level::High, Speed::Low), // X12
    ];
    spawner.spawn(relay(relays).unwrap());

    let mut send_data = false;

    loop {
        let b = {
            let mut buf = [0u8; 1];
            if let Err(e) = usb.read(&mut buf).await {
                error!("UART Error: {:?}", e);
            }
            buf[0]
        };
        if b < 0b10000000 {
            RELAY_CHANNEL.send(b).await;
            if send_data {
                usb.write(&[b]).await.unwrap();
            }
        } else {
            // 0000000 - USB Takeover
            // 0000001 - Net Takeover
            // 0000010 - Send Data (Off)
            // 0000011 - Send Data (On)
            // 0000100 - Testing End
            // 0000101 - Testing Begin
            // 1111111 - Reset
            match b {
                SEND_DATA_OFF => send_data = false,
                SEND_DATA_ON => send_data = true,
                TESTING_BEGIN => {
                    usb.write(&[TESTING_BEGIN]).await.unwrap();
                    test().await;
                }
                RESET => RELAY_CHANNEL.send(b).await,
                _ => {}
            }
        }
    }
}
