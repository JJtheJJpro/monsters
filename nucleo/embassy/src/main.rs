#![no_std]
#![no_main]

mod button;
mod codes;
mod led;
mod relay;
mod usb;
mod wifi;

extern crate alloc;

use crate::wifi::{IncomingData, WaitESP01, WifiCallbackFuture, WifiEvent};
use alloc::boxed::Box;
use core::panic::PanicInfo;
use defmt::{error, info, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    exti::ExtiInput,
    gpio::{Level, Output, OutputOpenDrain, Pull, Speed},
    time::Hertz,
    usart::{Config as UartConfig, Uart},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::Timer;
use embassy_usb::driver::EndpointError;
use embedded_alloc::LlffHeap as Heap;
use embedded_io::Write as _;

#[global_allocator]
static HEAP: Heap = Heap::empty();

bind_interrupts!(pub struct Irqs {
    EXTI15_10 => embassy_stm32::exti::InterruptHandler<embassy_stm32::interrupt::typelevel::EXTI15_10>;
    USART2 => embassy_stm32::usart::InterruptHandler<embassy_stm32::peripherals::USART2>;
    USART3 => embassy_stm32::usart::InterruptHandler<embassy_stm32::peripherals::USART3>;
    DMA1_STREAM1 => embassy_stm32::dma::InterruptHandler<embassy_stm32::peripherals::DMA1_CH1>;
    DMA1_STREAM3 => embassy_stm32::dma::InterruptHandler<embassy_stm32::peripherals::DMA1_CH3>;
    DMA1_STREAM5 => embassy_stm32::dma::InterruptHandler<embassy_stm32::peripherals::DMA1_CH5>;
    DMA1_STREAM6 => embassy_stm32::dma::InterruptHandler<embassy_stm32::peripherals::DMA1_CH6>;
    OTG_FS => embassy_stm32::usb::InterruptHandler<embassy_stm32::peripherals::USB_OTG_FS>;
});

#[panic_handler]
fn custom_panic(info: &PanicInfo<'_>) -> ! {
    cortex_m::interrupt::free(|_| {
        // 1. (Optional) Turn on a specific onboard LED to signal a crash.
        // Since you are in a panic state, you bypass Embassy structures
        // and write directly to the hardware registers if needed, or just skip it.

        // 2. Format or log the message if you have a raw emergency logger.
        // Warning: Do not use Embassy's async channels or mutexes here!
        // A panic can happen while a mutex is held, causing a deadlock.

        /* if let Some(location) = info.location() {
            // Log location.file(), location.line() via a low-level blocking UART/RTT
        }
        */

        // SAFETY: once panic happens, only one thread is running.
        let (mut r, mut g, mut b, mut usb) = unsafe {
            let mut config = UartConfig::default();
            config.baudrate = 115200;
            (
                Output::new(
                    embassy_stm32::peripherals::PB14::steal(),
                    Level::Low,
                    Speed::VeryHigh,
                ),
                Output::new(
                    embassy_stm32::peripherals::PB0::steal(),
                    Level::Low,
                    Speed::VeryHigh,
                ),
                Output::new(
                    embassy_stm32::peripherals::PB7::steal(),
                    Level::Low,
                    Speed::VeryHigh,
                ),
                Uart::new(
                    embassy_stm32::peripherals::USART3::steal(),
                    embassy_stm32::peripherals::PD9::steal(),
                    embassy_stm32::peripherals::PD8::steal(),
                    embassy_stm32::peripherals::DMA1_CH3::steal(),
                    embassy_stm32::peripherals::DMA1_CH1::steal(),
                    Irqs,
                    config,
                )
                .unwrap(),
            )
        };

        let mut i = 5;

        // 3. Keep the CPU from doing anything else or trigger a reset
        while i > 0 {
            // Choice A: Halt the CPU so you can attach a debugger and inspect memory
            //cortex_m::asm::bkpt();

            // Choice B: Or uncomment below to force a hard system reset after a panic
            //cortex_m::peripheral::SCB::sys_reset();

            write!(usb, "{}", info).unwrap();

            r.set_high();
            g.set_high();
            b.set_high();

            error!("{}", info);

            cortex_m::asm::delay(15_000_000);

            r.set_low();
            g.set_low();
            b.set_low();

            cortex_m::asm::delay(20_000_000);

            i -= 1;
        }
        cortex_m::peripheral::SCB::sys_reset();
    })
}

#[embassy_executor::task]
async fn connect_lan() {
    Timer::after_millis(500).await;
    if !wifi::test_startup().await {
        error!("wifi test bad");
        return;
    }
    info!("wifi test good");

    loop {
        const SSID: &str = env!("WIFI_SSID");
        const PWD: &str = env!("WIFI_PASS");
        if wifi::connect_to_ap_current(SSID, PWD, None).await {
            let asdf = wifi::local_ip_raw().await.unwrap();
            info!("{}", &asdf);
            led::LED_CHANNEL.send(led::XOR_MASK | led::G).await;
            break;
        } else {
            error!("error connecting to network");
            Timer::after_secs(5).await;
        }
    }

    loop {
        if let Some(v) = wifi::multiple_connections_mode().await {
            if v == 0 {
                if wifi::set_multiple_connections_mode(1).await {
                    info!("multiple connections set good");
                } else {
                    error!("multiple connections set failed");
                }
            } else {
                break;
            }
        }
    }

    info!("setting up server...");

    if !wifi::set_tcp_server_timeout(0).await {
        if let Some(v) = wifi::tcp_server_timeout().await {
            warn!(
                "failed to set tcp server timeout, using default value {}",
                v
            );
        } else {
            warn!("failed to set and get tcp server timeout");
        }
    } else {
        info!("tcp server timeout now indefinite");
    }

    if !wifi::configure_tcp_server(1, Some(8080)).await {
        error!("failed to setup tcp server on port 8080");
        return;
    } else {
        info!("server set up on port 8080");
    }

    //let mut auth_map = alloc::collections::BTreeMap::new();
    //loop {
    //    let inc_data = INCOMING_DATA_CHANNEL.receive().await;
    //    if !auth_map.contains_key(&inc_data.link_id) {
    //        auth_map.insert(inc_data.link_id, false);
    //    }
    //    if auth_map[&inc_data.link_id] == false {
    //        if inc_data.data == b"7dcfa8078e2c5e3d5d33573c115531f28dbc45afa4df8acd78751d5f9d730fe9"
    //        {
    //            assert_eq!(auth_map.insert(inc_data.link_id, true), Some(false));
    //            info!("id {} authorized", inc_data.link_id);
    //            wifi::request_send_multi(inc_data.link_id, 1, &[codes::AUTHORIZED]).await;
    //        } else {
    //            info!("id {} unauthorized", inc_data.link_id);
    //            wifi::request_send_multi(inc_data.link_id, 1, &[codes::UNAUTHORIZED]).await;
    //        }
    //        continue;
    //    }
    //    info!("0b{:08b}", inc_data.data[0]);
    //    wifi::request_send_multi(inc_data.link_id, 1, &[0]).await;
    //}
}

static SEND_DATA: Mutex<ThreadModeRawMutex, bool> = Mutex::new(false);
static AUTH_MAP: Mutex<ThreadModeRawMutex, alloc::collections::BTreeMap<u8, bool>> =
    Mutex::new(alloc::collections::BTreeMap::new());

fn on_wifi_event(event: WifiEvent) -> WifiCallbackFuture {
    Box::pin(async move {
        match event {
            WifiEvent::Connected(id) => {
                info!("id {} connected", id);
                AUTH_MAP.lock().await.insert(id, false);
            }
            WifiEvent::Closed(id) => {
                info!("id {} disconnected", id);
                AUTH_MAP.lock().await.remove(&id).unwrap();
            }
            _ => {}
        }
    })
}

fn on_wifi_data_event(data: IncomingData) -> WifiCallbackFuture {
    Box::pin(async move {
        let id = data.link_id;
        let data = data.data;

        if !AUTH_MAP.lock().await.contains_key(&id) {
            AUTH_MAP.lock().await.insert(id, false);
        }
        if AUTH_MAP.lock().await[&id] == false {
            if data == b"7dcfa8078e2c5e3d5d33573c115531f28dbc45afa4df8acd78751d5f9d730fe9" {
                assert_eq!(AUTH_MAP.lock().await.insert(id, true), Some(false));
                info!("id {} authorized", id);
                wifi::request_send_multi(id, 1, &[codes::AUTHORIZED]).await;
            } else {
                info!("id {} unauthorized", id);
                wifi::request_send_multi(id, 1, &[codes::UNAUTHORIZED]).await;
            }
        } else {
            process_byte(data[0], Some(id), None).await;
            //info!("0b{:08b}", data[0]);
            //wifi::request_send_multi(id, 1, &[0]).await;
        }
    })
}

async fn process_byte(
    b: u8,
    net_link_id: Option<u8>,
    usb_dev: Option<
        &mut embassy_usb::class::cdc_acm::CdcAcmClass<
            'static,
            embassy_stm32::usb::Driver<'static, embassy_stm32::peripherals::USB_OTG_FS>,
        >,
    >,
) {
    match b {
        b if b & 0b10000000 == 0 => {
            relay::RELAY_CHANNEL.send(b).await;
            if *SEND_DATA.lock().await {
                if let Some(id) = net_link_id {
                    wifi::request_send_multi(id, 1, &[b]).await;
                } else if let Some(usb) = usb_dev {
                    usb.write_packet(&[b]).await.unwrap();
                }
            }
        }
        codes::USB_TAKEOVER => {}
        codes::NET_TAKEOVER => {}
        codes::SEND_DATA_OFF => {
            *SEND_DATA.lock().await = false;
        }
        codes::SEND_DATA_ON => {
            *SEND_DATA.lock().await = true;
        }
        codes::RESET => {
            relay::RELAY_CHANNEL.send(codes::RESET).await;
        }
        _ => {}
    }
}

async fn test() {
    for i in 0..36 {
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize "heap" and peripherals.
    // SAFETY: first code lines to run make it safe.
    unsafe {
        embedded_alloc::init!(HEAP, 64 * 1024);
    }
    // Configure RCC for 216MHz core clock and 48MHz USB clock from 8MHz HSE bypass (ST-LINK MCO).
    let mut rcc_config = embassy_stm32::Config::default();
    {
        use embassy_stm32::rcc::*;
        rcc_config.rcc.hse = Some(Hse {
            freq: Hertz(8_000_000),
            mode: HseMode::Bypass,
        });
        rcc_config.rcc.pll_src = PllSource::HSE;
        rcc_config.rcc.pll = Some(Pll {
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL216,
            divp: Some(PllPDiv::DIV2), // 8MHz / 4 * 216 / 2 = 216MHz Sysclk
            divq: Some(PllQDiv::DIV9), // 8MHz / 4 * 216 / 9 = 48MHz USB clock
            divr: None,
        });
        rcc_config.rcc.ahb_pre = AHBPrescaler::DIV1;
        rcc_config.rcc.apb1_pre = APBPrescaler::DIV4;
        rcc_config.rcc.apb2_pre = APBPrescaler::DIV2;
        rcc_config.rcc.sys = Sysclk::PLL1_P;
        rcc_config.rcc.mux.clk48sel = mux::Clk48sel::PLL1_Q;
    }
    let p = embassy_stm32::init(rcc_config);

    // Initialize specific peripherals
    let red_led = Output::new(p.PB14, Level::Low, Speed::VeryHigh);
    let green_led = Output::new(p.PB0, Level::Low, Speed::VeryHigh);
    let blue_led = Output::new(p.PB7, Level::Low, Speed::VeryHigh);
    let user_button = ExtiInput::new(p.PC13, p.EXTI13, Pull::Down, Irqs);

    // Initialize USB OTG FS (CN13) on PA11 (DM) and PA12 (DP) as CDC-ACM serial
    let (usb_device, mut usb) = usb::init(p.USB_OTG_FS, p.PA12, p.PA11);

    // Initialize WiFi module
    let mut wifi_uart_config = UartConfig::default();
    wifi_uart_config.baudrate = 115200;
    let (wifi_tx, wifi_rx) = Uart::new(
        p.USART2,
        p.PD6,
        p.PD5,
        p.DMA1_CH6,
        p.DMA1_CH5,
        Irqs,
        wifi_uart_config,
    )
    .unwrap()
    .wait_wifi()
    .await
    .split(); // WiFi serial

    // Relay map
    let relays = [
        // P - like reading a book - last 6 rows of CN7
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
        // A - On left - [1-2]:CN7 - [4-8, 10, 12-13, 15, 17]:CN10
        OutputOpenDrain::new(p.PC6, Level::High, Speed::Low), // A01
        OutputOpenDrain::new(p.PB15, Level::High, Speed::Low), // A02
        OutputOpenDrain::new(p.PB1, Level::High, Speed::Low), // A03
        OutputOpenDrain::new(p.PC2, Level::High, Speed::Low), // A04
        OutputOpenDrain::new(p.PF4, Level::High, Speed::Low), // A05
        OutputOpenDrain::new(p.PB6, Level::High, Speed::Low), // A06
        OutputOpenDrain::new(p.PB2, Level::High, Speed::Low), // A07
        OutputOpenDrain::new(p.PD13, Level::High, Speed::Low), // A08
        OutputOpenDrain::new(p.PD11, Level::High, Speed::Low), // A09
        OutputOpenDrain::new(p.PE2, Level::High, Speed::Low), // A10
        OutputOpenDrain::new(p.PA0, Level::High, Speed::Low), // A11
        OutputOpenDrain::new(p.PE0, Level::High, Speed::Low), // A12
        // X - On right - [1-2]:CN7 - [4-8, 10, 12-13, 15, 17]:CN10
        OutputOpenDrain::new(p.PB8, Level::High, Speed::Low), // X01
        OutputOpenDrain::new(p.PB9, Level::High, Speed::Low), // X02
        OutputOpenDrain::new(p.PF14, Level::High, Speed::Low), // X03
        OutputOpenDrain::new(p.PE13, Level::High, Speed::Low), // X04
        OutputOpenDrain::new(p.PF15, Level::High, Speed::Low), // X05
        OutputOpenDrain::new(p.PG14, Level::High, Speed::Low), // X06
        OutputOpenDrain::new(p.PG9, Level::High, Speed::Low), // X07
        OutputOpenDrain::new(p.PE7, Level::High, Speed::Low), // X08
        OutputOpenDrain::new(p.PE10, Level::High, Speed::Low), // X09
        OutputOpenDrain::new(p.PE12, Level::High, Speed::Low), // X10
        OutputOpenDrain::new(p.PE15, Level::High, Speed::Low), // X11
        OutputOpenDrain::new(p.PB11, Level::High, Speed::Low), // X12
    ];

    // Spawn tasks
    spawner.spawn(led::led(red_led, green_led, blue_led).unwrap()); // LEDs
    spawner.spawn(relay::relay(relays).unwrap()); // Relays
    spawner.spawn(button::button(user_button).unwrap()); // Button
    spawner.spawn(wifi::wifi_writer(wifi_tx).unwrap()); // Wifi tx
    spawner.spawn(
        wifi::wifi_callback_runner(wifi::WifiCallbacks {
            on_event: Some(on_wifi_event),
            on_data: Some(on_wifi_data_event),
        })
        .unwrap(),
    ); // Wifi callbacks
    spawner.spawn(
        wifi::wifi_reader(
            wifi_rx,
            wifi::WifiCallbacks {
                on_event: Some(on_wifi_event),
                on_data: Some(on_wifi_data_event),
            },
        )
        .unwrap(),
    ); // Wifi rx
    spawner.spawn(connect_lan().unwrap()); // Handling connections
    spawner.spawn(usb::usb_task(usb_device).unwrap()); // USB Device runner

    loop {
        info!("Waiting for USB connection");
        usb.wait_connection().await;
        info!("USB Serial connected");

        loop {
            let mut buf = [0u8; 64];
            let n = match usb.read_packet(&mut buf).await {
                Ok(n) => n,
                Err(EndpointError::BufferOverflow) => {
                    error!("USB buffer overflow");
                    continue;
                }
                Err(EndpointError::Disabled) => {
                    info!("USB Serial disconnected");
                    break;
                }
            };

            for &b in &buf[..n] {
                info!("0b{:08b}", b);
                process_byte(b, None, Some(&mut usb)).await;
                //if b & 0b10000000 == 0 {
                //    relay::RELAY_CHANNEL.send(b).await;
                //    if send_data {
                //        let _ = usb.write_packet(&[b]).await;
                //    }
                //} else {
                //    // 0000000 - USB Takeover
                //    // 0000001 - Net Takeover
                //    // 0000010 - Send Data (Off)
                //    // 0000011 - Send Data (On)
                //    // 0000100 - Testing End
                //    // 0000101 - Testing Begin
                //    // 1111111 - Reset
                //    match b {
                //        SEND_DATA_OFF => send_data = false,
                //        SEND_DATA_ON => send_data = true,
                //        TESTING_BEGIN => {
                //            let _ = usb.write_packet(&[TESTING_BEGIN]).await;
                //            test().await;
                //        }
                //        RESET => relay::RELAY_CHANNEL.send(b).await,
                //        _ => {}
                //    }
                //}
            }
        }
    }
}
