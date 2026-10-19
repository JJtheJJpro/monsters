#![no_std]

pub const NET_OFF: u8 = 0b10000000;
pub const NET_ON: u8 = 0b10000001;
pub const SEND_DATA_OFF: u8 = 0b10000010;
pub const SEND_DATA_ON: u8 = 0b10000011;
pub const TEST_END: u8 = 0b10000100;
pub const TEST_BEGIN: u8 = 0b10000101;
pub const TEST_ERROR: u8 = 0b10000110;
pub const TEST_IN_PROGRESS: u8 = 0b10000111;
pub const UNAUTHORIZED: u8 = 0b10001000;
pub const AUTHORIZED: u8 = 0b10001001;
pub const KEY_GUIDE_OFF: u8 = 0b10001010;
pub const KEY_GUIDE_ON: u8 = 0b10001011;
pub const DISCONNECTED: u8 = 0b10001100;
pub const CONNECTING: u8 = 0b10001101;
pub const CONNECTED: u8 = 0b10001110;
pub const ERROR: u8 = 0b10001111;
pub const INPUT_KEYBOARD: u8 = 0b10010000;
pub const INPUT_GUI: u8 = 0b10010001;

pub const RESET: u8 = 0b11111111;
