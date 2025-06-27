use core::mem::transmute;

use crate::color::hsv2rgb;

const fn set_color(data: &mut [u8], color: (u8, u8, u8)) {
    data[0] = color.0;
    data[1] = color.1;
    data[2] = color.2;
}

const fn get_color(data: &[u8]) -> (u8, u8, u8) {
    (data[0], data[1], data[2])
}

pub const RGB_PROFILE_SIZE: usize = 40;

pub const RGB_PROFILE_OFF: u8 = 0;
pub const RGB_PROFILE_STATIC_SOLID: u8 = 1;
pub const RGB_PROFILE_STATIC_PER_KEY: u8 = 2;
pub const RGB_PROFILE_WAVE: u8 = 3;
pub const RGB_PROFILE_BREATHE: u8 = 4;
pub const RGB_PROFILE_RAINBOW_SOLID: u8 = 5;
pub const RGB_PROFILE_RAINBOW_WAVE: u8 = 6;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RgbProfile {
    Off,
    StaticSolid {
        brightness: u8,
        color: (u8, u8, u8),
    },
    StaticPerKey {
        brightness: u8,
        colors: [(u8, u8, u8); 12],
    },
    Wave {
        brightness: u8,
        speed: i8,
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
            Self::Off => RGB_PROFILE_OFF,
            Self::StaticSolid {
                brightness: _,
                color: _,
            } => RGB_PROFILE_STATIC_SOLID,
            Self::StaticPerKey {
                brightness: _,
                colors: _,
            } => RGB_PROFILE_STATIC_PER_KEY,
            Self::Wave {
                brightness: _,
                speed: _,
                speed_x: _,
                speed_y: _,
                color_count: _,
                colors: _,
            } => RGB_PROFILE_WAVE,
            Self::Breathe {
                brightness: _,
                hold_time: _,
                trans_time: _,
                color_count: _,
                colors: _,
            } => RGB_PROFILE_BREATHE,
            Self::RainbowSolid {
                brightness: _,
                speed: _,
                saturation: _,
                value: _,
            } => RGB_PROFILE_RAINBOW_SOLID,
            Self::RainbowWave {
                brightness: _,
                speed: _,
                speed_x: _,
                speed_y: _,
                saturation: _,
                value: _,
            } => RGB_PROFILE_RAINBOW_WAVE,
        }
    }

    pub fn brightness(&self) -> u8 {
        match self {
            Self::Off => 0,
            Self::StaticSolid {
                brightness,
                color: _,
            } => *brightness,
            Self::StaticPerKey {
                brightness,
                colors: _,
            } => *brightness,
            Self::Wave {
                brightness,
                speed: _,
                speed_x: _,
                speed_y: _,
                color_count: _,
                colors: _,
            } => *brightness,
            Self::Breathe {
                brightness,
                hold_time: _,
                trans_time: _,
                color_count: _,
                colors: _,
            } => *brightness,
            Self::RainbowSolid {
                brightness,
                speed: _,
                saturation: _,
                value: _,
            } => *brightness,
            Self::RainbowWave {
                brightness,
                speed: _,
                speed_x: _,
                speed_y: _,
                saturation: _,
                value: _,
            } => *brightness,
        }
    }

    pub fn encode(self) -> [u8; RGB_PROFILE_SIZE] {
        let mut data = [0u8; RGB_PROFILE_SIZE];
        data[0] = self.get_type();
        data[1] = self.brightness();

        match self {
            Self::Off => (),
            Self::StaticSolid {
                brightness: _,
                color,
            } => {
                set_color(&mut data[2..=4], color);
            }
            Self::StaticPerKey {
                brightness: _,
                colors,
            } => {
                set_color(&mut data[2..=4], colors[0]);
                set_color(&mut data[5..=7], colors[1]);
                set_color(&mut data[8..=10], colors[2]);
                set_color(&mut data[11..=13], colors[3]);
                set_color(&mut data[14..=16], colors[4]);
                set_color(&mut data[17..=19], colors[5]);
                set_color(&mut data[20..=22], colors[6]);
                set_color(&mut data[23..=25], colors[7]);
                set_color(&mut data[26..=28], colors[8]);
                set_color(&mut data[29..=31], colors[9]);
                set_color(&mut data[32..=34], colors[10]);
                set_color(&mut data[35..=37], colors[11]);
            }
            Self::Wave {
                brightness: _,
                speed,
                speed_x,
                speed_y,
                color_count,
                colors,
            } => {
                data[2] = unsafe { transmute(speed) };
                data[3] = unsafe { transmute(speed_x) };
                data[4] = unsafe { transmute(speed_y) };
                data[5] = color_count;
                set_color(&mut data[6..=8], colors[0]);
                set_color(&mut data[9..=11], colors[1]);
                set_color(&mut data[12..=14], colors[2]);
                set_color(&mut data[15..=17], colors[3]);
            }
            Self::Breathe {
                brightness: _,
                hold_time,
                trans_time,
                color_count,
                colors,
            } => {
                data[2] = hold_time;
                data[3] = trans_time;
                data[4] = color_count;
                set_color(&mut data[5..=7], colors[0]);
                set_color(&mut data[8..=10], colors[1]);
                set_color(&mut data[11..=13], colors[2]);
                set_color(&mut data[14..=16], colors[3]);
            }
            Self::RainbowSolid {
                brightness: _,
                speed,
                saturation,
                value,
            } => {
                data[2] = unsafe { transmute(speed) };
                data[3] = unsafe { transmute(saturation) };
                data[4] = unsafe { transmute(value) };
            }
            Self::RainbowWave {
                brightness: _,
                speed,
                speed_x,
                speed_y,
                saturation,
                value,
            } => {
                data[2] = unsafe { transmute(speed) };
                data[3] = unsafe { transmute(speed_x) };
                data[4] = unsafe { transmute(speed_y) };
                data[5] = unsafe { transmute(saturation) };
                data[6] = unsafe { transmute(value) };
            }
        }

        data
    }

    pub fn decode(data: &[u8]) -> Self {
        match data[0] {
            RGB_PROFILE_OFF => Self::Off,
            RGB_PROFILE_STATIC_SOLID => Self::StaticSolid {
                brightness: data[1],
                color: get_color(&data[2..=4]),
            },
            RGB_PROFILE_STATIC_PER_KEY => Self::StaticPerKey {
                brightness: data[1],
                colors: [
                    get_color(&data[2..=4]),
                    get_color(&data[5..=7]),
                    get_color(&data[8..=10]),
                    get_color(&data[11..=13]),
                    get_color(&data[14..=16]),
                    get_color(&data[17..=19]),
                    get_color(&data[20..=22]),
                    get_color(&data[23..=25]),
                    get_color(&data[26..=28]),
                    get_color(&data[29..=31]),
                    get_color(&data[32..=34]),
                    get_color(&data[35..=37]),
                ],
            },
            RGB_PROFILE_WAVE => Self::Wave {
                brightness: data[1],
                speed: unsafe { transmute(data[2]) },
                speed_x: unsafe { transmute(data[3]) },
                speed_y: unsafe { transmute(data[4]) },
                color_count: data[5],
                colors: [
                    get_color(&data[6..=8]),
                    get_color(&data[9..=11]),
                    get_color(&data[12..=14]),
                    get_color(&data[15..=17]),
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
            _ => panic!(),
        }
    }

    pub fn calculate_matrix(&self, t: u64) -> [(u8, u8, u8); 12] {
        let mut buffer = [(0u8, 0u8, 0u8); 12];

        match self {
            Self::Off => {}
            Self::StaticSolid {
                brightness: _,
                color,
            } => {
                for led in buffer.iter_mut() {
                    *led = *color;
                }
            }
            Self::StaticPerKey {
                brightness: _,
                colors,
            } => {
                for (i, led) in buffer.iter_mut().enumerate() {
                    *led = colors[i];
                }
            }
            Self::Wave {
                brightness: _,
                speed,
                speed_x,
                speed_y,
                color_count,
                colors,
            } => {
                let color_count = *color_count as usize;
                let n = color_count as f32;
                let sx = *speed_x as f32;
                let sy = *speed_y as f32;
                let t = (t as f32) / 100_000.0 * (*speed as f32);
                let factor = 50f32;

                for i in 0..12 {
                    let x = (i % 4) as f32;
                    let y = (i / 4) as f32;

                    let t = (t + (sx * x) + (sy * y)) % (n * factor);
                    let r = (t / factor) as usize;

                    let color1 = colors[r];
                    let color2 = colors[if r + 1 == color_count { 0 } else { r + 1 }];
                    let p = (t % factor) / factor;

                    let trans_color = (
                        ((color1.0 as f32) + (((color2.0 as f32) - (color1.0 as f32)) * p)) as u8,
                        ((color1.1 as f32) + (((color2.1 as f32) - (color1.1 as f32)) * p)) as u8,
                        ((color1.2 as f32) + (((color2.2 as f32) - (color1.2 as f32)) * p)) as u8,
                    );

                    buffer[i] = trans_color;
                }
            }
            Self::Breathe {
                brightness: _,
                hold_time,
                trans_time,
                color_count,
                colors,
            } => {
                let color_count = *color_count as usize;
                let h = (*hold_time as u64) * 100_000;
                let r = (*trans_time as u64) * 100_000;
                let n = color_count as u64;
                let t = t % (n * (h + r));
                let c = t % (h + r);
                let n = (t / (h + r)) as usize;

                if c > h {
                    // transitioning color
                    let p = ((c - h) as f32) / (r as f32);
                    let color1 = colors[n];
                    let color2 = colors[if n + 1 == color_count { 0 } else { n + 1 }];
                    let trans_color = (
                        ((color1.0 as f32) + (((color2.0 as f32) - (color1.0 as f32)) * p)) as u8,
                        ((color1.1 as f32) + (((color2.1 as f32) - (color1.1 as f32)) * p)) as u8,
                        ((color1.2 as f32) + (((color2.2 as f32) - (color1.2 as f32)) * p)) as u8,
                    );
                    for led in buffer.iter_mut() {
                        *led = trans_color.into();
                    }
                } else {
                    // holding color
                    for led in buffer.iter_mut() {
                        *led = colors[n].into();
                    }
                }
            }
            Self::RainbowSolid {
                brightness: _,
                speed,
                saturation,
                value,
            } => {
                let t = t as f32;
                let s = *speed as f32;
                let sat = (*saturation as f32) / 100.0;
                let val = (*value as f32) / 100.0;
                let s = hsv2rgb((t / 1_000_000.0 * s) % 360.0, sat, val).into();

                for led in buffer.iter_mut() {
                    *led = s;
                }
            }
            Self::RainbowWave {
                brightness: _,
                speed,
                speed_x,
                speed_y,
                saturation,
                value,
            } => {
                let t = t as f32;
                let s = *speed as f32;
                let sx = *speed_x as f32;
                let sy = *speed_y as f32;

                let sat = (*saturation as f32) / 100.0;
                let val = (*value as f32) / 100.0;

                for (i, led) in buffer.iter_mut().enumerate() {
                    let x = (i % 4) as f32;
                    let y = (i / 4) as f32;
                    *led = hsv2rgb(
                        (t / 1_000_000.0 * s + (sx * x) + (sy * y)) % 360.0,
                        sat,
                        val,
                    )
                    .into();
                }
            }
        };

        buffer
    }

    pub const fn default_device_profile() -> Self {
        Self::Breathe {
            brightness: 20,
            hold_time: 20,
            trans_time: 10,
            color_count: 2,
            colors: [(255, 255, 255), (150, 150, 150), (0, 0, 0), (0, 0, 0)],
        }
    }

    pub const fn default_gui_profile() -> Self {
        Self::default_rainbow_wave()
    }

    pub const fn default_static_solid() -> Self {
        RgbProfile::StaticSolid {
            brightness: 25,
            color: (255, 200, 100),
        }
    }

    pub const fn default_static_per_key() -> Self {
        RgbProfile::StaticPerKey {
            brightness: 25,
            colors: [
                (100, 155, 255),
                (255, 200, 100),
                (255, 200, 100),
                (100, 155, 255),
                (255, 200, 100),
                (100, 155, 255),
                (100, 155, 255),
                (255, 200, 100),
                (100, 155, 255),
                (255, 200, 100),
                (255, 200, 100),
                (100, 155, 255),
            ],
        }
    }

    pub const fn default_wave() -> Self {
        RgbProfile::Wave {
            brightness: 25,
            speed: 10,
            speed_x: 20,
            speed_y: 0,
            color_count: 3,
            colors: [(51, 187, 255), (153, 119, 255), (255, 119, 221), (0, 0, 0)],
        }
    }

    pub const fn default_breathe() -> Self {
        RgbProfile::Breathe {
            brightness: 25,
            hold_time: 20,
            trans_time: 5,
            color_count: 3,
            colors: [(51, 187, 255), (153, 119, 255), (255, 119, 221), (0, 0, 0)],
        }
    }

    pub const fn default_rainbow_solid() -> Self {
        RgbProfile::RainbowSolid {
            brightness: 25,
            speed: 30,
            saturation: 100,
            value: 100,
        }
    }

    pub const fn default_rainbow_wave() -> Self {
        RgbProfile::RainbowWave {
            brightness: 25,
            speed: 100,
            speed_x: 0,
            speed_y: 30,
            saturation: 100,
            value: 100,
        }
    }
}
