use embassy_stm32::Peri;
use embassy_stm32::peripherals::{PA11, PA12, USB_OTG_FS};
use embassy_stm32::usb::{Config as UsbDriverConfig, Driver, InterruptHandler};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, UsbDevice};
use static_cell::StaticCell;

pub type MyUsbDriver = Driver<'static, USB_OTG_FS>;

static EP_OUT_BUFFER: StaticCell<[u8; 256]> = StaticCell::new();
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static STATE: StaticCell<State> = StaticCell::new();

#[embassy_executor::task]
pub async fn usb_task(mut usb: embassy_usb::UsbDevice<'static, MyUsbDriver>) -> ! {
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
) -> MyUsbDriver {
    let mut config = UsbDriverConfig::default();
    config.vbus_detection = false;
    let ep_out_buffer = EP_OUT_BUFFER.init([0u8; 256]);
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
    let driver = new_driver(peri, crate::Irqs, dp, dm);

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
