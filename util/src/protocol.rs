// All the utilities for the communication protocol

pub const MAX_PACKET_SIZE: usize = 4095;

pub const CMD_GREET: u8 = b'\x05';

pub const CMD_GET_INPUT_KEYS: u8 = b'\x41';
pub const CMD_SET_KEYBOARD_INPUT: u8 = b'\x42';
pub const CMD_SET_MOUSE_INPUT: u8 = b'\x43';
pub const CMD_SET_GAMEPAD_INPUT: u8 = b'\x44';
pub const CMD_SET_RGB_MODE: u8 = b'\x45';
pub const CMD_SET_SCR_MODE: u8 = b'\x46';
pub const CMD_SET_SCR_ICON: u8 = b'\x47';
pub const CMD_SET_PROFILE_NAME: u8 = b'\x48';
pub const CMD_SET_SYSTEM_STATS: u8 = b'\x4A';

pub const CMD_IDENTIFY: u8 = b'\x07';
pub const CMD_UPDATE: u8 = b'\x0F';
pub const CMD_DISCONNECT: u8 = b'\x10';
pub const CMD_NEGATIVE_ACK: u8 = b'\x15';
pub const CMD_UNKNOWN: u8 = b'?';

pub const RSP_LINK_HEADER: u8 = b'\x01';
pub const RSP_LINK_DELIMITER: u8 = b'\x02';

pub const RSP_ACK: u8 = b'\x06';
pub const RSP_INPUT_HEADER: u8 = b'!';
// pub const RSP_RGB_HEADER: u8 = b'*';
pub const RSP_UNKNOWN: u8 = b'?';
pub const RSP_DISCONNECTED: u8 = b'\x04';

#[derive(PartialEq, Clone, Copy, Debug)]
#[repr(u8)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    Greeting,
    GetInputKeys,
    SetKeyboardInput,
    SetMouseInput,
    SetGamepadInput,
    SetRgbMode,
    SetScrIcon,
    SetScrMode,
    SetProfileName,
    SetSystemStats,
    Identify,
    Update,
    Disconnect,
    NegativeAck,
    Unknown,
}
impl Command {
    pub fn decode(w: u8) -> Self {
        match w {
            CMD_GREET => Self::Greeting,
            CMD_GET_INPUT_KEYS => Self::GetInputKeys,
            CMD_SET_KEYBOARD_INPUT => Self::SetKeyboardInput,
            CMD_SET_MOUSE_INPUT => Self::SetMouseInput,
            CMD_SET_GAMEPAD_INPUT => Self::SetGamepadInput,
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

fn decode_size_digit(w: u8) -> usize {
    // todo: switch to results instead of panics
    match w {
        b'0' => 0x0,
        b'1' => 0x1,
        b'2' => 0x2,
        b'3' => 0x3,
        b'4' => 0x4,
        b'5' => 0x5,
        b'6' => 0x6,
        b'7' => 0x7,
        b'8' => 0x8,
        b'9' => 0x9,
        b'a' => 0xA,
        b'A' => 0xA,
        b'b' => 0xB,
        b'B' => 0xB,
        b'c' => 0xC,
        b'C' => 0xC,
        b'd' => 0xD,
        b'D' => 0xD,
        b'e' => 0xE,
        b'E' => 0xE,
        b'f' => 0xF,
        b'F' => 0xF,
        _ => panic!("cannot decode digit {}", w),
    }
}

fn encode_size_digit(w: usize) -> u8 {
    // todo: switch to results instead of panics
    match w {
        0x0 => b'0',
        0x1 => b'1',
        0x2 => b'2',
        0x3 => b'3',
        0x4 => b'4',
        0x5 => b'5',
        0x6 => b'6',
        0x7 => b'7',
        0x8 => b'8',
        0x9 => b'9',
        0xA => b'A',
        0xB => b'B',
        0xC => b'C',
        0xD => b'D',
        0xE => b'E',
        0xF => b'F',
        _ => panic!("cannot encode digit {}", w),
    }
}

pub fn decode_packet_size(w1: u8, w2: u8, w3: u8) -> usize {
    // todo: switch to results instead of panics
    decode_size_digit(w1) * 16 * 16 + decode_size_digit(w2) * 16 + decode_size_digit(w3)
}

pub fn encode_packet_size(s: usize) -> [u8; 3] {
    if s > 16 * 16 * 16 - 1 {
        panic!("packet too big!")
    }

    let w3 = (s / (16 * 16)) % 16;
    let w2 = (s / 16) % 16;
    let w1 = s % 16;

    [
        encode_size_digit(w3),
        encode_size_digit(w2),
        encode_size_digit(w1),
    ]
}
