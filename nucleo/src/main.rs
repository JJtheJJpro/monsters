#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::{
    Peri, bind_interrupts,
    exti::ExtiInput,
    gpio::{AnyPin, Level, Output, Pull, Speed},
    usart::{Config, Uart},
};
use embassy_time::Timer;
use embedded_io_async::Write;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    EXTI15_10 => embassy_stm32::exti::InterruptHandler<embassy_stm32::interrupt::typelevel::EXTI15_10>;
    USART3 => embassy_stm32::usart::InterruptHandler<embassy_stm32::peripherals::USART3>;
    DMA1_STREAM3 => embassy_stm32::dma::InterruptHandler<embassy_stm32::peripherals::DMA1_CH3>;
    DMA1_STREAM1 => embassy_stm32::dma::InterruptHandler<embassy_stm32::peripherals::DMA1_CH1>;
});

#[embassy_executor::task]
async fn blink(
    mut btn: ExtiInput<'static, embassy_stm32::mode::Async>,
    mut red: Output<'static>,
    mut green: Output<'static>,
    mut blue: Output<'static>,
) {
    loop {
        btn.wait_for_high().await;
        red.set_high();
        green.set_high();
        blue.set_high();
        btn.wait_for_low().await;
        red.set_low();
        green.set_low();
        blue.set_low();
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());

    let green_led = {
        let bp: Peri<'_, AnyPin> = p.PB0.into();
        Output::new(bp, Level::Low, Speed::VeryHigh)
    };
    let blue_led = {
        let bp: Peri<'_, AnyPin> = p.PB7.into();
        Output::new(bp, Level::Low, Speed::VeryHigh)
    };
    let red_led = {
        let bp: Peri<'_, AnyPin> = p.PB14.into();
        Output::new(bp, Level::Low, Speed::VeryHigh)
    };

    let mut config = Config::default();
    config.baudrate = 115200;

    let mut uart = Uart::new(p.USART3, p.PD9, p.PD8, p.DMA1_CH3, p.DMA1_CH1, Irqs, config).unwrap();

    Timer::after_secs(2).await;
    uart.write_all(b"Hello from Embassy Serial!\r\n")
        .await
        .unwrap();

    let button: ExtiInput<'_, embassy_stm32::mode::Async> =
        ExtiInput::new(p.PC13, p.EXTI13, Pull::Down, Irqs);
    spawner.spawn(blink(button, red_led, green_led, blue_led).unwrap());

    loop {
        let b = {
            let mut buf = [0u8; 1];
            match uart.read(&mut buf).await {
                Ok(_) => {}
                Err(e) => error!("UART Error: {:?}", e),
            }
            buf[0]
        };

        
    }
}
