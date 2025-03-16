use core::mem::transmute;

pub fn hsv2rgb(hue: f32, sat: f32, val: f32) -> (u8, u8, u8) {
    let c = val * sat;
    let v = (hue / 60.0) % 2.0 - 1.0;
    let v = if v < 0.0 { -v } else { v };
    let x = c * (1.0 - v);
    let m = val - c;
    let (r, g, b) = if hue < 60.0 {
        (c, x, 0.0)
    } else if hue < 120.0 {
        (x, c, 0.0)
    } else if hue < 180.0 {
        (0.0, c, x)
    } else if hue < 240.0 {
        (0.0, x, c)
    } else if hue < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    let r = ((r + m) * 255.0) as u8;
    let g = ((g + m) * 255.0) as u8;
    let b = ((b + m) * 255.0) as u8;

    (r, g, b)
}

pub fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r = ((r as u16) & 0b11111000) << 8;
    let g = ((g as u16) & 0b11111100) << 3;
    let b = (b as u16) >> 3;
    r | g | b
}

fn set_color(data: &mut [u8], color: (u8, u8, u8)) {
    data[0] = color.0;
    data[1] = color.1;
    data[2] = color.2;
}

fn get_color(data: &[u8]) -> (u8, u8, u8) {
    (data[0], data[1], data[2])
}

pub const RGB_PROFILE_OFF: u8 = 0;
pub const RGB_PROFILE_STATIC: u8 = 1;
pub const RGB_PROFILE_WAVE: u8 = 2;
pub const RGB_PROFILE_BREATHE: u8 = 3;
pub const RGB_PROFILE_RAINBOW_SOLID: u8 = 4;
pub const RGB_PROFILE_RAINBOW_WAVE: u8 = 5;

// colors are only 24 bits, the first 8 bits are unused
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RgbProfile {
    Off,
    Static {
        brightness: u8,
        color: (u8, u8, u8),
    },
    Wave {
        brightness: u8,
        speed_x: i8,
        speed_y: i8,
        color_count: u8,
        colors: [(u8, u8, u8); 4],
    },
    Breathe {
        brightness: u8,
        hold_time: u8,
        trans_time: u8,
        color_count: u8,
        colors: [(u8, u8, u8); 4],
    },
    RainbowSolid {
        brightness: u8,
        speed: i8,
        saturation: u8,
        value: u8,
    },
    RainbowWave {
        brightness: u8,
        speed: i8,
        speed_x: i8,
        speed_y: i8,
        saturation: u8,
        value: u8,
    },
}
impl RgbProfile {
    pub fn get_type(&self) -> u8 {
        match self {
            RgbProfile::Off => RGB_PROFILE_OFF,
            RgbProfile::Static {
                brightness: _,
                color: _,
            } => RGB_PROFILE_STATIC,
            RgbProfile::Wave {
                brightness: _,
                speed_x: _,
                speed_y: _,
                color_count: _,
                colors: _,
            } => RGB_PROFILE_WAVE,
            RgbProfile::Breathe {
                brightness: _,
                hold_time: _,
                trans_time: _,
                color_count: _,
                colors: _,
            } => RGB_PROFILE_BREATHE,
            RgbProfile::RainbowSolid {
                brightness: _,
                speed: _,
                saturation: _,
                value: _,
            } => RGB_PROFILE_RAINBOW_SOLID,
            RgbProfile::RainbowWave {
                brightness: _,
                speed: _,
                speed_x: _,
                speed_y: _,
                saturation: _,
                value: _,
            } => RGB_PROFILE_RAINBOW_WAVE,
        }
    }

    pub fn encode(self) -> [u8; 32] {
        match self {
            Self::Off => [0u8; 32],
            Self::Static { brightness, color } => {
                let mut data = [0u8; 32];
                data[0] = RGB_PROFILE_STATIC; // static type
                data[1] = brightness;
                set_color(&mut data[2..=4], color);

                data
            }
            Self::Wave {
                brightness,
                speed_x,
                speed_y,
                color_count,
                colors,
            } => {
                let mut data = [0u8; 32];
                data[0] = RGB_PROFILE_WAVE; // wave type
                data[1] = brightness;
                data[2] = unsafe { transmute(speed_x) };
                data[3] = unsafe { transmute(speed_y) };
                data[4] = color_count;
                set_color(&mut data[5..=7], colors[0]);
                set_color(&mut data[8..=10], colors[1]);
                set_color(&mut data[11..=13], colors[2]);
                set_color(&mut data[14..=16], colors[3]);

                data
            }
            Self::Breathe {
                brightness,
                hold_time,
                trans_time,
                color_count,
                colors,
            } => {
                let mut data = [0u8; 32];
                data[0] = RGB_PROFILE_BREATHE; // breathe type
                data[1] = brightness;
                data[2] = hold_time;
                data[3] = trans_time;
                data[4] = color_count;
                set_color(&mut data[5..=7], colors[0]);
                set_color(&mut data[8..=10], colors[1]);
                set_color(&mut data[11..=13], colors[2]);
                set_color(&mut data[14..=16], colors[3]);

                data
            }
            Self::RainbowSolid {
                brightness,
                speed,
                saturation,
                value,
            } => {
                let mut data = [0u8; 32];
                data[0] = RGB_PROFILE_RAINBOW_SOLID; // rainbow solid
                data[1] = brightness;
                data[2] = unsafe { transmute(speed) };
                data[3] = unsafe { transmute(saturation) };
                data[4] = unsafe { transmute(value) };

                data
            }
            Self::RainbowWave {
                brightness,
                speed,
                speed_x,
                speed_y,
                saturation,
                value,
            } => {
                let mut data = [0u8; 32];
                data[0] = RGB_PROFILE_RAINBOW_WAVE; // rainbow wave
                data[1] = brightness;
                data[2] = unsafe { transmute(speed) };
                data[3] = unsafe { transmute(speed_x) };
                data[4] = unsafe { transmute(speed_y) };
                data[5] = unsafe { transmute(saturation) };
                data[6] = unsafe { transmute(value) };

                data
            }
        }
    }

    pub fn decode(data: [u8; 32]) -> Self {
        let t = data[0];
        match t {
            RGB_PROFILE_STATIC => Self::Static {
                brightness: data[1],
                color: get_color(&data[2..=4]),
            },
            RGB_PROFILE_WAVE => Self::Wave {
                brightness: data[1],
                speed_x: unsafe { transmute(data[2]) },
                speed_y: unsafe { transmute(data[3]) },
                color_count: data[4],
                colors: [
                    get_color(&data[5..=7]),
                    get_color(&data[8..=10]),
                    get_color(&data[11..=13]),
                    get_color(&data[14..=16]),
                ],
            },
            RGB_PROFILE_BREATHE => Self::Breathe {
                brightness: data[1],
                hold_time: data[2],
                trans_time: data[3],
                color_count: data[4],
                colors: [
                    get_color(&data[5..=7]),
                    get_color(&data[8..=10]),
                    get_color(&data[11..=13]),
                    get_color(&data[14..=16]),
                ],
            },
            RGB_PROFILE_RAINBOW_SOLID => Self::RainbowSolid {
                brightness: data[1],
                speed: unsafe { transmute(data[2]) },
                saturation: data[3],
                value: data[4],
            },
            RGB_PROFILE_RAINBOW_WAVE => Self::RainbowWave {
                brightness: data[1],
                speed: unsafe { transmute(data[2]) },
                speed_x: unsafe { transmute(data[3]) },
                speed_y: unsafe { transmute(data[4]) },
                saturation: data[5],
                value: data[6],
            },
            _ => Self::Off,
        }
    }
}
