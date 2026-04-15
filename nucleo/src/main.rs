#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::{
    Peri, bind_interrupts,
    exti::ExtiInput,
    gpio::{AnyPin, Level, Output, Pull, Speed},
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    EXTI15_10 => embassy_stm32::exti::InterruptHandler<embassy_stm32::interrupt::typelevel::EXTI15_10>;
});

#[embassy_executor::task]
async fn blink(pin: Peri<'static, AnyPin>) {
    let mut led = Output::new(pin, Level::Low, Speed::VeryHigh);

    loop {
        // Timekeeping is globally available, no need to mess with hardware timers.
        led.set_high();
        Timer::after_millis(1000).await;
        led.set_low();
        Timer::after_millis(1000).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());

    spawner.spawn(blink(p.PB0.into()).unwrap());

    let mut button = ExtiInput::new(p.PC13, p.EXTI13, Pull::Down, Irqs);
    loop {
        // Asynchronously wait for GPIO events, allowing other tasks
        // to run, or the core to sleep.
        button.wait_for_high().await;
        info!("Button pressed!");
        button.wait_for_low().await;
        info!("Button released!");
    }
}
