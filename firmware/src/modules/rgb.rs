//! RGB LEDs under the keys

use embedded_hal::timer::CountDown as _;
use jukebox_util::rgb::RgbProfile;
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

use crate::util::{DEFAULT_RGB_PROFILE, RGB_CONTROLS};

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

        let default_rgb_profile = DEFAULT_RGB_PROFILE.with_lock(|p| p.1.clone());

        RgbMod {
            ws: ws,
            buffer: [(0, 0, 0).into(); RGB_LEN],
            timer: count_down,
            rgb_mode: default_rgb_profile,
        }
    }

    pub fn clear(&mut self) {
        self.ws
            .write(brightness([(0, 0, 0).into(); RGB_LEN].iter().copied(), 0))
            .unwrap();
    }

    pub fn update(&mut self, t: Instant) {
        if !self.timer.wait().is_ok() {
            return;
        }

        let t = t.duration_since_epoch().ticks();

        RGB_CONTROLS.with_lock(|c| {
            if c.0 {
                self.rgb_mode = c.1.clone();
            }
        });

        let buffer = self.rgb_mode.calculate_matrix(t);

        let brtns = self.rgb_mode.brightness();

        // transform zigzag into appropriate grid
        self.buffer = [
            buffer[0].into(),
            buffer[1].into(),
            buffer[2].into(),
            buffer[3].into(),
            buffer[7].into(),
            buffer[6].into(),
            buffer[5].into(),
            buffer[4].into(),
            buffer[8].into(),
            buffer[9].into(),
            buffer[10].into(),
            buffer[11].into(),
        ];

        self.ws
            .write(brightness(self.buffer.iter().copied(), brtns))
            .unwrap();
    }
}
