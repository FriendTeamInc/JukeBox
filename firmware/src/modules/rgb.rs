//! RGB LEDs under the keys

#![allow(dead_code)]

use embedded_hal::timer::CountDown as _;
use jukebox_util::color::hsv2rgb;
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

const RGB_LEN: usize = 12;
const FRAME_TIME: u32 = 33;

pub struct RgbMod {
    ws: Ws2812<PIO0, SM0, CountDown, Pin<DynPinId, FunctionPio0, PullDown>>,
    brightness: u8,
    buffer: [RGB8; RGB_LEN],
    timer: CountDown,
}

impl RgbMod {
    pub fn new(
        ws: Ws2812<PIO0, SM0, CountDown, Pin<DynPinId, FunctionPio0, PullDown>>,
        mut count_down: CountDown,
    ) -> Self {
        count_down.start(FRAME_TIME.millis());

        RgbMod {
            ws: ws,
            brightness: 40,
            buffer: [(0, 0, 0).into(); RGB_LEN],
            timer: count_down,
        }
    }

    pub fn clear(&mut self) {
        self.brightness = 0;
        self.buffer = [(0, 0, 0).into(); RGB_LEN];
        self.ws
            .write(brightness(self.buffer.iter().copied(), self.brightness))
            .unwrap();
    }

    pub fn update(&mut self, t: Instant) {
        if !self.timer.wait().is_ok() {
            return;
        }

        let t = ((t.duration_since_epoch().ticks() >> 14) % 360) as f32;

        for (i, led) in self.buffer.iter_mut().enumerate() {
            *led = hsv2rgb((t + (10 * (RGB_LEN - i)) as f32) % 360.0, 1.0, 1.0).into();
        }

        self.ws
            .write(brightness(self.buffer.iter().copied(), self.brightness))
            .unwrap();
    }
}
