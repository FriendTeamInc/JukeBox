//! Screen for fun graphics

#![allow(dead_code)]

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

use crate::{st7789::St7789, ConnectionStatus};

const REFRESH_RATE: u32 = 33;

pub struct ScreenMod {
    st: St7789<PIO1, SM1, Pin<DynPinId, FunctionPio1, PullDown>>,
    timer: CountDown,
    connection_status: &'static ConnectionStatus,
}

impl ScreenMod {
    pub fn new(
        mut st: St7789<PIO1, SM1, Pin<DynPinId, FunctionPio1, PullDown>>,
        mut count_down: CountDown,
        connection_status: &'static ConnectionStatus,
    ) -> Self {
        count_down.start(REFRESH_RATE.millis());

        st.backlight_on();

        ScreenMod {
            st: st,
            timer: count_down,
            connection_status,
        }
    }

    pub fn backlight_off(&mut self) {
        self.st.backlight_off();
    }

    pub fn clear(&mut self) {
        self.st.clear_framebuffer();
        self.st.push_framebuffer();
    }

    pub fn update(&mut self, _t: Instant, _timer: &Timer) {
        if !self.timer.wait().is_ok() {
            return;
        }

        let t = ((_t.duration_since_epoch().ticks() >> 14) % 360) as f32;
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
