#[macro_use]
mod helper;

use alloc::{boxed::Box, string::String, vec::Vec};
use core::{
    fmt::Write as _,
    future::Future,
    pin::Pin,
};
use defmt_rtt as _;
use embassy_stm32::{
    mode::Async,
    usart::{Uart, UartRx, UartTx},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel, mutex::Mutex};
use embedded_io_async::Write as _;

pub const THE_SIZE: usize = 8192;
//pub const MAX_RET_SIZE: usize = 2070;

pub(crate) trait WaitESP01 {
    /// Provides optional functionality to negate possible junk bytes sent from the module.
    async fn wait_wifi(self) -> Self;
}
impl WaitESP01 for Uart<'_, embassy_stm32::mode::Async> {
    async fn wait_wifi(mut self) -> Self {
        // In the chance only of when the device first turns on,
        // the control board will load faster than the wifi module,
        // in which case we must keep all contact away and test the module
        // until multiple successive tests have passed.
        let mut i = 0;
        loop {
            let mut buf = [0; 1024];
            embedded_io_async::Write::write_all(&mut self, b"AT\r\n")
                .await
                .unwrap();
            match self.read_until_idle(&mut buf).await {
                Ok(sz) => {
                    let against = unsafe { str::from_utf8_unchecked(&buf[..sz]) };
                    if against.contains("OK\r\n") {
                        i += 1;
                    }
                }
                Err(_) => {
                    i = 0;
                }
            }
            if i > 3 {
                return self;
            }
        }
    }
}

#[derive(Clone, Debug, defmt::Format)]
pub struct IncomingData {
    pub link_id: u8,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, defmt::Format)]
pub enum WifiEvent {
    Connected(u8),
    Closed(u8),
    WifiConnected,
    WifiGotIp,
    WifiDisconnect,
    Ready,
}

pub type WifiCallbackFuture = Pin<Box<dyn Future<Output = ()> + 'static>>;
pub type WifiEventCallback = fn(WifiEvent) -> WifiCallbackFuture;
pub type WifiDataCallback = fn(IncomingData) -> WifiCallbackFuture;

#[derive(Clone, Copy)]
pub struct WifiCallbacks {
    pub on_event: Option<WifiEventCallback>,
    pub on_data: Option<WifiDataCallback>,
}

impl WifiCallbacks {
    pub const fn none() -> Self {
        Self {
            on_event: None,
            on_data: None,
        }
    }
}

enum WifiCallbackMessage {
    Event(WifiEvent),
    Data(IncomingData),
}

static WIFI_CALLBACK_CHANNEL: Channel<ThreadModeRawMutex, WifiCallbackMessage, 8> = Channel::new();

async fn notify_event(callbacks: &WifiCallbacks, event: WifiEvent) {
    if callbacks.on_event.is_some() {
        let _ = WIFI_CALLBACK_CHANNEL.try_send(WifiCallbackMessage::Event(event.clone()));
    }
    let _ = WIFI_EVENT_CHANNEL.try_send(event);
}

async fn notify_data(callbacks: &WifiCallbacks, data: IncomingData) {
    if callbacks.on_data.is_some() {
        let _ = WIFI_CALLBACK_CHANNEL.try_send(WifiCallbackMessage::Data(data));
    }
}

pub static INCOMING_DATA_CHANNEL: Channel<ThreadModeRawMutex, IncomingData, 8> = Channel::new();
pub static WIFI_EVENT_CHANNEL: Channel<ThreadModeRawMutex, WifiEvent, 8> = Channel::new();
static TO_WIFI_CHANNEL: Channel<ThreadModeRawMutex, WifiCmd, 4> = Channel::new();
static FROM_WIFI_CHANNEL: Channel<ThreadModeRawMutex, ([u8; THE_SIZE], usize), 4> = Channel::new();
static FORMAT_HOLDER: Mutex<ThreadModeRawMutex, String> = Mutex::new(String::new());
static ECHO: Mutex<ThreadModeRawMutex, bool> = Mutex::new(true);

static DMA_BUF: static_cell::StaticCell<[u8; 4096]> = static_cell::StaticCell::new();

enum ParseState {
    Normal,
    ParsingIpdHeader {
        header_buf: Vec<u8>,
    },
    ReadingIpdPayload {
        link_id: u8,
        total_len: usize,
        received: usize,
        payload: Vec<u8>,
    },
}

fn trim_ascii_slice(mut s: &[u8]) -> &[u8] {
    while let Some((&first, rest)) = s.split_first() {
        if first.is_ascii_whitespace() {
            s = rest;
        } else {
            break;
        }
    }
    while let Some((&last, rest)) = s.split_last() {
        if last.is_ascii_whitespace() {
            s = rest;
        } else {
            break;
        }
    }
    s
}

#[embassy_executor::task]
pub(crate) async fn wifi_writer(mut tx: UartTx<'static, Async>) {
    loop {
        let cmd = TO_WIFI_CHANNEL.receive().await;
        tx.write_all(cmd.as_ref()).await.unwrap();
    }
}

#[embassy_executor::task]
pub(crate) async fn wifi_callback_runner(callbacks: WifiCallbacks) {
    loop {
        match WIFI_CALLBACK_CHANNEL.receive().await {
            WifiCallbackMessage::Event(event) => {
                if let Some(on_event) = callbacks.on_event {
                    on_event(event).await;
                }
            }
            WifiCallbackMessage::Data(data) => {
                if let Some(on_data) = callbacks.on_data {
                    on_data(data).await;
                }
            }
        }
    }
}

#[embassy_executor::task]
pub(crate) async fn wifi_reader(rx: UartRx<'static, Async>, callbacks: WifiCallbacks) {
    let dma_buf = DMA_BUF.init([0u8; 4096]);
    let mut rx = rx.into_ring_buffered(dma_buf);

    // Initial check / echo turn-off
    {
        let mut buf = [0u8; 32];
        TO_WIFI_CHANNEL.send(WifiCmd::Static(b"ATE0\r\n")).await;
        if let Ok(sz) = rx.read(&mut buf).await {
            *ECHO.lock().await = false;
            defmt::info!("Echo disabled on ESP-01 (read {} initial bytes)", sz);
        }
    }

    let mut state = ParseState::Normal;
    let mut line_buf: Vec<u8> = Vec::with_capacity(128);
    let mut cmd_acc: Vec<u8> = Vec::with_capacity(512);
    let mut read_chunk = [0u8; 512];

    loop {
        let n = match rx.read(&mut read_chunk).await {
            Ok(0) => continue,
            Ok(n) => n,
            Err(err) => {
                defmt::error!("ESP-01 UART RX error: {:?}", defmt::Debug2Format(&err));
                continue;
            }
        };

        let mut idx = 0;
        while idx < n {
            match &mut state {
                ParseState::ReadingIpdPayload {
                    link_id,
                    total_len,
                    received,
                    payload,
                } => {
                    let needed = *total_len - *received;
                    let available = n - idx;
                    let take = core::cmp::min(needed, available);

                    payload.extend_from_slice(&read_chunk[idx..idx + take]);
                    *received += take;
                    idx += take;

                    if *received >= *total_len {
                        let finished_link_id = *link_id;
                        let finished_payload = core::mem::take(payload);
                        let pkt = IncomingData {
                            link_id: finished_link_id,
                            data: finished_payload,
                        };

                        notify_data(&callbacks, pkt.clone()).await;

                        if let Err(_) = INCOMING_DATA_CHANNEL.try_send(pkt) {
                            defmt::warn!(
                                "INCOMING_DATA_CHANNEL full, dropped packet for link {}",
                                finished_link_id
                            );
                        }

                        state = ParseState::Normal;
                    }
                }

                ParseState::ParsingIpdHeader { header_buf } => {
                    let b = read_chunk[idx];
                    idx += 1;

                    if b == b':' {
                        let header_str = core::str::from_utf8(header_buf).unwrap_or("");
                        let mut parts = header_str.split(',');
                        let mut link_id = 0u8;
                        let mut len = 0usize;

                        if let Some(first) = parts.next() {
                            if let Some(second) = parts.next() {
                                link_id = first.trim().parse().unwrap_or(0);
                                len = second.trim().parse().unwrap_or(0);
                            } else {
                                len = first.trim().parse().unwrap_or(0);
                            }
                        }

                        if len == 0 {
                            let pkt = IncomingData {
                                link_id,
                                data: Vec::new(),
                            };
                            notify_data(&callbacks, pkt.clone()).await;
                            let _ = INCOMING_DATA_CHANNEL.try_send(pkt);
                            state = ParseState::Normal;
                        } else {
                            state = ParseState::ReadingIpdPayload {
                                link_id,
                                total_len: len,
                                received: 0,
                                payload: Vec::with_capacity(len),
                            };
                        }
                    } else if header_buf.len() < 64 {
                        header_buf.push(b);
                    } else {
                        defmt::error!("Malformed +IPD header, resetting state");
                        state = ParseState::Normal;
                    }
                }

                ParseState::Normal => {
                    let b = read_chunk[idx];
                    idx += 1;

                    line_buf.push(b);

                    // Detect "+IPD," prefix anywhere in line
                    if line_buf.ends_with(b"+IPD,") {
                        let prefix_pos = line_buf.len() - 5;
                        if prefix_pos > 0 {
                            cmd_acc.extend_from_slice(&line_buf[..prefix_pos]);
                        }
                        line_buf.clear();
                        state = ParseState::ParsingIpdHeader {
                            header_buf: Vec::new(),
                        };
                        continue;
                    }

                    // Detect prompt "> " or ">"
                    if line_buf.ends_with(b"> ") || line_buf == b">" {
                        cmd_acc.extend_from_slice(&line_buf);
                        line_buf.clear();

                        let mut out = [0u8; THE_SIZE];
                        let len = core::cmp::min(cmd_acc.len(), THE_SIZE);
                        out[..len].copy_from_slice(&cmd_acc[..len]);
                        let _ = FROM_WIFI_CHANNEL.try_send((out, len));
                        cmd_acc.clear();
                        continue;
                    }

                    // Line terminator
                    if b == b'\n' {
                        let trimmed = trim_ascii_slice(&line_buf);

                        let is_urc = if trimmed == b"WIFI CONNECTED" {
                            notify_event(&callbacks, WifiEvent::WifiConnected).await;
                            true
                        } else if trimmed == b"WIFI GOT IP" {
                            notify_event(&callbacks, WifiEvent::WifiGotIp).await;
                            true
                        } else if trimmed == b"WIFI DISCONNECT" {
                            notify_event(&callbacks, WifiEvent::WifiDisconnect).await;
                            true
                        } else if trimmed == b"ready" {
                            notify_event(&callbacks, WifiEvent::Ready).await;
                            true
                        } else if trimmed.ends_with(b",CONNECT") {
                            if let Some(id_byte) = trimmed.strip_suffix(b",CONNECT") {
                                if let Ok(id_str) = core::str::from_utf8(id_byte) {
                                    if let Ok(id) = id_str.trim().parse::<u8>() {
                                        notify_event(&callbacks, WifiEvent::Connected(id)).await;
                                    }
                                }
                            }
                            true
                        } else if trimmed.ends_with(b",CLOSED") {
                            if let Some(id_byte) = trimmed.strip_suffix(b",CLOSED") {
                                if let Ok(id_str) = core::str::from_utf8(id_byte) {
                                    if let Ok(id) = id_str.trim().parse::<u8>() {
                                        notify_event(&callbacks, WifiEvent::Closed(id)).await;
                                    }
                                }
                            }
                            true
                        } else if trimmed == b"busy p..." || trimmed == b"busy s..." {
                            defmt::warn!("ESP-01 busy");
                            true
                        } else {
                            false
                        };

                        if !is_urc {
                            cmd_acc.extend_from_slice(&line_buf);

                            let is_terminal = trimmed == b"OK"
                                || trimmed == b"ERROR"
                                || trimmed == b"FAIL"
                                || trimmed == b"SEND OK"
                                || trimmed == b"SEND FAIL"
                                || trimmed == b"ALREADY CONNECTED"
                                || trimmed == b"ALREADY CONNECT";

                            if is_terminal {
                                let mut out = [0u8; THE_SIZE];
                                let len = core::cmp::min(cmd_acc.len(), THE_SIZE);
                                out[..len].copy_from_slice(&cmd_acc[..len]);
                                let _ = FROM_WIFI_CHANNEL.try_send((out, len));
                                cmd_acc.clear();
                            }
                        }

                        line_buf.clear();
                    }
                }
            }
        }
    }
}

#[derive(defmt::Format)]
pub struct VersionInfo {
    pub at_version: String,
    pub sdk_version: String,
    pub compile_time: String,
}

#[derive(defmt::Format)]
pub enum SleepMode {
    Disabled,
    Light,
    Modem,
}
impl SleepMode {
    const fn to_u8(&self) -> u8 {
        match self {
            Self::Disabled => 0,
            Self::Light => 1,
            Self::Modem => 2,
        }
    }
    const fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Disabled,
            1 => Self::Light,
            2 => Self::Modem,
            _ => unsafe { core::hint::unreachable_unchecked() },
        }
    }
}

#[derive(defmt::Format)]
pub enum WifiMode {
    Station = 1,
    AccessPoint,
    StationAccessPoint,
}
impl WifiMode {
    const fn to_u8(&self) -> u8 {
        match self {
            Self::Station => 1,
            Self::AccessPoint => 2,
            Self::StationAccessPoint => 3,
        }
    }
    const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Station,
            2 => Self::AccessPoint,
            3 => Self::StationAccessPoint,
            _ => unsafe { core::hint::unreachable_unchecked() },
        }
    }
}

#[derive(defmt::Format)]
pub struct WifiInfo {
    pub ssid: String,
    pub bssid: String,
    pub channel: u8,
    pub rssi: i8,
}

#[derive(defmt::Format)]
pub struct DhcpState {
    pub soft_ap_enabled: bool,
    pub station_enabled: bool,
}

#[derive(defmt::Format)]
pub struct DhcpServerRange {
    pub lease_time_minutes: u16,
    pub start_ip: String,
    pub end_ip: String,
}

pub enum WifiCmd {
    Static(&'static [u8]),
    Dynamic(Vec<u8>),
}
impl AsRef<[u8]> for WifiCmd {
    fn as_ref(&self) -> &[u8] {
        match self {
            WifiCmd::Static(slice) => slice,
            WifiCmd::Dynamic(vec) => vec.as_slice(),
        }
    }
}

fn trim_quotes(v: &str) -> &str {
    v.trim().trim_matches('"')
}

fn first_data_line<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    for line in s.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some(rest.trim());
        }
    }
    None
}

fn first_number<T: core::str::FromStr>(s: &str) -> Option<T> {
    let bytes = s.as_bytes();
    let mut start = None;

    for (idx, b) in bytes.iter().enumerate() {
        if b.is_ascii_digit() {
            start = Some(idx);
            break;
        }
    }

    let start = start?;
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }

    s[start..end].parse().ok()
}

fn response_to_string(buf: &[u8], len: usize) -> Option<String> {
    String::try_from(unsafe { str::from_utf8_unchecked(&buf[..len]) }).ok()
}

/// Returns true if module is online and running.
pub async fn test_startup() -> bool {
    WRITE_READ!(AT!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

/// Resets the module.  Returns true if successful.
pub async fn reset() -> bool {
    WRITE_READ!(AT_RST!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

// Gets the versions of the AT commands and SDK the module is using.
pub async fn version_info() -> Option<VersionInfo> {
    WRITE_READ!(AT_GMR!(Execute), buf, _);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf) };
        let at_idx = s.find("AT version:").unwrap() + "AT version:".len();
        const AT_LEN: usize = "#.#.#.#(MMM DD YYYY HH:MM:SS)".len();
        let sdk_idx = s.find("SDK version:").unwrap() + "SDK version:".len();
        const SDK_LEN: usize = "#.#.#(HHHHHHH)".len();
        let cmp_idx = s.find("compile time:").unwrap() + "compile time:".len();
        const CMP_LEN: usize = "MMM DD YYY HH:MM:SS".len();

        let r = unsafe {
            VersionInfo {
                at_version: String::try_from(str::from_utf8_unchecked(
                    &buf[at_idx..(at_idx + AT_LEN)],
                ))
                .unwrap(),
                sdk_version: String::try_from(str::from_utf8_unchecked(
                    &buf[sdk_idx..(sdk_idx + SDK_LEN)],
                ))
                .unwrap(),
                compile_time: String::try_from(str::from_utf8_unchecked(
                    &buf[cmp_idx..(cmp_idx + CMP_LEN)],
                ))
                .unwrap(),
            }
        };

        Some(r)
    } else {
        None
    }
}

/// Puts the module into a deep-sleep and wakes up after the amount of time.
///
/// ## Adjustments Needed
///
/// For this to work correctly, make sure to connect the XPD_DCDC pin to the EXT_RSTB pin.
///
/// Some smaller modules (e.g. ESP-01) do not support such connections and must either use a light-sleep or have a separate controller handle it.
pub async fn deep_sleep(ms: u32) -> bool {
    AT_GSLP!(Set cmd - ms);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

/// Entered commands are echoed back.
pub async fn echo_off() -> bool {
    WRITE_READ!(ATE0!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

/// Entered commands are not echoed back.
pub async fn echo_on() -> bool {
    WRITE_READ!(ATE1!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

/// Reset all parameters saved in flash, restoring the factory default settings of the module.
///
/// The module will be restarted when this command is executed.
pub async fn factory_reset() -> bool {
    WRITE_READ!(AT_RESTORE!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

/// Set's the module's current UART configuration.
///
/// This will NOT store in the flash.
pub async fn set_current_uart_config(
    baud: u32,
    databits: u8,
    stopbits: u8,
    parity: u8,
    flowctl: u8,
) -> bool {
    AT_UART!(CUR Set cmd - baud, databits, stopbits, parity, flowctl);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

/// Set's the module's default UART configuration.
///
/// This will store in the flash.
pub async fn set_default_uart_config(
    baud: u32,
    databits: u8,
    stopbits: u8,
    parity: u8,
    flowctl: u8,
) -> bool {
    AT_UART!(DEF Set cmd - baud, databits, stopbits, parity, flowctl);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

/// Gets the module's current sleep mode.
pub async fn get_sleep_mode() -> Option<SleepMode> {
    WRITE_READ!(AT_SLEEP!(Query), buf, _);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf) };
        let idx = s.find("+SLEEP:").unwrap() + "+SLEEP:".len();

        let asdf =
            unsafe { String::try_from(str::from_utf8_unchecked(&buf[idx..(idx + 1)])).unwrap() };
        Some(SleepMode::from_u8(asdf.parse().unwrap()))
    } else {
        None
    }
}

/// Sets the module's sleep mode.
pub async fn set_sleep_mode(mode: SleepMode) -> bool {
    AT_SLEEP!(Set cmd - mode.to_u8());
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

/// Sets the maximum value of the RF TX power.
pub async fn set_max_tx_power(tx_power: u8) -> bool {
    AT_RFPOWER!(Set cmd - tx_power);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

/// Gets the power voltage of the module's VDD3P3.  TOUT pin has to be suspended.
pub async fn get_vdd3p3_power_voltage() -> Option<u16> {
    WRITE_READ!(AT_RFVDD!(Query), buf, _);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf) };
        let idx = s.find("+RFVDD:").unwrap() + "+RFVDD:".len();

        let asdf =
            unsafe { String::try_from(str::from_utf8_unchecked(&buf[idx..(idx + 4)])).unwrap() };
        Some(asdf.parse().unwrap())
    } else {
        None
    }
}

/// Sets the power voltage of the module's VDD3P3.
pub async fn set_vdd3p3_power_voltage(vdd3p3: u16) -> bool {
    AT_RFVDD!(Set cmd - vdd3p3);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

/// Automatically set RF TX power according to VDD3P3.  TOUT pin has to be suspended.
pub async fn set_rf_tx_to_vdd3p3() -> bool {
    WRITE_READ!(AT_RFVDD!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

/// WIP
pub async fn current_wifi_mode_scope() -> Option<(u8, u8)> {
    WRITE_READ!(AT_CWMODE!(CUR Test), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let modes = first_data_line(s, "+CWMODE_CUR:(")
            .or_else(|| first_data_line(s, "+CWMODE:("))?
            .trim_end_matches(')');

        let mut first = None;
        let mut last = None;
        for ch in modes.chars() {
            if let Some(v) = ch.to_digit(10) {
                let v = v as u8;
                if first.is_none() {
                    first = Some(v);
                }
                last = Some(v);
            }
        }

        Some((first?, last?))
    } else {
        None
    }
}

/// Gets the current wifi mode.
pub async fn current_wifi_mode() -> Option<WifiMode> {
    WRITE_READ!(AT_CWMODE!(CUR Query), buf, _);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf) };
        let idx = s.find("+CWMODE_CUR:").unwrap() + "+CWMODE_CUR:".len();

        let asdf =
            unsafe { String::try_from(str::from_utf8_unchecked(&buf[idx..(idx + 1)])).unwrap() };
        Some(WifiMode::from_u8(asdf.parse().unwrap()))
    } else {
        None
    }
}

/// Sets the current wifi mode.
pub async fn set_current_wifi_mode(mode: WifiMode) -> bool {
    AT_CWMODE!(CUR Set cmd - mode.to_u8());
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

/// WIP
pub async fn default_wifi_mode_scope() -> Option<(u8, u8)> {
    WRITE_READ!(AT_CWMODE!(DEF Test), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let modes = first_data_line(s, "+CWMODE_DEF:(")
            .or_else(|| first_data_line(s, "+CWMODE:("))?
            .trim_end_matches(')');

        let mut first = None;
        let mut last = None;
        for ch in modes.chars() {
            if let Some(v) = ch.to_digit(10) {
                let v = v as u8;
                if first.is_none() {
                    first = Some(v);
                }
                last = Some(v);
            }
        }

        Some((first?, last?))
    } else {
        None
    }
}

/// Gets the default wifi mode.
pub async fn default_wifi_mode() -> Option<WifiMode> {
    WRITE_READ!(AT_CWMODE!(DEF Query), buf, _);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf) };
        let idx = s.find("+CWMODE_DEF:").unwrap() + "+CWMODE_DEF:".len();

        let asdf =
            unsafe { String::try_from(str::from_utf8_unchecked(&buf[idx..(idx + 1)])).unwrap() };
        Some(WifiMode::from_u8(asdf.parse().unwrap()))
    } else {
        None
    }
}

/// Sets the default wifi mode.
pub async fn set_default_wifi_mode(mode: WifiMode) -> bool {
    AT_CWMODE!(DEF Set cmd - mode.to_u8());
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn current_connected_network() -> Option<WifiInfo> {
    WRITE_READ!(AT_CWJAP!(CUR Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let line = first_data_line(s, "+CWJAP_CUR:").or_else(|| first_data_line(s, "+CWJAP:"))?;

        let mut it = line.splitn(4, ',');
        let ssid = trim_quotes(it.next()?);
        let bssid = trim_quotes(it.next()?);
        let channel = trim_quotes(it.next()?).parse().ok()?;
        let rssi = trim_quotes(it.next()?).parse().ok()?;

        Some(WifiInfo {
            ssid: String::try_from(ssid).ok()?,
            bssid: String::try_from(bssid).ok()?,
            channel,
            rssi,
        })
    } else {
        None
    }
}

pub async fn default_connected_network() -> Option<WifiInfo> {
    WRITE_READ!(AT_CWJAP!(DEF Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let line = first_data_line(s, "+CWJAP_DEF:").or_else(|| first_data_line(s, "+CWJAP:"))?;

        let mut it = line.splitn(4, ',');
        let ssid = trim_quotes(it.next()?);
        let bssid = trim_quotes(it.next()?);
        let channel = trim_quotes(it.next()?).parse().ok()?;
        let rssi = trim_quotes(it.next()?).parse().ok()?;

        Some(WifiInfo {
            ssid: String::try_from(ssid).ok()?,
            bssid: String::try_from(bssid).ok()?,
            channel,
            rssi,
        })
    } else {
        None
    }
}

pub async fn connect_to_ap_current(ssid: &str, pwd: &str, bssid: Option<&str>) -> bool {
    match bssid {
        Some(bssid) => {
            AT_CWJAP!(CUR Set cmd - ssid, pwd, bssid);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        None => {
            AT_CWJAP!(CUR Set cmd - ssid, pwd);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
    }
}

pub async fn connect_to_ap_default(ssid: &str, pwd: &str, bssid: Option<&str>) -> bool {
    match bssid {
        Some(bssid) => {
            AT_CWJAP!(DEF Set cmd - ssid, pwd, bssid);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        None => {
            AT_CWJAP!(DEF Set cmd - ssid, pwd);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
    }
}

pub async fn set_ap_list_options(sort_enable: bool, mask: u8) -> bool {
    let sort_enable = if sort_enable { 1 } else { 0 };
    AT_CWLAPOPT!(Set cmd - sort_enable, mask);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn list_available_aps_raw() -> Option<String> {
    WRITE_READ!(AT_CWLAP!(Execute), buf, l);
    if TRUE_ON_OK!(buf) {
        response_to_string(&buf, l)
    } else {
        defmt::info!("{}", str::from_utf8(&buf[..l]).unwrap());
        None
    }
}

pub async fn list_available_aps_by_ssid_raw(ssid: &str) -> Option<String> {
    AT_CWLAP!(Set cmd - ssid);
    WRITE_READ!(cmd, buf, l);
    if TRUE_ON_OK!(buf) {
        response_to_string(&buf, l)
    } else {
        None
    }
}

pub async fn list_available_aps_by_ssid_mac_channel_raw(
    ssid: &str,
    mac: &str,
    channel: u8,
) -> Option<String> {
    AT_CWLAP!(Set cmd - ssid, mac, channel);
    WRITE_READ!(cmd, buf, l);
    if TRUE_ON_OK!(buf) {
        response_to_string(&buf, l)
    } else {
        None
    }
}

pub async fn disconnect_from_ap() -> bool {
    WRITE_READ!(AT_CWQAP!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn current_softap_config_raw() -> Option<String> {
    WRITE_READ!(AT_CWSAP!(CUR Query), buf, l);
    if TRUE_ON_OK!(buf) {
        response_to_string(&buf, l)
    } else {
        None
    }
}

pub async fn default_softap_config_raw() -> Option<String> {
    WRITE_READ!(AT_CWSAP!(DEF Query), buf, l);
    if TRUE_ON_OK!(buf) {
        response_to_string(&buf, l)
    } else {
        None
    }
}

pub async fn set_current_softap_config(
    ssid: &str,
    pwd: &str,
    channel: u8,
    ecn: u8,
    max_conn: Option<u8>,
    ssid_hidden: Option<u8>,
) -> bool {
    match (max_conn, ssid_hidden) {
        (None, None) => {
            AT_CWSAP!(CUR Set cmd - ssid, pwd, channel, ecn);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(max_conn), None) => {
            AT_CWSAP!(CUR Set cmd - ssid, pwd, channel, ecn, max_conn);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (None, Some(ssid_hidden)) => {
            AT_CWSAP!(CUR Set cmd - ssid, pwd, channel, ecn, , ssid_hidden);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(max_conn), Some(ssid_hidden)) => {
            AT_CWSAP!(CUR Set cmd - ssid, pwd, channel, ecn, max_conn, ssid_hidden);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
    }
}

pub async fn set_default_softap_config(
    ssid: &str,
    pwd: &str,
    channel: u8,
    ecn: u8,
    max_conn: Option<u8>,
    ssid_hidden: Option<u8>,
) -> bool {
    match (max_conn, ssid_hidden) {
        (None, None) => {
            AT_CWSAP!(DEF Set cmd - ssid, pwd, channel, ecn);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(max_conn), None) => {
            AT_CWSAP!(DEF Set cmd - ssid, pwd, channel, ecn, max_conn);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (None, Some(ssid_hidden)) => {
            AT_CWSAP!(DEF Set cmd - ssid, pwd, channel, ecn, , ssid_hidden);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(max_conn), Some(ssid_hidden)) => {
            AT_CWSAP!(DEF Set cmd - ssid, pwd, channel, ecn, max_conn, ssid_hidden);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
    }
}

pub async fn connected_station_list_raw() -> Option<String> {
    WRITE_READ!(AT_CWLIF!(Execute), buf, l);
    if TRUE_ON_OK!(buf) {
        response_to_string(&buf, l)
    } else {
        None
    }
}

pub async fn current_dhcp_state() -> Option<DhcpState> {
    WRITE_READ!(AT_CWDHCP!(CUR Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let bits: u8 = first_number(s)?;
        Some(DhcpState {
            soft_ap_enabled: bits & 0b01 != 0,
            station_enabled: bits & 0b10 != 0,
        })
    } else {
        None
    }
}

pub async fn default_dhcp_state() -> Option<DhcpState> {
    WRITE_READ!(AT_CWDHCP!(DEF Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let bits: u8 = first_number(s)?;
        Some(DhcpState {
            soft_ap_enabled: bits & 0b01 != 0,
            station_enabled: bits & 0b10 != 0,
        })
    } else {
        None
    }
}

pub async fn set_current_dhcp(mode: u8, enable: bool) -> bool {
    let enable = if enable { 1 } else { 0 };
    AT_CWDHCP!(CUR Set cmd - mode, enable);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn set_default_dhcp(mode: u8, enable: bool) -> bool {
    let enable = if enable { 1 } else { 0 };
    AT_CWDHCP!(DEF Set cmd - mode, enable);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn current_dhcp_server_range() -> Option<DhcpServerRange> {
    WRITE_READ!(AT_CWDHCPS!(CUR Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let line =
            first_data_line(s, "+CWDHCPS_CUR=").or_else(|| first_data_line(s, "+CWDHCPS_CUR:"))?;
        let mut it = line.splitn(3, ',');
        let lease_time_minutes = trim_quotes(it.next()?).parse().ok()?;
        let start_ip = String::try_from(trim_quotes(it.next()?)).ok()?;
        let end_ip = String::try_from(trim_quotes(it.next()?)).ok()?;

        Some(DhcpServerRange {
            lease_time_minutes,
            start_ip,
            end_ip,
        })
    } else {
        None
    }
}

pub async fn default_dhcp_server_range() -> Option<DhcpServerRange> {
    WRITE_READ!(AT_CWDHCPS!(DEF Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let line =
            first_data_line(s, "+CWDHCPS_DEF=").or_else(|| first_data_line(s, "+CWDHCPS_DEF:"))?;
        let mut it = line.splitn(3, ',');
        let lease_time_minutes = trim_quotes(it.next()?).parse().ok()?;
        let start_ip = String::try_from(trim_quotes(it.next()?)).ok()?;
        let end_ip = String::try_from(trim_quotes(it.next()?)).ok()?;

        Some(DhcpServerRange {
            lease_time_minutes,
            start_ip,
            end_ip,
        })
    } else {
        None
    }
}

pub async fn disable_current_dhcp_server_custom_range() -> bool {
    AT_CWDHCPS!(CUR Set cmd - 0);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn set_current_dhcp_server_range(
    lease_time_minutes: u16,
    start_ip: &str,
    end_ip: &str,
) -> bool {
    AT_CWDHCPS!(CUR Set cmd - 1, lease_time_minutes, start_ip, end_ip);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn disable_default_dhcp_server_custom_range() -> bool {
    AT_CWDHCPS!(DEF Set cmd - 0);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn set_default_dhcp_server_range(
    lease_time_minutes: u16,
    start_ip: &str,
    end_ip: &str,
) -> bool {
    AT_CWDHCPS!(DEF Set cmd - 1, lease_time_minutes, start_ip, end_ip);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn set_auto_connect(enable: bool) -> bool {
    let enable = if enable { 1 } else { 0 };
    AT_CWAUTOCONN!(Set cmd - enable);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn current_station_mac() -> Option<String> {
    WRITE_READ!(AT_CIPSTAMAC!(CUR Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let mac = first_data_line(s, "+CIPSTAMAC_CUR:")?;
        String::try_from(trim_quotes(mac)).ok()
    } else {
        None
    }
}

pub async fn default_station_mac() -> Option<String> {
    WRITE_READ!(AT_CIPSTAMAC!(DEF Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let mac = first_data_line(s, "+CIPSTAMAC_DEF:")?;
        String::try_from(trim_quotes(mac)).ok()
    } else {
        None
    }
}

pub async fn set_current_station_mac(mac: &str) -> bool {
    AT_CIPSTAMAC!(CUR Set cmd - mac);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn set_default_station_mac(mac: &str) -> bool {
    AT_CIPSTAMAC!(DEF Set cmd - mac);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn current_softap_mac() -> Option<String> {
    WRITE_READ!(AT_CIPAPMAC!(CUR Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let mac = first_data_line(s, "+CIPAPMAC_CUR:")?;
        String::try_from(trim_quotes(mac)).ok()
    } else {
        None
    }
}

pub async fn default_softap_mac() -> Option<String> {
    WRITE_READ!(AT_CIPAPMAC!(DEF Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let mac = first_data_line(s, "+CIPAPMAC_DEF:")?;
        String::try_from(trim_quotes(mac)).ok()
    } else {
        None
    }
}

pub async fn set_current_softap_mac(mac: &str) -> bool {
    AT_CIPAPMAC!(CUR Set cmd - mac);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn set_default_softap_mac(mac: &str) -> bool {
    AT_CIPAPMAC!(DEF Set cmd - mac);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn current_station_ip() -> Option<String> {
    WRITE_READ!(AT_CIPSTA!(CUR Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let ip = first_data_line(s, "+CIPSTA_CUR:").or_else(|| first_data_line(s, "+CIPSTA:"))?;
        String::try_from(trim_quotes(ip)).ok()
    } else {
        None
    }
}

pub async fn default_station_ip() -> Option<String> {
    WRITE_READ!(AT_CIPSTA!(DEF Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let ip = first_data_line(s, "+CIPSTA_DEF:").or_else(|| first_data_line(s, "+CIPSTA:"))?;
        String::try_from(trim_quotes(ip)).ok()
    } else {
        None
    }
}

pub async fn set_current_station_ip(
    ip: &str,
    gateway: Option<&str>,
    netmask: Option<&str>,
) -> bool {
    match (gateway, netmask) {
        (None, None) => {
            AT_CIPSTA!(CUR Set cmd - ip);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(gateway), None) => {
            AT_CIPSTA!(CUR Set cmd - ip, gateway);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(gateway), Some(netmask)) => {
            AT_CIPSTA!(CUR Set cmd - ip, gateway, netmask);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (None, Some(_)) => false,
    }
}

pub async fn set_default_station_ip(
    ip: &str,
    gateway: Option<&str>,
    netmask: Option<&str>,
) -> bool {
    match (gateway, netmask) {
        (None, None) => {
            AT_CIPSTA!(DEF Set cmd - ip);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(gateway), None) => {
            AT_CIPSTA!(DEF Set cmd - ip, gateway);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(gateway), Some(netmask)) => {
            AT_CIPSTA!(DEF Set cmd - ip, gateway, netmask);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (None, Some(_)) => false,
    }
}

pub async fn current_softap_ip() -> Option<String> {
    WRITE_READ!(AT_CIPAP!(CUR Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let ip = first_data_line(s, "+CIPAP_CUR:").or_else(|| first_data_line(s, "+CIPAP:"))?;
        String::try_from(trim_quotes(ip)).ok()
    } else {
        None
    }
}

pub async fn default_softap_ip() -> Option<String> {
    WRITE_READ!(AT_CIPAP!(DEF Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let ip = first_data_line(s, "+CIPAP_DEF:").or_else(|| first_data_line(s, "+CIPAP:"))?;
        String::try_from(trim_quotes(ip)).ok()
    } else {
        None
    }
}

pub async fn set_current_softap_ip(ip: &str, gateway: Option<&str>, netmask: Option<&str>) -> bool {
    match (gateway, netmask) {
        (None, None) => {
            AT_CIPAP!(CUR Set cmd - ip);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(gateway), None) => {
            AT_CIPAP!(CUR Set cmd - ip, gateway);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(gateway), Some(netmask)) => {
            AT_CIPAP!(CUR Set cmd - ip, gateway, netmask);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (None, Some(_)) => false,
    }
}

pub async fn set_default_softap_ip(ip: &str, gateway: Option<&str>, netmask: Option<&str>) -> bool {
    match (gateway, netmask) {
        (None, None) => {
            AT_CIPAP!(DEF Set cmd - ip);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(gateway), None) => {
            AT_CIPAP!(DEF Set cmd - ip, gateway);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (Some(gateway), Some(netmask)) => {
            AT_CIPAP!(DEF Set cmd - ip, gateway, netmask);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        (None, Some(_)) => false,
    }
}

pub async fn start_smart_config() -> bool {
    WRITE_READ!(AT_CWSTARTSMART!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn start_smart_config_with_type(kind: u8) -> bool {
    AT_CWSTARTSMART!(Set cmd - kind);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn stop_smart_config() -> bool {
    WRITE_READ!(AT_CWSTOPSMART!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn start_wechat_discover(wechat_number: &str, dev_type: &str, time_s: u32) -> bool {
    AT_CWSTARTDISCOVER!(Set cmd - wechat_number, dev_type, time_s);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn stop_wechat_discover() -> bool {
    WRITE_READ!(AT_CWSTOPDISCOVER!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn set_wps(enable: bool) -> bool {
    let enable = if enable { 1 } else { 0 };
    AT_WPS!(Set cmd - enable);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn set_mdns(enable: bool, hostname: &str, server_name: &str, server_port: u16) -> bool {
    let enable = if enable { 1 } else { 0 };
    AT_MDNS!(Set cmd - enable, hostname, server_name, server_port);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn network_connection_status_raw() -> Option<String> {
    WRITE_READ!(AT_CIPSTATUS!(Execute), buf, l);
    if TRUE_ON_OK!(buf) {
        response_to_string(&buf, l)
    } else {
        None
    }
}

pub async fn dns_lookup(domain_name: &str) -> Option<String> {
    AT_CIPDOMAIN!(Set cmd - domain_name);
    WRITE_READ!(cmd, buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let ip = first_data_line(s, "+CIPDOMAIN:")?;
        String::try_from(trim_quotes(ip)).ok()
    } else {
        None
    }
}

pub async fn open_tcp_connection(remote_ip_or_domain: &str, remote_port: u16) -> bool {
    AT_CIPSTART!(Set cmd - "TCP", remote_ip_or_domain, remote_port);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn open_tcp_connection_keep_alive(
    remote_ip_or_domain: &str,
    remote_port: u16,
    keep_alive_s: u16,
) -> bool {
    AT_CIPSTART!(Set cmd - "TCP", remote_ip_or_domain, remote_port, keep_alive_s);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn open_tcp_connection_multi(
    link_id: u8,
    remote_ip_or_domain: &str,
    remote_port: u16,
) -> bool {
    AT_CIPSTART!(Set cmd - link_id, "TCP", remote_ip_or_domain, remote_port);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn open_tcp_connection_multi_keep_alive(
    link_id: u8,
    remote_ip_or_domain: &str,
    remote_port: u16,
    keep_alive_s: u16,
) -> bool {
    AT_CIPSTART!(Set cmd - link_id, "TCP", remote_ip_or_domain, remote_port, keep_alive_s);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn open_udp_transmission(
    remote_ip: &str,
    remote_port: u16,
    local_port: u16,
    mode: u8,
) -> bool {
    AT_CIPSTART!(Set cmd - "UDP", remote_ip, remote_port, local_port, mode);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn open_udp_transmission_multi(
    link_id: u8,
    remote_ip: &str,
    remote_port: u16,
    local_port: u16,
    mode: u8,
) -> bool {
    AT_CIPSTART!(Set cmd - link_id, "UDP", remote_ip, remote_port, local_port, mode);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn set_ssl_buffer_size(size: u16) -> bool {
    AT_CIPSSLSIZE!(Set cmd - size);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

async fn do_send(cmd: WifiCmd, length: u16, payload: &[u8]) -> bool {
    TO_WIFI_CHANNEL.send(cmd).await;

    // Step 1: Wait for prompt '>' or 'OK'
    let (first_buf, first_len) = FROM_WIFI_CHANNEL.receive().await;
    let first_slice = &first_buf[..first_len];

    if first_slice.windows(5).any(|w| w == b"ERROR")
        || first_slice.windows(4).any(|w| w == b"FAIL")
        || first_slice.windows(9).any(|w| w == b"link is not")
    {
        return false;
    }

    // If the prompt ">" was sent as a separate message from "OK", drain it
    if !first_slice.windows(1).any(|w| w == b">") {
        if let Ok((next_buf, next_len)) = FROM_WIFI_CHANNEL.try_receive() {
            let next_slice = &next_buf[..next_len];
            if next_slice.windows(5).any(|w| w == b"ERROR")
                || next_slice.windows(4).any(|w| w == b"FAIL")
            {
                return false;
            }
        }
    }

    // Step 2: Send the raw payload bytes
    let send_len = core::cmp::min(length as usize, payload.len());
    TO_WIFI_CHANNEL
        .send(WifiCmd::Dynamic(payload[..send_len].to_vec()))
        .await;

    // Step 3: Wait for confirmation (SEND OK / SEND FAIL / ERROR)
    loop {
        let (resp_buf, resp_len) = FROM_WIFI_CHANNEL.receive().await;
        let resp_slice = &resp_buf[..resp_len];

        // If it was another '>' prompt leftover, continue waiting for SEND OK
        let trimmed = trim_ascii_slice(resp_slice);
        if trimmed == b">" || trimmed == b"> " {
            continue;
        }

        if resp_slice.windows(7).any(|w| w == b"SEND OK") || TRUE_ON_OK!(resp_slice) {
            return true;
        }
        if resp_slice.windows(9).any(|w| w == b"SEND FAIL")
            || resp_slice.windows(5).any(|w| w == b"ERROR")
            || resp_slice.windows(4).any(|w| w == b"FAIL")
        {
            return false;
        }
    }
}

pub async fn request_send(length: u16, payload: &[u8]) -> bool {
    AT_CIPSEND!(Set cmd - length);
    do_send(cmd, length, payload).await
}

pub async fn request_send_multi(link_id: u8, length: u16, payload: &[u8]) -> bool {
    AT_CIPSEND!(Set cmd - link_id, length);
    do_send(cmd, length, payload).await
}

pub async fn request_send_udp(
    length: u16,
    remote_ip: &str,
    remote_port: u16,
    payload: &[u8],
) -> bool {
    AT_CIPSEND!(Set cmd - length, remote_ip, remote_port);
    do_send(cmd, length, payload).await
}

pub async fn request_send_udp_multi(
    link_id: u8,
    length: u16,
    remote_ip: &str,
    remote_port: u16,
    payload: &[u8],
) -> bool {
    AT_CIPSEND!(Set cmd - link_id, length, remote_ip, remote_port);
    do_send(cmd, length, payload).await
}

pub async fn enter_unvarnished_transmission() -> bool {
    WRITE_READ!(AT_CIPSEND!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn request_send_ex(length: u16) -> bool {
    AT_CIPSENDEX!(Set cmd - length);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn request_send_ex_multi(link_id: u8, length: u16) -> bool {
    AT_CIPSENDEX!(Set cmd - link_id, length);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn request_send_ex_udp(length: u16, remote_ip: &str, remote_port: u16) -> bool {
    AT_CIPSENDEX!(Set cmd - length, remote_ip, remote_port);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn request_send_ex_udp_multi(
    link_id: u8,
    length: u16,
    remote_ip: &str,
    remote_port: u16,
) -> bool {
    AT_CIPSENDEX!(Set cmd - link_id, length, remote_ip, remote_port);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn write_send_buffer(length: u16) -> bool {
    AT_CIPSENDBUF!(Set cmd - length);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn write_send_buffer_multi(link_id: u8, length: u16) -> bool {
    AT_CIPSENDBUF!(Set cmd - link_id, length);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn send_buffer_status_raw() -> Option<String> {
    WRITE_READ!(AT_CIPBUFSTATUS!(Execute), buf, l);
    if TRUE_ON_OK!(buf) {
        response_to_string(&buf, l)
    } else {
        None
    }
}

pub async fn send_buffer_status_multi_raw(link_id: u8) -> Option<String> {
    AT_CIPBUFSTATUS!(Set cmd - link_id);
    WRITE_READ!(cmd, buf, l);
    if TRUE_ON_OK!(buf) {
        response_to_string(&buf, l)
    } else {
        None
    }
}

pub async fn was_segment_sent_successfully(segment_id: u32) -> Option<bool> {
    AT_CIPCHECKSEQ!(Set cmd - segment_id);
    WRITE_READ!(cmd, buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        if s.contains("TRUE") {
            Some(true)
        } else if s.contains("FALSE") {
            Some(false)
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn was_segment_sent_successfully_multi(link_id: u8, segment_id: u32) -> Option<bool> {
    AT_CIPCHECKSEQ!(Set cmd - link_id, segment_id);
    WRITE_READ!(cmd, buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        if s.contains("TRUE") {
            Some(true)
        } else if s.contains("FALSE") {
            Some(false)
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn reset_segment_id_count() -> bool {
    WRITE_READ!(AT_CIPBUFRESET!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn reset_segment_id_count_multi(link_id: u8) -> bool {
    AT_CIPBUFRESET!(Set cmd - link_id);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn close_connection() -> bool {
    WRITE_READ!(AT_CIPCLOSE!(Execute), buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn close_connection_by_link(link_id: u8) -> bool {
    AT_CIPCLOSE!(Set cmd - link_id);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn local_ip_raw() -> Option<String> {
    WRITE_READ!(AT_CIFSR!(Execute), buf, l);
    if TRUE_ON_OK!(buf) {
        response_to_string(&buf, l)
    } else {
        None
    }
}

pub async fn multiple_connections_mode() -> Option<u8> {
    WRITE_READ!(AT_CIPMUX!(Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let mode = first_data_line(s, "+CIPMUX:")?;
        mode.parse().ok()
    } else {
        None
    }
}

pub async fn set_multiple_connections_mode(mode: u8) -> bool {
    AT_CIPMUX!(Set cmd - mode);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn configure_tcp_server(mode: u8, port: Option<u16>) -> bool {
    match port {
        Some(port) => {
            AT_CIPSERVER!(Set cmd - mode, port);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
        None => {
            AT_CIPSERVER!(Set cmd - mode);
            WRITE_READ!(cmd, buf, _);
            TRUE_ON_OK!(buf)
        }
    }
}

pub async fn transfer_mode() -> Option<u8> {
    WRITE_READ!(AT_CIPMODE!(Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let mode = first_data_line(s, "+CIPMODE:")?;
        mode.parse().ok()
    } else {
        None
    }
}

pub async fn set_transfer_mode(mode: u8) -> bool {
    AT_CIPMODE!(Set cmd - mode);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn save_transparent_link(mode: u8, remote_ip: &str, remote_port: u16) -> bool {
    AT_SAVETRANSLINK!(Set cmd - mode, remote_ip, remote_port);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn save_transparent_link_with_type(
    mode: u8,
    remote_ip: &str,
    remote_port: u16,
    kind: &str,
) -> bool {
    AT_SAVETRANSLINK!(Set cmd - mode, remote_ip, remote_port, kind);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn save_transparent_link_with_extra(
    mode: u8,
    remote_ip: &str,
    remote_port: u16,
    kind: &str,
    extra: u16,
) -> bool {
    AT_SAVETRANSLINK!(Set cmd - mode, remote_ip, remote_port, kind, extra);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn tcp_server_timeout() -> Option<u16> {
    WRITE_READ!(AT_CIPSTO!(Query), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let time = first_data_line(s, "+CIPSTO:")?;
        time.parse().ok()
    } else {
        None
    }
}

pub async fn set_tcp_server_timeout(time_s: u16) -> bool {
    AT_CIPSTO!(Set cmd - time_s);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}

pub async fn ping(ip_or_domain: &str) -> Option<u32> {
    AT_PING!(Set cmd - ip_or_domain);
    WRITE_READ!(cmd, buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        for line in s.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix('+').and_then(first_number::<u32>) {
                return Some(v);
            }
        }
        None
    } else {
        None
    }
}

pub async fn update_through_network_stage() -> Option<u8> {
    WRITE_READ!(AT_CIUPDATE!(Execute), buf, l);
    if TRUE_ON_OK!(buf) {
        let s = unsafe { str::from_utf8_unchecked(&buf[..l]) };
        let mut out = None;
        for line in s.lines() {
            if let Some(v) = line
                .trim()
                .strip_prefix("+CIUPDATE:")
                .and_then(|x| x.trim().parse::<u8>().ok())
            {
                out = Some(v);
            }
        }
        out
    } else {
        None
    }
}

pub async fn set_ipd_info_mode(mode: u8) -> bool {
    AT_CIPDINFO!(Set cmd - mode);
    WRITE_READ!(cmd, buf, _);
    TRUE_ON_OK!(buf)
}
