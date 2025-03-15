//! RGB LEDs under the keys

#![allow(dead_code)]

use embedded_hal::timer::CountDown as _;
use jukebox_util::color::{hsv2rgb, RgbProfile};
use rp2040_hal::{
    fugit::ExtU32,
    gpio::{DynPinId, FunctionPio0, Pin, PullDown},
    pac::PIO0,
    pio::SM0,
    timer::{CountDown, Instant},
};
use smart_leds::brightness;
use smart_leds_trait::{SmartLedsWrite, RGB8};
use ws2812_pio::Ws2812;

use crate::RgbControls;

const RGB_LEN: usize = 12;
const FRAME_TIME: u32 = 33;

pub struct RgbMod {
    ws: Ws2812<PIO0, SM0, CountDown, Pin<DynPinId, FunctionPio0, PullDown>>,
    buffer: [RGB8; RGB_LEN],
    timer: CountDown,
    rgb_mode: RgbProfile,
}

impl RgbMod {
    pub fn new(
        ws: Ws2812<PIO0, SM0, CountDown, Pin<DynPinId, FunctionPio0, PullDown>>,
        mut count_down: CountDown,
    ) -> Self {
        count_down.start(FRAME_TIME.millis());

        // let t1 = RGBControl::Off;
        // let t2 = RGBControl::Static {
        //     brightness: 0,
        //     color: (0x33, 0xBB, 0xFF),
        // };
        // let t3 = RGBControl::Wave {
        //     brightness: 0,
        //     speed_x: 0,
        //     speed_y: 0,
        //     color_count: 3,
        //     colors: [
        //         (0x33, 0xBB, 0xFF),
        //         (0x99, 0x77, 0xFF),
        //         (0xFF, 0x77, 0xDD),
        //         (0, 0, 0),
        //     ],
        // };
        // let t4 = RGBControl::Breathe {
        //     brightness: 0,
        //     hold_time: 50,
        //     trans_time: 10,
        //     color_count: 3,
        //     colors: [
        //         (0x33, 0xBB, 0xFF),
        //         (0x99, 0x77, 0xFF),
        //         (0xFF, 0x77, 0xDD),
        //         (0, 0, 0),
        //     ],
        // };
        // let t5 = RGBControl::RainbowSolid {
        //     brightness: 0,
        //     speed: 30,
        //     saturation: 100,
        //     value: 100,
        // };
        // let t6 = RGBControl::RainbowWave {
        //     brightness: 0,
        //     speed: 100,
        //     speed_x: 0,
        //     speed_y: 30,
        //     saturation: 100,
        //     value: 100,
        // };

        RgbMod {
            ws: ws,
            buffer: [(0, 0, 0).into(); RGB_LEN],
            timer: count_down,
            rgb_mode: RgbProfile::Off,
        }
    }

    pub fn clear(&mut self) {
        self.ws
            .write(brightness([(0, 0, 0).into(); RGB_LEN].iter().copied(), 0))
            .unwrap();
    }

    pub fn update(&mut self, t: Instant, rgb_controls: &RgbControls) {
        if !self.timer.wait().is_ok() {
            return;
        }

        let t = t.duration_since_epoch().ticks();

        rgb_controls.with_lock(|c| {
            if c.0 {
                self.rgb_mode = c.1.clone();
                // TODO: save rgb settings to eeprom
            }
        });

        let mut buffer = [(0, 0, 0).into(); RGB_LEN];

        let brtns = match self.rgb_mode {
            RgbProfile::Off => {
                self.clear();
                0
            }
            RgbProfile::Static { brightness, color } => {
                for led in buffer.iter_mut() {
                    *led = color.into();
                }
                brightness
            }
            RgbProfile::Wave {
                brightness,
                speed_x,
                speed_y,
                color_count,
                colors,
            } => todo!(),
            RgbProfile::Breathe {
                brightness,
                hold_time,
                trans_time,
                color_count,
                colors,
            } => {
                let color_count = color_count as usize;
                let h = (hold_time as u64) * 100_000;
                let r = (trans_time as u64) * 100_000;
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

                brightness
            }
            RgbProfile::RainbowSolid {
                brightness,
                speed,
                saturation,
                value,
            } => {
                let t = t as f32;
                let s = speed as f32;
                let sat = (saturation as f32) / 100.0;
                let val = (value as f32) / 100.0;
                let s = hsv2rgb((t / 1_000_000.0 * s) % 360.0, sat, val).into();

                for led in buffer.iter_mut() {
                    *led = s;
                }

                brightness
            }
            RgbProfile::RainbowWave {
                brightness,
                speed,
                speed_x,
                speed_y,
                saturation,
                value,
            } => {
                // TODO
                let t = t as f32;
                let s = speed as f32;
                let sx = speed_x as f32;
                let sy = speed_y as f32;

                let sat = (saturation as f32) / 100.0;
                let val = (value as f32) / 100.0;

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

                brightness
            }
        };

        // transform zigzag into appropriate grid
        self.buffer = [
            buffer[0], buffer[1], buffer[2], buffer[3], buffer[7], buffer[6], buffer[5], buffer[4],
            buffer[8], buffer[9], buffer[10], buffer[11],
        ];

        self.ws
            .write(brightness(self.buffer.iter().copied(), brtns))
            .unwrap();
    }
}
