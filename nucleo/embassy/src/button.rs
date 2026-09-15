use crate::{led, relay};
use embassy_stm32::exti::ExtiInput;

#[embassy_executor::task]
pub(crate) async fn button(mut btn: ExtiInput<'static, embassy_stm32::mode::Async>) {
    loop {
        btn.wait_for_high().await;
        relay::RELAY_CHANNEL.send(0b01000000).await;
        led::LED_CHANNEL.send(led::XOR_MASK | led::B).await;
        btn.wait_for_low().await;
        relay::RELAY_CHANNEL.send(0b00000000).await;
        led::LED_CHANNEL.send(led::XOR_MASK | led::B).await;
    }
}
