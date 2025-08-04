use bincode::{decode_from_slice, encode_into_slice, Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::smallstr::SmallStr;

pub const PROFILE_NAME_CODE_POINT_LEN: usize = 18;
pub const PROFILE_NAME_CHAR_LEN: usize = PROFILE_NAME_CODE_POINT_LEN * 4;
pub type ProfileName = SmallStr<{ PROFILE_NAME_CHAR_LEN + 1 }>;

pub const SCREEN_PROFILE_SIZE: usize = 256;

pub const SCREEN_PROFILE_OFF: u8 = 0;
pub const SCREEN_PROFILE_DISPLAY_KEYS: u8 = 1;
pub const SCREEN_PROFILE_DISPLAY_STATS: u8 = 2;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ScreenProfile {
    Off,
    DisplayKeys {
        brightness: u8,
        background_color: u16,
        text_color: u16,
        show_profile_name: bool,
    },
    DisplayStats {
        brightness: u8,
        background_color: u16,
        text_color: u16,
        show_profile_name: bool,
    },
}
impl ScreenProfile {
    pub fn get_type(&self) -> u8 {
        match self {
            Self::Off => SCREEN_PROFILE_OFF,
            Self::DisplayKeys {
                brightness: _,
                background_color: _,
                text_color: _,
                show_profile_name: _,
            } => SCREEN_PROFILE_DISPLAY_KEYS,
            Self::DisplayStats {
                brightness: _,
                background_color: _,
                text_color: _,
                show_profile_name: _,
            } => SCREEN_PROFILE_DISPLAY_STATS,
        }
    }

    pub fn brightness(&self) -> u8 {
        match self {
            Self::Off => 0,
            Self::DisplayKeys {
                brightness,
                background_color: _,
                text_color: _,
                show_profile_name: _,
            } => *brightness,
            Self::DisplayStats {
                brightness,
                background_color: _,
                text_color: _,
                show_profile_name: _,
            } => *brightness,
        }
    }

    pub fn background_color(&self) -> u16 {
        match self {
            Self::Off => 0,
            Self::DisplayKeys {
                brightness: _,
                background_color,
                text_color: _,
                show_profile_name: _,
            } => *background_color,
            Self::DisplayStats {
                brightness: _,
                background_color,
                text_color: _,
                show_profile_name: _,
            } => *background_color,
        }
    }

    pub fn text_color(&self) -> u16 {
        match self {
            Self::Off => 0,
            Self::DisplayKeys {
                brightness: _,
                background_color: _,
                text_color,
                show_profile_name: _,
            } => *text_color,
            Self::DisplayStats {
                brightness: _,
                background_color: _,
                text_color,
                show_profile_name: _,
            } => *text_color,
        }
    }

    pub fn encode(self) -> [u8; SCREEN_PROFILE_SIZE] {
        let mut data = [0u8; SCREEN_PROFILE_SIZE];
        let _ = encode_into_slice(self, &mut data, bincode::config::standard()).unwrap();

        data
    }

    pub fn decode(data: &[u8]) -> Self {
        decode_from_slice(data, bincode::config::standard())
            .unwrap()
            .0
    }

    pub const fn default_profile() -> Self {
        Self::default_display_keys()
    }

    pub const fn default_display_keys() -> Self {
        Self::DisplayKeys {
            brightness: 100,
            background_color: 0x01B3,
            text_color: 0xFFFF,
            show_profile_name: true,
        }
    }

    pub const fn default_display_stats() -> Self {
        Self::DisplayStats {
            brightness: 255,
            background_color: 0x01B3,
            text_color: 0xFFFF,
            show_profile_name: true,
        }
    }
}
