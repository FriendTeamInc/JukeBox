//! Screen for fun graphics

#![allow(dead_code)]

#[allow(unused_imports)]
use defmt::*;

use embedded_hal::timer::CountDown as _;
use rp2040_hal::{
    fugit::ExtU32,
    gpio::{DynPinId, FunctionPio1, Pin, PullDown},
    pac::PIO1,
    pio::SM1,
    timer::{CountDown, Instant},
    Timer,
};

use crate::{st7789::St7789, ConnectionStatus};

macro_rules! load_bmp {
    ($path:literal) => {{
        let (_, bmp) = include_bytes!($path).split_at(0x7A);
        if bmp.len() != (64 * 64 * 2) {
            core::panic!()
        }
        let mut bytes = [0u16; 64 * 64];

        let mut i = 0;
        while i < (64 * 64) {
            bytes[i] = ((bmp[i * 2 + 1] as u16) << 8) | (bmp[i * 2] as u16);
            i += 1;
        }
        bytes
    }};
}

const DISCORD_ICON: [u16; 4096] = load_bmp!("../../../assets/action_icons/discord-base.bmp");

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

    pub fn draw_icon(&mut self, icon: &[u16], x: usize, y: usize) {
        let old_color = self.st.get_color();
        let mut h = 0;
        while h < 64 {
            let mut w = 0;
            while w < 64 {
                let c = icon[64 * h + w];
                self.st.set_color(c);
                self.st.put_pixel(63 - h + x, w + y);
                w += 1;
            }
            h += 1;
        }
        self.st.set_color(0);

        self.st.rectangle(x, y, 2, 2);
        self.st.rectangle(x, y, 4, 1);
        self.st.rectangle(x, y, 1, 4);

        self.st.rectangle(x, y + 64 - 2, 2, 2);
        self.st.rectangle(x, y + 64 - 2 + 1, 4, 1);
        self.st.rectangle(x, y + 64 - 2 - 2, 1, 4);

        self.st.rectangle(x + 64 - 2, y, 2, 2);
        self.st.rectangle(x + 64 - 2 - 2, y, 4, 1);
        self.st.rectangle(x + 64 - 2 + 1, y, 1, 4);

        self.st.rectangle(x + 64 - 2, y + 64 - 2, 2, 2);
        self.st.rectangle(x + 64 - 2 - 2, y + 64 - 2 + 1, 4, 1);
        self.st.rectangle(x + 64 - 2 + 1, y + 64 - 2 - 2, 1, 4);

        self.st.set_color(old_color);
    }

    pub fn update(&mut self, _t: Instant, _timer: &Timer) {
        if !self.timer.wait().is_ok() {
            return;
        }

        self.st.clear_framebuffer();

        for y in 0..3 {
            for x in 0..4 {
                self.draw_icon(&DISCORD_ICON, 2 + (64 + 6) * y, 23 + (64 + 6) * x);
            }
        }

        self.st.push_framebuffer();
    }
}
