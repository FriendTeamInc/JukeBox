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

fn put_color(data: &mut [u8], color: (u8, u8, u8)) {
    data[0] = color.0;
    data[1] = color.1;
    data[2] = color.2;
}

fn get_color(data: &[u8]) -> (u8, u8, u8) {
    (data[0], data[1], data[2])
}

const _COLOR_OFF: u8 = 0;
const COLOR_STATIC: u8 = 1;
const COLOR_WAVE: u8 = 2;
const COLOR_BREATHE: u8 = 3;
const COLOR_RAINBOW_SOLID: u8 = 4;
const COLOR_RAINBOW_WAVE: u8 = 5;

// colors are only 24 bits, the first 8 bits are unused
#[derive(Debug, Clone, PartialEq)]
pub enum RGBControl {
    Off,
    Static {
        color: (u8, u8, u8),
    },
    Wave {
        speed_x: i8,
        speed_y: i8,
        color_count: u8,
        colors: [(u8, u8, u8); 4],
    },
    Breathe {
        hold_time: u8,
        trans_time: u8,
        color_count: u8,
        colors: [(u8, u8, u8); 4],
    },
    RainbowSolid {
        speed: i8,
        saturation: u8,
        value: u8,
    },
    RainbowWave {
        speed: i8,
        speed_x: i8,
        speed_y: i8,
        saturation: u8,
        value: u8,
    },
}
impl RGBControl {
    pub fn encode(self) -> [u8; 32] {
        match self {
            Self::Off => [0u8; 32],
            Self::Static { color } => {
                let mut data = [0u8; 32];
                data[0] = COLOR_STATIC; // static type
                put_color(&mut data[1..=3], color);

                data
            }
            Self::Wave {
                speed_x,
                speed_y,
                color_count,
                colors,
            } => {
                let mut data = [0u8; 32];
                data[0] = COLOR_WAVE; // wave type
                data[1] = unsafe { transmute(speed_x) };
                data[2] = unsafe { transmute(speed_y) };
                data[3] = color_count;
                put_color(&mut data[4..=6], colors[0]);
                put_color(&mut data[7..=9], colors[1]);
                put_color(&mut data[10..=12], colors[2]);
                put_color(&mut data[13..=15], colors[3]);

                data
            }
            Self::Breathe {
                hold_time,
                trans_time,
                color_count,
                colors,
            } => {
                let mut data = [0u8; 32];
                data[0] = COLOR_BREATHE; // breathe type
                data[1] = hold_time;
                data[2] = trans_time;
                data[3] = color_count;
                put_color(&mut data[4..=6], colors[0]);
                put_color(&mut data[7..=9], colors[1]);
                put_color(&mut data[10..=12], colors[2]);
                put_color(&mut data[13..=15], colors[3]);

                data
            }
            Self::RainbowSolid {
                speed,
                saturation,
                value,
            } => {
                let mut data = [0u8; 32];
                data[0] = COLOR_RAINBOW_SOLID; // rainbow solid
                data[1] = unsafe { transmute(speed) };
                data[2] = unsafe { transmute(saturation) };
                data[3] = unsafe { transmute(value) };

                data
            }
            Self::RainbowWave {
                speed,
                speed_x,
                speed_y,
                saturation,
                value,
            } => {
                let mut data = [0u8; 32];
                data[0] = COLOR_RAINBOW_WAVE; // rainbow wave
                data[1] = unsafe { transmute(speed) };
                data[2] = unsafe { transmute(speed_x) };
                data[3] = unsafe { transmute(speed_y) };
                data[4] = unsafe { transmute(saturation) };
                data[5] = unsafe { transmute(value) };

                data
            }
        }
    }

    pub fn decode(data: [u8; 32]) -> Self {
        let t = data[0];
        match t {
            COLOR_STATIC => Self::Static {
                color: get_color(&data[1..=3]),
            },
            COLOR_WAVE => Self::Wave {
                speed_x: unsafe { transmute(data[1]) },
                speed_y: unsafe { transmute(data[2]) },
                color_count: data[3],
                colors: [
                    get_color(&data[4..=6]),
                    get_color(&data[7..=9]),
                    get_color(&data[10..=12]),
                    get_color(&data[13..=15]),
                ],
            },
            COLOR_BREATHE => Self::Breathe {
                hold_time: data[1],
                trans_time: data[2],
                color_count: data[3],
                colors: [
                    get_color(&data[4..=6]),
                    get_color(&data[7..=9]),
                    get_color(&data[10..=12]),
                    get_color(&data[13..=15]),
                ],
            },
            COLOR_RAINBOW_SOLID => Self::RainbowSolid {
                speed: unsafe { transmute(data[1]) },
                saturation: data[2],
                value: data[3],
            },
            COLOR_RAINBOW_WAVE => Self::RainbowWave {
                speed: unsafe { transmute(data[1]) },
                speed_x: unsafe { transmute(data[2]) },
                speed_y: unsafe { transmute(data[3]) },
                saturation: data[4],
                value: data[5],
            },
            _ => Self::Off,
        }
    }
}
