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
pub const RGB_PROFILE_STATIC_SOLID: u8 = 1;
pub const RGB_PROFILE_STATIC_PER_KEY: u8 = 2;
pub const RGB_PROFILE_WAVE: u8 = 3;
pub const RGB_PROFILE_BREATHE: u8 = 4;
pub const RGB_PROFILE_RAINBOW_SOLID: u8 = 5;
pub const RGB_PROFILE_RAINBOW_WAVE: u8 = 6;

// colors are only 24 bits, the first 8 bits are unused
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
            RgbProfile::StaticSolid {
                brightness: _,
                color: _,
            } => RGB_PROFILE_STATIC_SOLID,
            RgbProfile::StaticPerKey {
                brightness: _,
                colors: _,
            } => RGB_PROFILE_STATIC_PER_KEY,
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

    pub fn encode(self) -> [u8; 60] {
        match self {
            Self::Off => [0u8; 60],
            Self::StaticSolid { brightness, color } => {
                let mut data = [0u8; 60];
                data[0] = self.get_type();
                data[1] = brightness;
                set_color(&mut data[2..=4], color);

                data
            }
            Self::StaticPerKey { brightness, colors } => {
                let mut data = [0u8; 60];
                data[0] = self.get_type();
                data[1] = brightness;
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

                data
            }
            Self::Wave {
                brightness,
                speed_x,
                speed_y,
                color_count,
                colors,
            } => {
                let mut data = [0u8; 60];
                data[0] = self.get_type();
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
                let mut data = [0u8; 60];
                data[0] = self.get_type();
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
                let mut data = [0u8; 60];
                data[0] = self.get_type();
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
                let mut data = [0u8; 60];
                data[0] = self.get_type();
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

    pub fn decode(data: &[u8]) -> Self {
        let t = data[0];
        match t {
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

    pub fn brightness(&self) -> u8 {
        match self {
            RgbProfile::Off => 0,
            RgbProfile::StaticSolid {
                brightness,
                color: _,
            } => *brightness,
            RgbProfile::StaticPerKey {
                brightness,
                colors: _,
            } => *brightness,
            RgbProfile::Wave {
                brightness,
                speed_x: _,
                speed_y: _,
                color_count: _,
                colors: _,
            } => *brightness,
            RgbProfile::Breathe {
                brightness,
                hold_time: _,
                trans_time: _,
                color_count: _,
                colors: _,
            } => *brightness,
            RgbProfile::RainbowSolid {
                brightness,
                speed: _,
                saturation: _,
                value: _,
            } => *brightness,
            RgbProfile::RainbowWave {
                brightness,
                speed: _,
                speed_x: _,
                speed_y: _,
                saturation: _,
                value: _,
            } => *brightness,
        }
    }

    pub fn calculate_matrix(&self, t: u64) -> [(u8, u8, u8); 12] {
        let mut buffer = [(0u8, 0u8, 0u8); 12];

        match self {
            RgbProfile::Off => {}
            RgbProfile::StaticSolid {
                brightness: _,
                color,
            } => {
                for led in buffer.iter_mut() {
                    *led = *color;
                }
            }
            RgbProfile::StaticPerKey {
                brightness: _,
                colors,
            } => {
                for (i, led) in buffer.iter_mut().enumerate() {
                    *led = colors[i];
                }
            }
            RgbProfile::Wave {
                brightness: _,
                speed_x: _,
                speed_y: _,
                color_count: _,
                colors: _,
            } => todo!(),
            RgbProfile::Breathe {
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
            RgbProfile::RainbowSolid {
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
            RgbProfile::RainbowWave {
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
            colors: [(255, 255, 255), (127, 127, 127), (0, 0, 0), (0, 0, 0)],
        }
    }

    pub const fn default_gui_profile() -> Self {
        Self::RainbowWave {
            brightness: 25,
            speed: 100,
            speed_x: 0,
            speed_y: 30,
            saturation: 100,
            value: 100,
        }
    }
}
