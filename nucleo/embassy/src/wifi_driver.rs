use defmt::*;
use embassy_stm32::usart::Uart;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embedded_io_async::Write;
use {defmt_rtt as _, panic_probe as _};

static TO_WIFI_CHANNEL: Channel<ThreadModeRawMutex, &[u8], 4> = Channel::new();
static FROM_WIFI_CHANNEL: Channel<ThreadModeRawMutex, ([u8; 256], usize), 4> = Channel::new();

// Set = AT+<x>=<...>
// Inquiry = AT+<x>?
// Test = AT+<x>=?
// Execute = AT+<x>

const AT: &[u8] = b"AT\r\n";
const AT_RST: &[u8] = b"AT+RST\r\n";
const AT_GMR: &[u8] = b"AT+GMR\r\n";
macro_rules! AT_GSLP {
    ($expr:expr) => {
        const_format::formatcp!("AT+GSLP={}\r\n", $expr).as_bytes()
    };
}

#[embassy_executor::task]
pub(crate) async fn wifi(mut w: Uart<'static, embassy_stm32::mode::Async>) {
    loop {
        w.write_all(TO_WIFI_CHANNEL.receive().await).await.unwrap();

        let mut buf = [0; 256];
        let l = w.read_until_idle(&mut buf).await.unwrap();

        FROM_WIFI_CHANNEL.send((buf, l)).await;
    }
}

pub struct VersionInfo {
    pub at_version: heapless::String<32>,
    pub sdk_version: heapless::String<32>,
    pub compile_time: heapless::String<32>,
}

/// Test the setup function of the module, returns true if response is OK.
pub async fn test_startup() -> bool {
    TO_WIFI_CHANNEL.send(AT).await;

    let (out, len) = FROM_WIFI_CHANNEL.receive().await;
    &out[..len] == b"AT\r\n\r\nOK\r\n"
}

/// Restart the module, returns true if response is OK.
pub async fn restart() -> bool {
    TO_WIFI_CHANNEL.send(AT_RST).await;

    let (out, len) = FROM_WIFI_CHANNEL.receive().await;
    &out[..len] == b"AT+RST\r\n\r\nOK\r\n"
}

/// Gets the versions of AT commands and SDK the module is using.
pub async fn version_info() -> VersionInfo {
    TO_WIFI_CHANNEL.send(AT_GMR).await;

    let (out, len) = FROM_WIFI_CHANNEL.receive().await;
    let buf = &out[..len];

    const AT_IDX: usize = "AT+GMR\r\nAT version:".len();
    const AT_LEN: usize = "#.#.#.#(MMM DD YYYY HH:MM:SS)".len();
    const SDK_IDX: usize = AT_IDX + AT_LEN + "\r\nSDK version:".len();
    const SDK_LEN: usize = "#.#.#(HHHHHHH)".len();
    const CMP_IDX: usize = SDK_IDX + SDK_LEN + "\r\ncompile time:".len();
    const CMP_LEN: usize = "MMM DD YYY HH:MM:SS".len();

    let r = VersionInfo {
        at_version: unsafe {
            heapless::String::try_from(str::from_utf8_unchecked(&buf[AT_IDX..(AT_IDX + AT_LEN)]))
                .unwrap()
        },
        sdk_version: unsafe {
            heapless::String::try_from(str::from_utf8_unchecked(&buf[SDK_IDX..(SDK_IDX + SDK_LEN)]))
                .unwrap()
        },
        compile_time: unsafe {
            heapless::String::try_from(str::from_utf8_unchecked(&buf[CMP_IDX..(CMP_IDX + CMP_LEN)]))
                .unwrap()
        },
    };

    r
}

macro_rules! asdf {
    () => {{
        TO_WIFI_CHANNEL.send(AT_GMR).await;
        let (out, len) = FROM_WIFI_CHANNEL.receive().await;
        &out[..len]
    }};
}

pub async fn sleep(ms: u32) -> bool {
    let asdff = asdf!();

    true
}

#[derive(defmt::Format, PartialEq)]
pub enum WiFiMode {
    /// Station Mode
    Client = 1,
    /// SoftAP Mode
    AccessPoint,
    /// Both Station and SoftAP Mode
    ClientAndAccessPoint,
}
impl WiFiMode {
    pub const fn from_u8(n: u8) -> Self {
        match n {
            1 => Self::Client,
            2 => Self::AccessPoint,
            3 => Self::ClientAndAccessPoint,
            // SAFETY: the CWMODE functions only return 1, 2, or 3.
            _ => unsafe { core::hint::unreachable_unchecked() },
        }
    }
}

// CWMODE
pub async fn current_wifi_mode() -> WiFiMode {
    TO_WIFI_CHANNEL.send(b"AT+CWMODE_CUR?\r\n").await;

    let (out, len) = FROM_WIFI_CHANNEL.receive().await;
    let buf = &out[..len];

    const IDX: usize = "AT+CWMODE_CUR?\r\n+CWMODE_CUR:".len();

    let n = u8::from_str_radix(
        unsafe { str::from_utf8_unchecked(&buf[IDX..(IDX + 1)]) },
        10,
    )
    .unwrap();

    WiFiMode::from_u8(n)
}

// CWJAP
// CWLAP
// CWQAP
// CWSAP
// CWLIF
// CIPSTATUS
// CIPSTART
// CIPCLOSE
// CIFSR
// CIPMUX
// CIPSERVER
// CIPMODE
// CIPSTO
