use core::{
    fmt::{self, Write as _},
    future::Future,
    task::{Context, Poll, Waker},
};
use embassy_futures::{
    select::{Either, select},
    yield_now,
};
use embassy_stm32::Peri;
use embassy_stm32::{
    gpio::{Input, Pull},
    peripherals::{PA9, PA11, PA12, USB_OTG_FS},
    usb::{Config as UsbDriverConfig, Driver, InterruptHandler},
};
use embassy_time::{Duration, Timer, with_timeout};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, UsbDevice};
use monsters_codes as codes;
use static_cell::StaticCell;

pub type MyUsbDriver = Driver<'static, USB_OTG_FS>;

static EP_OUT_BUFFER: StaticCell<[u8; 256]> = StaticCell::new();
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static STATE: StaticCell<State> = StaticCell::new();

static PANIC_EP_OUT_BUFFER: StaticCell<[u8; 256]> = StaticCell::new();
static PANIC_CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static PANIC_BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static PANIC_CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static PANIC_STATE: StaticCell<State> = StaticCell::new();

const PANIC_REPORT_CAPACITY: usize = 1024;
const PANIC_REPORT_TRUNCATION: &[u8] = b"\r\n[panic info truncated]\r\n";

struct PanicReport {
    bytes: [u8; PANIC_REPORT_CAPACITY],
    len: usize,
    truncated: bool,
}

impl PanicReport {
    fn new() -> Self {
        Self {
            bytes: [0; PANIC_REPORT_CAPACITY],
            len: 0,
            truncated: false,
        }
    }

    fn finish(&mut self) -> &[u8] {
        if self.truncated {
            let start = PANIC_REPORT_CAPACITY - PANIC_REPORT_TRUNCATION.len();
            self.bytes[start..].copy_from_slice(PANIC_REPORT_TRUNCATION);
            self.len = PANIC_REPORT_CAPACITY;
        }
        &self.bytes[..self.len]
    }

    fn push_bytes(&mut self, value: &[u8]) {
        let available = PANIC_REPORT_CAPACITY - self.len;
        let copied = available.min(value.len());
        self.bytes[self.len..self.len + copied].copy_from_slice(&value[..copied]);
        self.len += copied;
        self.truncated |= copied < value.len();
    }
}

impl fmt::Write for PanicReport {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.push_bytes(value.as_bytes());
        Ok(())
    }
}

#[embassy_executor::task]
pub async fn usb_device_task(mut usb: embassy_usb::UsbDevice<'static, MyUsbDriver>) -> ! {
    usb.run().await
}

fn new_driver(
    peri: Peri<'static, USB_OTG_FS>,
    irqs: impl embassy_stm32::interrupt::typelevel::Binding<
        embassy_stm32::interrupt::typelevel::OTG_FS,
        InterruptHandler<USB_OTG_FS>,
    > + 'static,
    dp: Peri<'static, PA12>,
    dm: Peri<'static, PA11>,
    ep_out_buffer: &'static mut [u8; 256],
) -> MyUsbDriver {
    let mut config = UsbDriverConfig::default();
    config.vbus_detection = false;
    Driver::new_fs(peri, irqs, dp, dm, ep_out_buffer, config)
}

pub fn init(
    peri: Peri<'static, USB_OTG_FS>,
    dp: Peri<'static, PA12>,
    dm: Peri<'static, PA11>,
) -> (
    UsbDevice<'static, MyUsbDriver>,
    CdcAcmClass<'static, MyUsbDriver>,
) {
    let driver = new_driver(peri, crate::Irqs, dp, dm, EP_OUT_BUFFER.init([0u8; 256]));

    let mut usb_config = embassy_usb::Config::new(0x4A4A, 0x4D4E);
    usb_config.manufacturer = Some("JJ Inc");
    usb_config.product = Some("Monster Box");
    usb_config.serial_number = Some("12345678");

    let config_descriptor = CONFIG_DESCRIPTOR.init([0u8; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0u8; 256]);
    let control_buf = CONTROL_BUF.init([0u8; 64]);
    let state = STATE.init(State::new());

    let mut builder = Builder::new(
        driver,
        usb_config,
        config_descriptor,
        bos_descriptor,
        &mut [],
        control_buf,
    );

    let usb_class = CdcAcmClass::new(&mut builder, state, 64);
    let usb_device = builder.build();
    //let (tx, rx) = usb_class.split();

    (usb_device, usb_class)
}

pub fn panic_report(info: &core::panic::PanicInfo<'_>) -> ! {
    // PA9 senses USB VBUS on the STM32F767.
    //let vbus = unsafe { Input::new(PA9::steal(), Pull::None) };
    //if !vbus.is_high() {
    //    cortex_m::peripheral::SCB::sys_reset();
    //}

    let mut cp = unsafe { cortex_m::Peripherals::steal() };
    cp.DCB.enable_trace();
    cortex_m::peripheral::DWT::unlock();
    cp.DWT.enable_cycle_counter();

    const CYCLES_PER_MS: u32 = 216_000; // 216 MHz; adjust if your clock differs

    async fn delay_ms(ms: u32) {
        let start = cortex_m::peripheral::DWT::cycle_count();
        while cortex_m::peripheral::DWT::cycle_count().wrapping_sub(start) < ms * CYCLES_PER_MS {
            yield_now().await;
        }
    }

    let otg = embassy_stm32::pac::USB_OTG_FS;
    otg.dctl().modify(|w| w.set_sdis(true)); // soft disconnect: drop D+ pull-up
    cortex_m::asm::delay(216_000_000 / 10); // ~100 ms @ 216 MHz; host sees detach

    let mut report = PanicReport::new();
    report.push_bytes(&[codes::ERROR]); // raw byte, no formatting
    let _ = write!(&mut report, "{info}\r\n");
    report.push_bytes(&[codes::RESET]);
    let report = report.finish();

    // The normal USB task has stopped with the executor. Reinitialize OTG FS and
    // poll both the device and CDC writes directly until the report is sent.
    let driver = unsafe {
        new_driver(
            USB_OTG_FS::steal(),
            crate::Irqs,
            PA12::steal(),
            PA11::steal(),
            PANIC_EP_OUT_BUFFER.init([0; 256]),
        )
    };

    let mut usb_config = embassy_usb::Config::new(0x4A4A, 0x4D4D);
    usb_config.manufacturer = Some("JJ Inc");
    usb_config.product = Some("Monster Box");
    usb_config.serial_number = Some("12345678");

    let config_descriptor = PANIC_CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = PANIC_BOS_DESCRIPTOR.init([0; 256]);
    let control_buf = PANIC_CONTROL_BUF.init([0; 64]);
    let state = PANIC_STATE.init(State::new());
    let mut builder = Builder::new(
        driver,
        usb_config,
        config_descriptor,
        bos_descriptor,
        &mut [],
        control_buf,
    );
    let mut usb_class = CdcAcmClass::new(&mut builder, state, 64);
    let mut usb_device = builder.build();

    async fn send_report(class: &mut CdcAcmClass<'static, MyUsbDriver>, report: &[u8]) -> bool {
        for packet in report.chunks(64) {
            if class.write_packet(packet).await.is_err() {
                return false;
            }
        }
        // The ZLP's completion confirms the host actually read the data.
        class.write_packet(&[]).await.is_ok()
    }

    let future = embassy_futures::join::join(usb_device.run(), async {
        // Allow ~30 s total; each delay_ms stays well under the u32 cycle-counter wrap (~19.9 s).
        let result = select(
            async {
                usb_class.wait_connection().await;
                defmt::info!("panic usb: configured");

                let mut ack = [0u8; 1];
                loop {
                    if !send_report(&mut usb_class, report).await {
                        return;
                    }

                    // Listen for the host's ack for up to 300 ms, then resend.
                    match select(usb_class.read_packet(&mut ack), delay_ms(300)).await {
                        Either::First(Ok(n)) if n > 0 => {
                            defmt::info!("panic usb: host acked");
                            break;
                        }
                        Either::First(Err(_)) => return, // disconnected
                        _ => {}                          // ZLP or timeout: send again
                    }
                }
            },
            async {
                for _ in 0..3 {
                    delay_ms(10_000).await;
                }
            },
        )
        .await;

        match result {
            Either::First(_) => defmt::info!("panic usb: sent"),
            Either::Second(_) => defmt::info!("panic usb: gave up"),
        }
        cortex_m::peripheral::SCB::sys_reset();
    });
    let mut future = core::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Pending => cortex_m::asm::nop(),
            Poll::Ready((never, _)) => match never {},
        }
    }
}
