// 64 possible relay locations, we only need 12*3=36, so we're fine.
// We could fit 9 more monsters if we wanted to.

#[macro_export]
macro_rules! MONST {
    (ON $num:literal) => {
        0b01000000 | $num
    };
    (OFF $num:literal) => {
        $num
    };
}

pub const USB_TAKEOVER: u8 = 0b10000000;
pub const NET_TAKEOVER: u8 = 0b10000001;
pub const SEND_DATA_OFF: u8 = 0b10000010;
pub const SEND_DATA_ON: u8 = 0b10000011;
pub const TESTING_END: u8 = 0b10000100;
pub const TESTING_BEGIN: u8 = 0b10000101;
//0b10000110;
pub const TEST_IN_PROGRESS: u8 = 0b10000111;
pub const UNAUTHORIZED: u8 = 0b10001000;
pub const AUTHORIZED: u8 = 0b10001001;

pub const RESET: u8 = 0b11111111;
