use crate::smallstr::SmallStr;

pub type ProfileName = SmallStr<{ 18 * 4 }>;

pub const SCREEN_PROFILE_OFF: u8 = 0;
pub const SCREEN_PROFILE_DISPLAY_KEYS: u8 = 1;
pub const SCREEN_PROFILE_DISPLAY_STATS: u8 = 2;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ScreenProfile {
    Off,
    DisplayKeys {
        brightness: u8,
        background_color: u16,
        profile_name_color: u16,
    },
    DisplayStats {
        brightness: u8,
        background_color: u16,
        profile_name_color: u16,
    },
}
impl ScreenProfile {
    pub fn get_type(&self) -> u8 {
        match self {
            Self::Off => SCREEN_PROFILE_OFF,
            Self::DisplayKeys {
                brightness: _,
                background_color: _,
                profile_name_color: _,
            } => SCREEN_PROFILE_DISPLAY_KEYS,
            Self::DisplayStats {
                brightness: _,
                background_color: _,
                profile_name_color: _,
            } => SCREEN_PROFILE_DISPLAY_STATS,
        }
    }

    pub fn brightness(&self) -> u8 {
        match self {
            Self::Off => 0,
            Self::DisplayKeys {
                brightness,
                background_color: _,
                profile_name_color: _,
            } => *brightness,
            Self::DisplayStats {
                brightness,
                background_color: _,
                profile_name_color: _,
            } => *brightness,
        }
    }

    pub fn background_color(&self) -> u16 {
        match self {
            Self::Off => 0,
            Self::DisplayKeys {
                brightness: _,
                background_color,
                profile_name_color: _,
            } => *background_color,
            Self::DisplayStats {
                brightness: _,
                background_color,
                profile_name_color: _,
            } => *background_color,
        }
    }

    pub fn profile_name_color(&self) -> u16 {
        match self {
            Self::Off => 0,
            Self::DisplayKeys {
                brightness: _,
                background_color: _,
                profile_name_color,
            } => *profile_name_color,
            Self::DisplayStats {
                brightness: _,
                background_color: _,
                profile_name_color,
            } => *profile_name_color,
        }
    }

    pub fn encode(self) -> [u8; 100] {
        let mut data = [0u8; 100];
        data[0] = self.get_type();
        data[1] = self.brightness();

        match self {
            Self::Off => {}
            Self::DisplayKeys {
                brightness: _,
                background_color,
                profile_name_color,
            } => {
                data[2] = (background_color >> 8) as u8;
                data[3] = (background_color & 0xFF) as u8;
                data[4] = (profile_name_color >> 8) as u8;
                data[5] = (profile_name_color & 0xFF) as u8;
            }
            Self::DisplayStats {
                brightness: _,
                background_color,
                profile_name_color,
            } => {
                data[2] = (background_color >> 8) as u8;
                data[3] = (background_color & 0xFF) as u8;
                data[4] = (profile_name_color >> 8) as u8;
                data[5] = (profile_name_color & 0xFF) as u8;
            }
        }

        data
    }

    pub fn decode(data: &[u8]) -> Self {
        match data[0] {
            SCREEN_PROFILE_OFF => Self::Off,
            SCREEN_PROFILE_DISPLAY_KEYS => Self::DisplayKeys {
                brightness: data[1],
                background_color: ((data[2] as u16) << 8) | (data[3] as u16),
                profile_name_color: ((data[4] as u16) << 8) | (data[5] as u16),
            },
            SCREEN_PROFILE_DISPLAY_STATS => Self::DisplayStats {
                brightness: data[1],
                background_color: ((data[2] as u16) << 8) | (data[3] as u16),
                profile_name_color: ((data[4] as u16) << 8) | (data[5] as u16),
            },
            _ => panic!(),
        }
    }

    pub const fn default_profile() -> Self {
        Self::DisplayKeys {
            brightness: 255,
            background_color: 0x4208,
            profile_name_color: 0xFFFF,
        }
    }
}
