// All the utilities for the communication protocol

use serde::{Deserialize, Serialize};

pub const MAX_PACKET_SIZE: usize = 4095;

const CMD_GREET: u8 = b'\x05';
const CMD_GET_INPUT_KEYS: u8 = b'\x41';
const CMD_SET_INPUT_EVENT: u8 = b'\x42';
const CMD_SET_RGB_MODE: u8 = b'\x45';
const CMD_SET_SCR_MODE: u8 = b'\x46';
const CMD_SET_SCR_ICON: u8 = b'\x47';
const CMD_SET_PROFILE_NAME: u8 = b'\x48';
const CMD_SET_SYSTEM_STATS: u8 = b'\x4A';
const CMD_SET_DEFAULT_INPUT_EVENT: u8 = b'\x52';
const CMD_SET_DEFAULT_RGB_MODE: u8 = b'\x55';
const CMD_SET_DEFAULT_SCR_MODE: u8 = b'\x56';
const CMD_IDENTIFY: u8 = b'\x07';
const CMD_UPDATE: u8 = b'\x0F';
const CMD_DISCONNECT: u8 = b'\x10';
const CMD_NEGATIVE_ACK: u8 = b'\x15';
const CMD_UNKNOWN: u8 = b'?';

pub const RSP_LINK_HEADER: u8 = b'\x01';
pub const RSP_LINK_DELIMITER: u8 = b'\x02';
pub const RSP_ACK: u8 = b'\x06';
pub const RSP_INPUT_HEADER: u8 = b'!';
pub const RSP_UNKNOWN: u8 = b'?';
pub const RSP_DISCONNECTED: u8 = b'\x04';
pub const RSP_FULL_ACK: &[u8] = &[b'0', b'0', b'1', RSP_ACK];
pub const RSP_FULL_UNKNOWN: &[u8] = &[b'0', b'0', b'1', RSP_UNKNOWN];
pub const RSP_FULL_DISCONNECTED: &[u8] = &[b'0', b'0', b'1', RSP_DISCONNECTED];

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    Greeting = CMD_GREET,

    GetInputKeys = CMD_GET_INPUT_KEYS,

    SetInputEvent = CMD_SET_INPUT_EVENT,
    SetRgbMode = CMD_SET_RGB_MODE,
    SetScrMode = CMD_SET_SCR_MODE,
    SetScrIcon = CMD_SET_SCR_ICON,
    SetProfileName = CMD_SET_PROFILE_NAME,
    SetSystemStats = CMD_SET_SYSTEM_STATS,

    SetDefaultInputEvent = CMD_SET_DEFAULT_INPUT_EVENT,
    SetDefaultRgbMode = CMD_SET_DEFAULT_RGB_MODE,
    SetDefaultScreenMode = CMD_SET_DEFAULT_SCR_MODE,

    Identify = CMD_IDENTIFY,
    Update = CMD_UPDATE,
    Disconnect = CMD_DISCONNECT,
    NegativeAck = CMD_NEGATIVE_ACK,
    Unknown = CMD_UNKNOWN,
}
impl From<u8> for Command {
    fn from(w: u8) -> Self {
        match w {
            CMD_GREET => Self::Greeting,
            CMD_GET_INPUT_KEYS => Self::GetInputKeys,
            CMD_SET_INPUT_EVENT => Self::SetInputEvent,
            CMD_SET_RGB_MODE => Self::SetRgbMode,
            CMD_SET_SCR_ICON => Self::SetScrIcon,
            CMD_SET_SCR_MODE => Self::SetScrMode,
            CMD_SET_PROFILE_NAME => Self::SetProfileName,
            CMD_SET_SYSTEM_STATS => Self::SetSystemStats,
            CMD_IDENTIFY => Self::Identify,
            CMD_UPDATE => Self::Update,
            CMD_DISCONNECT => Self::Disconnect,
            CMD_NEGATIVE_ACK => Self::NegativeAck,
            _ => Self::Unknown,
        }
    }
}
impl Into<u8> for Command {
    fn into(self) -> u8 {
        self as u8
    }
}

fn decode_size_digit(w: u8) -> Result<usize, ()> {
    match w {
        b'0' => Ok(0x0),
        b'1' => Ok(0x1),
        b'2' => Ok(0x2),
        b'3' => Ok(0x3),
        b'4' => Ok(0x4),
        b'5' => Ok(0x5),
        b'6' => Ok(0x6),
        b'7' => Ok(0x7),
        b'8' => Ok(0x8),
        b'9' => Ok(0x9),
        b'a' => Ok(0xA),
        b'A' => Ok(0xA),
        b'b' => Ok(0xB),
        b'B' => Ok(0xB),
        b'c' => Ok(0xC),
        b'C' => Ok(0xC),
        b'd' => Ok(0xD),
        b'D' => Ok(0xD),
        b'e' => Ok(0xE),
        b'E' => Ok(0xE),
        b'f' => Ok(0xF),
        b'F' => Ok(0xF),
        _ => Err(()), // panic!("cannot decode digit {}", w),
    }
}

fn encode_size_digit(w: usize) -> Result<u8, ()> {
    match w {
        0x0 => Ok(b'0'),
        0x1 => Ok(b'1'),
        0x2 => Ok(b'2'),
        0x3 => Ok(b'3'),
        0x4 => Ok(b'4'),
        0x5 => Ok(b'5'),
        0x6 => Ok(b'6'),
        0x7 => Ok(b'7'),
        0x8 => Ok(b'8'),
        0x9 => Ok(b'9'),
        0xA => Ok(b'A'),
        0xB => Ok(b'B'),
        0xC => Ok(b'C'),
        0xD => Ok(b'D'),
        0xE => Ok(b'E'),
        0xF => Ok(b'F'),
        _ => Err(()), // panic!("cannot encode digit {}", w),
    }
}

pub fn decode_packet_size(w1: u8, w2: u8, w3: u8) -> Result<usize, ()> {
    Ok(decode_size_digit(w1)? * 16 * 16 + decode_size_digit(w2)? * 16 + decode_size_digit(w3)?)
}

pub fn encode_packet_size(s: usize) -> Result<[u8; 3], ()> {
    if s > 16 * 16 * 16 - 1 {
        panic!("packet too big!")
    }

    let w3 = (s / (16 * 16)) % 16;
    let w2 = (s / 16) % 16;
    let w1 = s % 16;

    Ok([
        encode_size_digit(w3)?,
        encode_size_digit(w2)?,
        encode_size_digit(w1)?,
    ])
}
