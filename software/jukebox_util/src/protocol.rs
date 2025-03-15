// All the utilities for the communication protocol

pub const CMD_GREET: u8 = b'\x05';
pub const CMD_GET_INPUT_KEYS: u8 = b'\x30';
pub const CMD_GET_RGB: u8 = b'\x33';
pub const CMD_SET_RGB: u8 = b'\x34';
pub const CMD_GET_SCR: u8 = b'\x35';
pub const CMD_SET_SCR: u8 = b'\x36';
pub const CMD_IDENTIFY: u8 = b'\x37';
pub const CMD_UPDATE: u8 = b'\x38';
pub const CMD_DISCONNECT: u8 = b'\x39';
pub const CMD_NEGATIVE_ACK: u8 = b'\x15';
pub const CMD_UNKNOWN: u8 = b'?';

pub const CMD_DEVICE: u8 = b'U';
pub const CMD_END: &[u8] = b"\r\n";

pub const RSP_LINK_HEADER: u8 = b'L';
pub const RSP_LINK_DELIMITER: u8 = b',';

pub const RSP_ACK: u8 = b'A';
pub const RSP_INPUT_HEADER: u8 = b'I';
pub const RSP_RGB_HEADER: u8 = b'C';
pub const RSP_UNKNOWN: u8 = b'?';
pub const RSP_DISCONNECTED: u8 = b'\x04';

pub const RSP_END: &[u8] = b"\r\n\r\n";

#[derive(PartialEq, Clone, Copy, Debug)]
#[repr(u8)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    Greeting,
    GetInputKeys,
    GetRGB,
    SetRGB,
    GetScr,
    SetScr,
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
            CMD_GET_RGB => Self::GetRGB,
            CMD_SET_RGB => Self::SetRGB,
            CMD_GET_SCR => Self::GetScr,
            CMD_SET_SCR => Self::SetScr,
            CMD_IDENTIFY => Self::Identify,
            CMD_UPDATE => Self::Update,
            CMD_DISCONNECT => Self::Disconnect,
            CMD_NEGATIVE_ACK => Self::NegativeAck,
            _ => Self::Unknown,
        }
    }
}
