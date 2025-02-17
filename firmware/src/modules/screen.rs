//! Screen for fun graphics

#[allow(unused_imports)]
use defmt::*;

use embedded_hal::timer::CountDown as _;
use jukebox_util::color::{hsv2rgb, rgb565};
use rp2040_hal::{
    fugit::ExtU32,
    gpio::{DynPinId, FunctionPio1, Pin, PullDown},
    pac::PIO1,
    pio::SM1,
    timer::{CountDown, Instant},
    Timer,
};

use crate::st7789::St7789;

const REFRESH_RATE: u32 = 30;

pub struct ScreenMod {
    st: St7789<PIO1, SM1, Pin<DynPinId, FunctionPio1, PullDown>>,
    timer: CountDown,
}

impl ScreenMod {
    pub fn new(
        st: St7789<PIO1, SM1, Pin<DynPinId, FunctionPio1, PullDown>>,
        mut count_down: CountDown,
    ) -> Self {
        count_down.start(REFRESH_RATE.millis());

        ScreenMod {
            st: st,
            timer: count_down,
        }
    }

    pub fn clear(&mut self) {
        self.st.backlight_off();
        self.st.clear_framebuffer();
        self.st.push_framebuffer();
    }

    pub fn update(&mut self, t: Instant, _timer: &Timer) {
        if !self.timer.wait().is_ok() {
            return;
        }

        let t = ((t.duration_since_epoch().ticks() >> 14) % 360) as f32;
        let rgb = hsv2rgb(t, 1.0, 1.0);
        let rgb = rgb565(rgb.0, rgb.1, rgb.2);

        let time_start = _timer.get_counter();
        self.st.fill_framebuffer(rgb);
        let elapse1 = (_timer.get_counter() - time_start).to_micros();

        let time_start = _timer.get_counter();
        self.st.push_framebuffer();
        let elapse2 = (_timer.get_counter() - time_start).to_micros();

        info!("times: fill-fb={}us, push-fb={}us", elapse1, elapse2);
    }
}
