// Test = AT+<x>=?
// Query = AT+<x>?
// Set = AT+<x>=<...>
// Execute = AT+<x>

macro_rules! WRITE_READ {
    ($cmd:expr, $out:ident) => {
        TO_WIFI_CHANNEL.send($cmd).await;
        let (out, len) = FROM_WIFI_CHANNEL.receive().await;
        let $out = &out[..len];
    };
    ($cmd:expr, $out:ident, $len:ident) => {
        TO_WIFI_CHANNEL.send($cmd).await;
        let ($out, $len) = FROM_WIFI_CHANNEL.receive().await;
    };
    ($cmd:expr, $out:ident, _) => {
        TO_WIFI_CHANNEL.send($cmd).await;
        let ($out, _) = FROM_WIFI_CHANNEL.receive().await;
    };
}

macro_rules! TRUE_ON_OK {
    ($buf:expr) => {
        ($buf).windows(4).any(|w| w == b"OK\r\n") || ($buf).windows(2).any(|w| w == b"OK")
    };
}

macro_rules! DYNAMIC_BUILDER {
    ($val:ident, $fmt:literal, $($arg:tt)*) => {
        let mut tvec = Vec::<u8>::new();

        {
            let mut cmd = FORMAT_HOLDER.lock().await;
            cmd.clear();
            write!(cmd, concat!($fmt, "\r\n"), $($arg)*).unwrap();
            tvec.extend_from_slice(cmd.as_bytes());
        }

        let $val = WifiCmd::Dynamic(tvec);
    };
}

macro_rules! ASDF {
    (Test) => {
        compile_error!("AT doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT doesn't support the Set command")
    };
    (Execute) => {
        compile_error!("AT doesn't support the Execute command")
    };
}

/// - Execute
macro_rules! AT {
    (Test) => {
        compile_error!("AT doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT\r\n")
    };
}

/// - Execute
macro_rules! AT_RST {
    (Test) => {
        compile_error!("AT+RST doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+RST doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+RST doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT+RST\r\n")
    };
}

/// - Execute
///   - AT version info
///   - SDK version info
///   - Compile time
macro_rules! AT_GMR {
    (Test) => {
        compile_error!("AT+GMR doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+GMR doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+GMR doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT+GMR\r\n")
    };
}

/// - Set
///   - Time (in milliseconds)
macro_rules! AT_GSLP {
    (Test) => {
        compile_error!("AT+GSLP doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+GSLP doesn't support the Query command")
    };
    (Set $val:ident - $ms:expr) => {
        DYNAMIC_BUILDER!($val, "AT+GSLP={}", $ms)
    };
    (Execute) => {
        compile_error!("AT+GSLP doesn't support the Execute command")
    };
}

/// - Execute
macro_rules! ATE0 {
    (Test) => {
        compile_error!("ATE0 doesn't support the Test command")
    };
    (Query) => {
        compile_error!("ATE0 doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("ATE0 doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"ATE0\r\n")
    };
}

/// - Execute
macro_rules! ATE1 {
    (Test) => {
        compile_error!("ATE1 doesn't support the Test command")
    };
    (Query) => {
        compile_error!("ATE1 doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("ATE1 doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"ATE1\r\n")
    };
}

/// - Execute
macro_rules! AT_RESTORE {
    (Test) => {
        compile_error!("AT+RESTORE doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+RESTORE doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+RESTORE doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT+RESTORE\r\n")
    };
}

/// - (CUR|DEF) Set
///   - baudrate (between 110 and 115200*40)
///   - databits (between 5 and 8)
///   - stopbits (between 1 and 3)
///   - parity (between 0 and 2)
///   - flow control (between 0 and 3)
macro_rules! AT_UART {
    (Test) => {
        compile_error!("AT+UART doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+UART doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+UART requires CUR or DEF in front of Set")
    };
    (CUR Set $val:ident - $baud:expr, $databits:expr, $stopbits:expr, $parity:expr, $flowctl:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+UART_CUR={},{},{},{},{}",
            $baud,
            $databits,
            $stopbits,
            $parity,
            $flowctl
        );
    };
    (DEF Set $val:ident - $baud:expr, $databits:expr, $stopbits:expr, $parity:expr, $flowctl:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+UART_DEF={},{},{},{},{}",
            $baud,
            $databits,
            $stopbits,
            $parity,
            $flowctl
        );
    };
    (Execute) => {
        compile_error!("AT+UART doesn't support the Execute command")
    };
}

/// - Query
///   - Sleep mode (between 0 and 2)
/// - Set
///   - Sleep mode (between 0 and 2)
macro_rules! AT_SLEEP {
    (Test) => {
        compile_error!("AT+SLEEP doesn't support the Test command")
    };
    (Query) => {
        WifiCmd::Static(b"AT+SLEEP?\r\n")
    };
    (Set $val:ident - $mode:expr) => {
        DYNAMIC_BUILDER!($val, "AT+SLEEP={}", $mode)
    };
    (Execute) => {
        compile_error!("AT+SLEEP doesn't support the Execute command")
    };
}

/// - Set
///   - RF TX power (between 0 ~ 82)
macro_rules! AT_RFPOWER {
    (Test) => {
        compile_error!("AT+RFPOWER doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+RFPOWER doesn't support the Query command")
    };
    (Set $val:ident - $txpower:expr) => {
        DYNAMIC_BUILDER!($val, "AT+RFPOWER={}", $txpower)
    };
    (Execute) => {
        compile_error!("AT+RFPOWER doesn't support the Execute command")
    };
}

/// - Query
///   - RFVDD value
/// - Set
///   - RFVDD value
/// - Execute
macro_rules! AT_RFVDD {
    (Test) => {
        compile_error!("AT+RFVDD doesn't support the Test command")
    };
    (Query) => {
        WifiCmd::Static(b"AT+RFVDD?\r\n")
    };
    (Set $val:ident - $vdd33:expr) => {
        DYNAMIC_BUILDER!($val, "AT+RFVDD={}", $vdd33)
    };
    (Execute) => {
        WifiCmd::Static(b"AT+RFVDD\r\n")
    };
}

/// - (CUR|DEF) Test
///   - value scope of mode
/// - (CUR|DEF) Query
///   - mode (between 1 and 3)
/// - (CUR|DEF) Set
///   - mode (between 1 and 3)
macro_rules! AT_CWMODE {
    (Test) => {
        compile_error!("AT+CWMODE requires CUR or DEF in front of Test")
    };
    (CUR Test) => {
        WifiCmd::Static(b"AT+CWMODE_CUR=?\r\n")
    };
    (DEF Test) => {
        WifiCmd::Static(b"AT+CWMODE_DEF=?\r\n")
    };

    (Query) => {
        compile_error!("AT+CWMODE requires CUR or DEF in front of Query")
    };
    (CUR Query) => {
        WifiCmd::Static(b"AT+CWMODE_CUR?\r\n")
    };
    (DEF Query) => {
        WifiCmd::Static(b"AT+CWMODE_DEF?\r\n")
    };

    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CWMODE requires CUR or DEF in front of Set")
    };
    (CUR Set $val:ident - $mode:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWMODE_CUR={}", $mode)
    };
    (DEF Set $val:ident - $mode:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWMODE_DEF={}", $mode)
    };

    (Execute) => {
        compile_error!("AT+CWMODE doesn't support the Execute command")
    };
}

/// - (CUR|DEF) Query
///   - ssid
///   - bssid
///   - channel
///   - rssi
/// - (CUR|DEF) Set (returns error code on fail)
///   - ssid
///   - pwd
///   - [bssid]
macro_rules! AT_CWJAP {
    (Test) => {
        compile_error!("AT+CWJAP doesn't support the Test command")
    };

    (Query) => {
        compile_error!("AT+CWJAP requires CUR or DEF in front of Query")
    };
    (CUR Query) => {
        WifiCmd::Static(b"AT+CWJAP_CUR?\r\n")
    };
    (DEF Query) => {
        WifiCmd::Static(b"AT+CWJAP_DEF?\r\n")
    };

    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CWJAP requires CUR or DEF in front of Set")
    };
    (CUR Set $val:ident - $ssid:expr, $pwd:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWJAP_CUR=\"{}\",\"{}\"", $ssid, $pwd)
    };
    (CUR Set $val:ident - $ssid:expr, $pwd:expr, $bssid:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWJAP_CUR=\"{}\",\"{}\",\"{}\"",
            $ssid,
            $pwd,
            $bssid
        )
    };
    (DEF Set $val:ident - $ssid:expr, $pwd:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWJAP_DEF=\"{}\",\"{}\"", $ssid, $pwd)
    };
    (DEF Set $val:ident - $ssid:expr, $pwd:expr, $bssid:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWJAP_DEF=\"{}\",\"{}\",\"{}\"",
            $ssid,
            $pwd,
            $bssid
        )
    };

    (Execute) => {
        compile_error!("AT+CWJAP doesn't support the Execute command")
    };
}

/// - Set
///   - sort enable (0 or 1)
///   - mask
///     - bit 0 : ecn
///     - bit 1 : ssid
///     - bit 2 : rssi
///     - bit 3 : mac
///     - bit 4 : ch
///     - bit 5 : freq offset
///     - bit 6 : freq calibration
macro_rules! AT_CWLAPOPT {
    (Test) => {
        compile_error!("AT+CWLAPOPT doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CWLAPOPT doesn't support the Query command")
    };
    (Set $val:ident - $sort_enable:expr, $mask:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWLAPOPT={},{}", $sort_enable, $mask)
    };
    (Execute) => {
        compile_error!("AT+CWLAPOPT doesn't support the Execute command")
    };
}

/// - Set (return values found in execute, they both return the same)
///   - ssid
///   - [mac]
///   - [ch]
/// - Execute (see AT+CWLAPOPT for return value details)
///   - ecn
///   - ssid
///   - rssi
///   - mac
///   - ch
///   - freq offset
///   - freq calibration
macro_rules! AT_CWLAP {
    (Test) => {
        compile_error!("AT+CWLAP doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CWLAP doesn't support the Query command")
    };

    (Set $val:ident - $ssid:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWLAP=\"{}\"", $ssid)
    };
    (Set $val:ident - $ssid:expr, $mac:expr, $ch:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWLAP=\"{}\",\"{}\",{}", $ssid, $mac, $ch)
    };

    (Execute) => {
        WifiCmd::Static(b"AT+CWLAP\r\n")
    };
}

/// - Execute
macro_rules! AT_CWQAP {
    (Test) => {
        compile_error!("AT+CWQAP doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CWQAP doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CWQAP doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CWQAP\r\n")
    };
}

/// - (CUR|DEF) Query
///   - ssid
///   - pwd
///   - chl
///   - ecn
///   - max conn
///   - ssid hidden
/// - (CUR|DEF) Set
///   - ssid
///   - pwd
///   - chl
///   - ecn
///   - [max conn]
///   - [ssid hidden]
macro_rules! AT_CWSAP {
    (Test) => {
        compile_error!("AT doesn't support the Test command")
    };

    (Query) => {
        compile_error!("AT+CWSAP requires CUR or DEF in front of Query")
    };
    (CUR Query) => {
        WifiCmd::Static(b"AT+CWSAP_CUR?\r\n")
    };
    (DEF Query) => {
        WifiCmd::Static(b"AT+CWSAP_DEF?\r\n")
    };

    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CWSAP requires CUR or DEF in front of Set")
    };

    (CUR Set $val:ident - $ssid:expr, $pwd:expr, $chl:expr, $ecn:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWSAP_CUR={},{},{},{}", $ssid, $pwd, $chl, $ecn)
    };
    (CUR Set $val:ident - $ssid:expr, $pwd:expr, $chl:expr, $ecn:expr, $max_conn:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWSAP_CUR={},{},{},{},{}",
            $ssid,
            $pwd,
            $chl,
            $ecn,
            $max_conn
        )
    };
    (CUR Set $val:ident - $ssid:expr, $pwd:expr, $chl:expr, $ecn:expr, , $ssid_hidden:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWSAP_CUR={},{},{},{},,{}",
            $ssid,
            $pwd,
            $chl,
            $ecn,
            $ssid_hidden
        )
    };
    (CUR Set $val:ident - $ssid:expr, $pwd:expr, $chl:expr, $ecn:expr, $max_conn:expr, $ssid_hidden:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWSAP_CUR={},{},{},{},{},{}",
            $ssid,
            $pwd,
            $chl,
            $ecn,
            $max_conn,
            $ssid_hidden
        )
    };

    (DEF Set $val:ident - $ssid:expr, $pwd:expr, $chl:expr, $ecn:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWSAP_DEF={},{},{},{}", $ssid, $pwd, $chl, $ecn)
    };
    (DEF Set $val:ident - $ssid:expr, $pwd:expr, $chl:expr, $ecn:expr, $max_conn:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWSAP_DEF={},{},{},{},{}",
            $ssid,
            $pwd,
            $chl,
            $ecn,
            $max_conn
        )
    };
    (DEF Set $val:ident - $ssid:expr, $pwd:expr, $chl:expr, $ecn:expr, , $ssid_hidden:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWSAP_DEF={},{},{},{},,{}",
            $ssid,
            $pwd,
            $chl,
            $ecn,
            $ssid_hidden
        )
    };
    (DEF Set $val:ident - $ssid:expr, $pwd:expr, $chl:expr, $ecn:expr, $max_conn:expr, $ssid_hidden:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWSAP_DEF={},{},{},{},{},{}",
            $ssid,
            $pwd,
            $chl,
            $ecn,
            $max_conn,
            $ssid_hidden
        )
    };

    (Execute) => {
        compile_error!("AT+CWSAP doesn't support the Execute command")
    };
}

/// - Execute
///   - station IP list
macro_rules! AT_CWLIF {
    (Test) => {
        compile_error!("AT+CWLIF doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CWLIF doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CWLIF doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CWLIF\r\n")
    };
}

/// - (CUR|DEF) Query
///   - DHCP mode bits
/// - (CUR|DEF) Set
///   - mode
///   - enable
macro_rules! AT_CWDHCP {
    (Test) => {
        compile_error!("AT+CWDHCP doesn't support the Test command")
    };

    (Query) => {
        compile_error!("AT+CWDHCP requires CUR or DEF in front of Query")
    };
    (CUR Query) => {
        WifiCmd::Static(b"AT+CWDHCP_CUR?\r\n")
    };
    (DEF Query) => {
        WifiCmd::Static(b"AT+CWDHCP_DEF?\r\n")
    };

    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CWDHCP requires CUR or DEF in front of Set")
    };
    (CUR Set $val:ident - $mode:expr, $en:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWDHCP_CUR={},{}", $mode, $en)
    };
    (DEF Set $val:ident - $mode:expr, $en:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWDHCP_DEF={},{}", $mode, $en)
    };

    (Execute) => {
        compile_error!("AT+CWDHCP doesn't support the Execute command")
    };
}

/// - (CUR|DEF) Query
///   - lease time
///   - start IP
///   - end IP
/// - (CUR|DEF) Set
///   - disable: 0
///   - enable, lease time, start IP, end IP
macro_rules! AT_CWDHCPS {
    (Test) => {
        compile_error!("AT+CWDHCPS doesn't support the Test command")
    };

    (Query) => {
        compile_error!("AT+CWDHCPS requires CUR or DEF in front of Query")
    };
    (CUR Query) => {
        WifiCmd::Static(b"AT+CWDHCPS_CUR?\r\n")
    };
    (DEF Query) => {
        WifiCmd::Static(b"AT+CWDHCPS_DEF?\r\n")
    };

    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CWDHCPS requires CUR or DEF in front of Set")
    };
    (CUR Set $val:ident - 0) => {
        let $val = WifiCmd::Static(b"AT+CWDHCPS_CUR=0\r\n");
    };
    (CUR Set $val:ident - $enable:expr, $lease_time:expr, $start_ip:expr, $end_ip:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWDHCPS_CUR={},{},\"{}\",\"{}\"",
            $enable,
            $lease_time,
            $start_ip,
            $end_ip
        )
    };
    (DEF Set $val:ident - 0) => {
        let $val = WifiCmd::Static(b"AT+CWDHCPS_DEF=0\r\n");
    };
    (DEF Set $val:ident - $enable:expr, $lease_time:expr, $start_ip:expr, $end_ip:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWDHCPS_DEF={},{},\"{}\",\"{}\"",
            $enable,
            $lease_time,
            $start_ip,
            $end_ip
        )
    };

    (Execute) => {
        compile_error!("AT+CWDHCPS doesn't support the Execute command")
    };
}

/// - Set
///   - auto connect (0 or 1)
macro_rules! AT_CWAUTOCONN {
    (Test) => {
        compile_error!("AT+CWAUTOCONN doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CWAUTOCONN doesn't support the Query command")
    };
    (Set $val:ident - $enable:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWAUTOCONN={}", $enable)
    };
    (Execute) => {
        compile_error!("AT+CWAUTOCONN doesn't support the Execute command")
    };
}

/// - (CUR|DEF) Query
///   - station MAC
/// - (CUR|DEF) Set
///   - station MAC
macro_rules! AT_CIPSTAMAC {
    (Test) => {
        compile_error!("AT+CIPSTAMAC doesn't support the Test command")
    };

    (Query) => {
        compile_error!("AT+CIPSTAMAC requires CUR or DEF in front of Query")
    };
    (CUR Query) => {
        WifiCmd::Static(b"AT+CIPSTAMAC_CUR?\r\n")
    };
    (DEF Query) => {
        WifiCmd::Static(b"AT+CIPSTAMAC_DEF?\r\n")
    };

    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CIPSTAMAC requires CUR or DEF in front of Set")
    };
    (CUR Set $val:ident - $mac:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSTAMAC_CUR=\"{}\"", $mac)
    };
    (DEF Set $val:ident - $mac:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSTAMAC_DEF=\"{}\"", $mac)
    };

    (Execute) => {
        compile_error!("AT+CIPSTAMAC doesn't support the Execute command")
    };
}

/// - (CUR|DEF) Query
///   - softAP MAC
/// - (CUR|DEF) Set
///   - softAP MAC
macro_rules! AT_CIPAPMAC {
    (Test) => {
        compile_error!("AT+CIPAPMAC doesn't support the Test command")
    };

    (Query) => {
        compile_error!("AT+CIPAPMAC requires CUR or DEF in front of Query")
    };
    (CUR Query) => {
        WifiCmd::Static(b"AT+CIPAPMAC_CUR?\r\n")
    };
    (DEF Query) => {
        WifiCmd::Static(b"AT+CIPAPMAC_DEF?\r\n")
    };

    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CIPAPMAC requires CUR or DEF in front of Set")
    };
    (CUR Set $val:ident - $mac:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPAPMAC_CUR=\"{}\"", $mac)
    };
    (DEF Set $val:ident - $mac:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPAPMAC_DEF=\"{}\"", $mac)
    };

    (Execute) => {
        compile_error!("AT+CIPAPMAC doesn't support the Execute command")
    };
}

/// - (CUR|DEF) Query
///   - station IP
/// - (CUR|DEF) Set
///   - IP
///   - [gateway]
///   - [netmask]
macro_rules! AT_CIPSTA {
    (Test) => {
        compile_error!("AT+CIPSTA doesn't support the Test command")
    };

    (Query) => {
        compile_error!("AT+CIPSTA requires CUR or DEF in front of Query")
    };
    (CUR Query) => {
        WifiCmd::Static(b"AT+CIPSTA_CUR?\r\n")
    };
    (DEF Query) => {
        WifiCmd::Static(b"AT+CIPSTA_DEF?\r\n")
    };

    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CIPSTA requires CUR or DEF in front of Set")
    };
    (CUR Set $val:ident - $ip:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSTA_CUR=\"{}\"", $ip)
    };
    (CUR Set $val:ident - $ip:expr, $gateway:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSTA_CUR=\"{}\",\"{}\"", $ip, $gateway)
    };
    (CUR Set $val:ident - $ip:expr, $gateway:expr, $netmask:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSTA_CUR=\"{}\",\"{}\",\"{}\"",
            $ip,
            $gateway,
            $netmask
        )
    };
    (DEF Set $val:ident - $ip:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSTA_DEF=\"{}\"", $ip)
    };
    (DEF Set $val:ident - $ip:expr, $gateway:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSTA_DEF=\"{}\",\"{}\"", $ip, $gateway)
    };
    (DEF Set $val:ident - $ip:expr, $gateway:expr, $netmask:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSTA_DEF=\"{}\",\"{}\",\"{}\"",
            $ip,
            $gateway,
            $netmask
        )
    };

    (Execute) => {
        compile_error!("AT+CIPSTA doesn't support the Execute command")
    };
}

/// - (CUR|DEF) Query
///   - softAP IP
/// - (CUR|DEF) Set
///   - IP
///   - [gateway]
///   - [netmask]
macro_rules! AT_CIPAP {
    (Test) => {
        compile_error!("AT+CIPAP doesn't support the Test command")
    };

    (Query) => {
        compile_error!("AT+CIPAP requires CUR or DEF in front of Query")
    };
    (CUR Query) => {
        WifiCmd::Static(b"AT+CIPAP_CUR?\r\n")
    };
    (DEF Query) => {
        WifiCmd::Static(b"AT+CIPAP_DEF?\r\n")
    };

    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CIPAP requires CUR or DEF in front of Set")
    };
    (CUR Set $val:ident - $ip:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPAP_CUR=\"{}\"", $ip)
    };
    (CUR Set $val:ident - $ip:expr, $gateway:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPAP_CUR=\"{}\",\"{}\"", $ip, $gateway)
    };
    (CUR Set $val:ident - $ip:expr, $gateway:expr, $netmask:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPAP_CUR=\"{}\",\"{}\",\"{}\"",
            $ip,
            $gateway,
            $netmask
        )
    };
    (DEF Set $val:ident - $ip:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPAP_DEF=\"{}\"", $ip)
    };
    (DEF Set $val:ident - $ip:expr, $gateway:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPAP_DEF=\"{}\",\"{}\"", $ip, $gateway)
    };
    (DEF Set $val:ident - $ip:expr, $gateway:expr, $netmask:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPAP_DEF=\"{}\",\"{}\",\"{}\"",
            $ip,
            $gateway,
            $netmask
        )
    };

    (Execute) => {
        compile_error!("AT+CIPAP doesn't support the Execute command")
    };
}

/// - Execute
///   - SmartConfig with both ESP-Touch + AirKiss
/// - Set
///   - SmartConfig type
macro_rules! AT_CWSTARTSMART {
    (Test) => {
        compile_error!("AT+CWSTARTSMART doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CWSTARTSMART doesn't support the Query command")
    };
    (Set $val:ident - $kind:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CWSTARTSMART={}", $kind)
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CWSTARTSMART\r\n")
    };
}

/// - Execute
///   - Stop SmartConfig
macro_rules! AT_CWSTOPSMART {
    (Test) => {
        compile_error!("AT+CWSTOPSMART doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CWSTOPSMART doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CWSTOPSMART doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CWSTOPSMART\r\n")
    };
}

/// - Set
///   - WeChat number
///   - device type
///   - send interval (s)
macro_rules! AT_CWSTARTDISCOVER {
    (Test) => {
        compile_error!("AT+CWSTARTDISCOVER doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CWSTARTDISCOVER doesn't support the Query command")
    };
    (Set $val:ident - $wechat_number:expr, $dev_type:expr, $time:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CWSTARTDISCOVER=\"{}\",\"{}\",{}",
            $wechat_number,
            $dev_type,
            $time
        )
    };
    (Execute) => {
        compile_error!("AT+CWSTARTDISCOVER doesn't support the Execute command")
    };
}

/// - Execute
///   - Stop WeChat discover mode
macro_rules! AT_CWSTOPDISCOVER {
    (Test) => {
        compile_error!("AT+CWSTOPDISCOVER doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CWSTOPDISCOVER doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CWSTOPDISCOVER doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CWSTOPDISCOVER\r\n")
    };
}

/// - Set
///   - 0: stop WPS
///   - 1: start WPS
macro_rules! AT_WPS {
    (Test) => {
        compile_error!("AT+WPS doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+WPS doesn't support the Query command")
    };
    (Set $val:ident - $enable:expr) => {
        DYNAMIC_BUILDER!($val, "AT+WPS={}", $enable)
    };
    (Execute) => {
        compile_error!("AT+WPS doesn't support the Execute command")
    };
}

/// - Set
///   - enable
///   - hostname
///   - server name
///   - server port
macro_rules! AT_MDNS {
    (Test) => {
        compile_error!("AT+MDNS doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+MDNS doesn't support the Query command")
    };
    (Set $val:ident - $enable:expr, $hostname:expr, $server_name:expr, $server_port:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+MDNS={},\"{}\",\"{}\",{}",
            $enable,
            $hostname,
            $server_name,
            $server_port
        )
    };
    (Execute) => {
        compile_error!("AT+MDNS doesn't support the Execute command")
    };
}

/// - Execute
///   - Check network connection status
macro_rules! AT_CIPSTATUS {
    (Test) => {
        compile_error!("AT+CIPSTATUS doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPSTATUS doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CIPSTATUS doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CIPSTATUS\r\n")
    };
}

/// - Set
///   - domain name
macro_rules! AT_CIPDOMAIN {
    (Test) => {
        compile_error!("AT+CIPDOMAIN doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPDOMAIN doesn't support the Query command")
    };
    (Set $val:ident - $domain:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPDOMAIN=\"{}\"", $domain)
    };
    (Execute) => {
        compile_error!("AT+CIPDOMAIN doesn't support the Execute command")
    };
}

/// - Set
///   - Single: type, remote IP/domain, remote port [, keep alive]
///   - Multi: link ID, type, remote IP/domain, remote port [, keep alive]
///   - UDP: type, remote IP, remote port, local port, mode
macro_rules! AT_CIPSTART {
    (Test) => {
        compile_error!("AT+CIPSTART doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPSTART doesn't support the Query command")
    };

    (Set $val:ident - $kind:expr, $remote_ip:expr, $remote_port:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSTART=\"{}\",\"{}\",{}",
            $kind,
            $remote_ip,
            $remote_port
        )
    };
    (Set $val:ident - $kind:expr, $remote_ip:expr, $remote_port:expr, $keep_alive:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSTART=\"{}\",\"{}\",{},{}",
            $kind,
            $remote_ip,
            $remote_port,
            $keep_alive
        )
    };
    (Set $val:ident - $kind:expr, $remote_ip:expr, $remote_port:expr, $local_port:expr, $mode:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSTART=\"{}\",\"{}\",{},{},{}",
            $kind,
            $remote_ip,
            $remote_port,
            $local_port,
            $mode
        )
    };

    (Set $val:ident - $link_id:expr, $kind:expr, $remote_ip:expr, $remote_port:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSTART={},\"{}\",\"{}\",{}",
            $link_id,
            $kind,
            $remote_ip,
            $remote_port
        )
    };
    (Set $val:ident - $link_id:expr, $kind:expr, $remote_ip:expr, $remote_port:expr, $keep_alive:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSTART={},\"{}\",\"{}\",{},{}",
            $link_id,
            $kind,
            $remote_ip,
            $remote_port,
            $keep_alive
        )
    };
    (Set $val:ident - $link_id:expr, $kind:expr, $remote_ip:expr, $remote_port:expr, $local_port:expr, $mode:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSTART={},\"{}\",\"{}\",{},{},{}",
            $link_id,
            $kind,
            $remote_ip,
            $remote_port,
            $local_port,
            $mode
        )
    };

    (Execute) => {
        compile_error!("AT+CIPSTART doesn't support the Execute command")
    };
}

/// - Set
///   - SSL buffer size (2048..4096)
macro_rules! AT_CIPSSLSIZE {
    (Test) => {
        compile_error!("AT+CIPSSLSIZE doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPSSLSIZE doesn't support the Query command")
    };
    (Set $val:ident - $size:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSSLSIZE={}", $size)
    };
    (Execute) => {
        compile_error!("AT+CIPSSLSIZE doesn't support the Execute command")
    };
}

/// - Execute
///   - Enter transparent transmission (single connection only)
/// - Set
///   - length variants for single/multi/UDP
macro_rules! AT_CIPSEND {
    (Test) => {
        compile_error!("AT+CIPSEND doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPSEND doesn't support the Query command")
    };

    (Set $val:ident - $length:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSEND={}", $length)
    };
    (Set $val:ident - $link_id:expr, $length:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSEND={},{}", $link_id, $length)
    };
    (Set $val:ident - $length:expr, $remote_ip:expr, $remote_port:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSEND={},\"{}\",{}",
            $length,
            $remote_ip,
            $remote_port
        )
    };
    (Set $val:ident - $link_id:expr, $length:expr, $remote_ip:expr, $remote_port:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSEND={},{},\"{}\",{}",
            $link_id,
            $length,
            $remote_ip,
            $remote_port
        )
    };

    (Execute) => {
        WifiCmd::Static(b"AT+CIPSEND\r\n")
    };
}

/// - Set
///   - length variants for single/multi/UDP
macro_rules! AT_CIPSENDEX {
    (Test) => {
        compile_error!("AT+CIPSENDEX doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPSENDEX doesn't support the Query command")
    };

    (Set $val:ident - $length:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSENDEX={}", $length)
    };
    (Set $val:ident - $link_id:expr, $length:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSENDEX={},{}", $link_id, $length)
    };
    (Set $val:ident - $length:expr, $remote_ip:expr, $remote_port:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSENDEX={},\"{}\",{}",
            $length,
            $remote_ip,
            $remote_port
        )
    };
    (Set $val:ident - $link_id:expr, $length:expr, $remote_ip:expr, $remote_port:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+CIPSENDEX={},{},\"{}\",{}",
            $link_id,
            $length,
            $remote_ip,
            $remote_port
        )
    };

    (Execute) => {
        compile_error!("AT+CIPSENDEX doesn't support the Execute command")
    };
}

/// - Set
///   - single: length
///   - multi: link ID, length
macro_rules! AT_CIPSENDBUF {
    (Test) => {
        compile_error!("AT+CIPSENDBUF doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPSENDBUF doesn't support the Query command")
    };
    (Set $val:ident - $length:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSENDBUF={}", $length)
    };
    (Set $val:ident - $link_id:expr, $length:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSENDBUF={},{}", $link_id, $length)
    };
    (Execute) => {
        compile_error!("AT+CIPSENDBUF doesn't support the Execute command")
    };
}

/// - Execute
///   - single connection status
/// - Set
///   - multi connection status for a link ID
macro_rules! AT_CIPBUFSTATUS {
    (Test) => {
        compile_error!("AT+CIPBUFSTATUS doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPBUFSTATUS doesn't support the Query command")
    };
    (Set $val:ident - $link_id:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPBUFSTATUS={}", $link_id)
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CIPBUFSTATUS\r\n")
    };
}

/// - Set
///   - single: segment ID
///   - multi: link ID, segment ID
macro_rules! AT_CIPCHECKSEQ {
    (Test) => {
        compile_error!("AT+CIPCHECKSEQ doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPCHECKSEQ doesn't support the Query command")
    };
    (Set $val:ident - $segment_id:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPCHECKSEQ={}", $segment_id)
    };
    (Set $val:ident - $link_id:expr, $segment_id:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPCHECKSEQ={},{}", $link_id, $segment_id)
    };
    (Execute) => {
        compile_error!("AT+CIPCHECKSEQ doesn't support the Execute command")
    };
}

/// - Execute
///   - single connection segment ID reset
/// - Set
///   - multi connection segment ID reset
macro_rules! AT_CIPBUFRESET {
    (Test) => {
        compile_error!("AT+CIPBUFRESET doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPBUFRESET doesn't support the Query command")
    };
    (Set $val:ident - $link_id:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPBUFRESET={}", $link_id)
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CIPBUFRESET\r\n")
    };
}

/// - Execute
///   - close single connection
/// - Set
///   - close by link ID
macro_rules! AT_CIPCLOSE {
    (Test) => {
        compile_error!("AT+CIPCLOSE doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPCLOSE doesn't support the Query command")
    };
    (Set $val:ident - $link_id:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPCLOSE={}", $link_id)
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CIPCLOSE\r\n")
    };
}

/// - Execute
///   - get local IP(s)
macro_rules! AT_CIFSR {
    (Test) => {
        compile_error!("AT+CIFSR doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIFSR doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CIFSR doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CIFSR\r\n")
    };
}

/// - Query
///   - current connection mode
/// - Set
///   - 0 single
///   - 1 multi
macro_rules! AT_CIPMUX {
    (Test) => {
        compile_error!("AT+CIPMUX doesn't support the Test command")
    };
    (Query) => {
        WifiCmd::Static(b"AT+CIPMUX?\r\n")
    };
    (Set $val:ident - $mode:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPMUX={}", $mode)
    };
    (Execute) => {
        compile_error!("AT+CIPMUX doesn't support the Execute command")
    };
}

/// - Set
///   - mode
///   - [port]
macro_rules! AT_CIPSERVER {
    (Test) => {
        compile_error!("AT+CIPSERVER doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPSERVER doesn't support the Query command")
    };
    (Set $val:ident - $mode:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSERVER={}", $mode)
    };
    (Set $val:ident - $mode:expr, $port:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSERVER={},{}", $mode, $port)
    };
    (Execute) => {
        compile_error!("AT+CIPSERVER doesn't support the Execute command")
    };
}

/// - Query
///   - transfer mode
/// - Set
///   - transfer mode
macro_rules! AT_CIPMODE {
    (Test) => {
        compile_error!("AT+CIPMODE doesn't support the Test command")
    };
    (Query) => {
        WifiCmd::Static(b"AT+CIPMODE?\r\n")
    };
    (Set $val:ident - $mode:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPMODE={}", $mode)
    };
    (Execute) => {
        compile_error!("AT+CIPMODE doesn't support the Execute command")
    };
}

/// - Set
///   - mode, remote IP/domain, remote port [, type] [, keep alive]
///   - mode, remote IP, remote port, type, local port (UDP)
macro_rules! AT_SAVETRANSLINK {
    (Test) => {
        compile_error!("AT+SAVETRANSLINK doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+SAVETRANSLINK doesn't support the Query command")
    };
    (Set $val:ident - $mode:expr, $remote_ip:expr, $remote_port:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+SAVETRANSLINK={},\"{}\",{}",
            $mode,
            $remote_ip,
            $remote_port
        )
    };
    (Set $val:ident - $mode:expr, $remote_ip:expr, $remote_port:expr, $kind:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+SAVETRANSLINK={},\"{}\",{},\"{}\"",
            $mode,
            $remote_ip,
            $remote_port,
            $kind
        )
    };
    (Set $val:ident - $mode:expr, $remote_ip:expr, $remote_port:expr, $kind:expr, $extra:expr) => {
        DYNAMIC_BUILDER!(
            $val,
            "AT+SAVETRANSLINK={},\"{}\",{},\"{}\",{}",
            $mode,
            $remote_ip,
            $remote_port,
            $kind,
            $extra
        )
    };
    (Execute) => {
        compile_error!("AT+SAVETRANSLINK doesn't support the Execute command")
    };
}

/// - Query
///   - timeout
/// - Set
///   - timeout (s)
macro_rules! AT_CIPSTO {
    (Test) => {
        compile_error!("AT+CIPSTO doesn't support the Test command")
    };
    (Query) => {
        WifiCmd::Static(b"AT+CIPSTO?\r\n")
    };
    (Set $val:ident - $time:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPSTO={}", $time)
    };
    (Execute) => {
        compile_error!("AT+CIPSTO doesn't support the Execute command")
    };
}

/// - Set
///   - host IP/domain
macro_rules! AT_PING {
    (Test) => {
        compile_error!("AT+PING doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+PING doesn't support the Query command")
    };
    (Set $val:ident - $ip_or_domain:expr) => {
        DYNAMIC_BUILDER!($val, "AT+PING=\"{}\"", $ip_or_domain)
    };
    (Execute) => {
        compile_error!("AT+PING doesn't support the Execute command")
    };
}

/// - Execute
///   - update firmware through network
macro_rules! AT_CIUPDATE {
    (Test) => {
        compile_error!("AT+CIUPDATE doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIUPDATE doesn't support the Query command")
    };
    (Set $val:ident - $first:expr $(, $_:expr)*) => {
        compile_error!("AT+CIUPDATE doesn't support the Set command")
    };
    (Execute) => {
        WifiCmd::Static(b"AT+CIUPDATE\r\n")
    };
}

/// - Set
///   - show remote IP/port in +IPD
macro_rules! AT_CIPDINFO {
    (Test) => {
        compile_error!("AT+CIPDINFO doesn't support the Test command")
    };
    (Query) => {
        compile_error!("AT+CIPDINFO doesn't support the Query command")
    };
    (Set $val:ident - $mode:expr) => {
        DYNAMIC_BUILDER!($val, "AT+CIPDINFO={}", $mode)
    };
    (Execute) => {
        compile_error!("AT+CIPDINFO doesn't support the Execute command")
    };
}
