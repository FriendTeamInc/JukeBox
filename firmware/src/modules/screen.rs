//! Screen for fun graphics

#![allow(dead_code)]

#[allow(unused_imports)]
use defmt::*;

use embedded_dma::Word;
use embedded_hal::timer::CountDown as _;
use rp2040_hal::{
    dma::{single_buffer, Channel, CH0, CH1, CH2, CH3},
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

const ICON_BASE: &[[u16; 4096]] = &[
    load_bmp!("../../../assets/action_icons/meta-noaction.bmp"),
    load_bmp!("../../../assets/action_icons/meta-switchprofile.bmp"),
    load_bmp!("../../../assets/action_icons/meta-copyfromprofile.bmp"),
    load_bmp!("../../../assets/action_icons/system-appopen.bmp"),
    // load_bmp!("../../../assets/action_icons/system-webopen.bmp"),
    // load_bmp!("../../../assets/action_icons/system-inputcontrol.bmp"),
    load_bmp!("../../../assets/action_icons/system-outputcontrol.bmp"),
    load_bmp!("../../../assets/action_icons/soundboard-play.bmp"),
    load_bmp!("../../../assets/action_icons/input-keyboard.bmp"),
    load_bmp!("../../../assets/action_icons/input-mouse.bmp"),
    load_bmp!("../../../assets/action_icons/input-gamepad.bmp"),
    load_bmp!("../../../assets/action_icons/discord-headphones-1.bmp"),
    // load_bmp!("../../../assets/action_icons/discord-headphones-2.bmp"),
    load_bmp!("../../../assets/action_icons/discord-microphone-1.bmp"),
    // load_bmp!("../../../assets/action_icons/discord-microphone-2.bmp"),
    // load_bmp!("../../../assets/action_icons/discord-talking-1.bmp"),
    // load_bmp!("../../../assets/action_icons/discord-talking-2.bmp"),
    load_bmp!("../../../assets/action_icons/obs-base.bmp"),
];

const REFRESH_RATE: u32 = 50;
pub const SCR_W: usize = 240;
pub const SCR_H: usize = 320;
static mut FBDATA: [u16; SCR_W * SCR_H] = [0; SCR_W * SCR_H];
static CLEAR_VAL: RepeatReadTarget<u16> = RepeatReadTarget(0);
#[derive(Clone, Copy)]
struct RepeatReadTarget<W: Word>(W);
unsafe impl<W: Word> embedded_dma::ReadTarget for RepeatReadTarget<W> {
    type Word = W;
}
unsafe impl<W: Word> rp2040_hal::dma::ReadTarget for RepeatReadTarget<W> {
    type ReceivedWord = W;

    fn rx_treq() -> Option<u8> {
        None
    }

    fn rx_address_count(&self) -> (u32, u32) {
        (self as *const Self as u32, u32::MAX)
    }

    fn rx_increment(&self) -> bool {
        false
    }
}

pub struct ScreenMod {
    st: St7789<PIO1, SM1, Pin<DynPinId, FunctionPio1, PullDown>>,
    dma_ch0: Channel<CH0>,
    dma_ch1: Channel<CH1>,
    dma_ch2: Channel<CH2>,
    dma_ch3: Channel<CH3>,
    timer: CountDown,
    connection_status: &'static ConnectionStatus,
}

impl ScreenMod {
    pub fn new(
        st: St7789<PIO1, SM1, Pin<DynPinId, FunctionPio1, PullDown>>,
        dma_ch0: Channel<CH0>,
        dma_ch1: Channel<CH1>,
        dma_ch2: Channel<CH2>,
        dma_ch3: Channel<CH3>,
        mut timer: CountDown,
        connection_status: &'static ConnectionStatus,
    ) -> Self {
        timer.start(REFRESH_RATE.millis());

        ScreenMod {
            st,
            dma_ch0,
            dma_ch1,
            dma_ch2,
            dma_ch3,
            timer,
            connection_status,
        }
    }

    pub fn clear(&mut self) {
        self.st.backlight_off();
    }

    const fn put_pixel(&mut self, color: u16, x: usize, y: usize) {
        if x >= SCR_W || y >= SCR_H {
            return;
        }
        // doing unchecked access did not meaningfully improve performance
        unsafe {
            FBDATA[y * SCR_W + x] = color;
        }
    }

    fn rectangle(&mut self, color: u16, x: usize, y: usize, w: usize, h: usize) {
        for h in 0..h {
            for w in 0..w {
                self.put_pixel(color, x + w, y + h);
            }
        }
    }

    fn draw_icon(&mut self, icon: &[u16], x: usize, y: usize) {
        // icon drawing
        let mut h = 0;
        while h < 64 {
            let mut w = 0;
            while w < 64 {
                let c = icon[64 * h + w];
                self.put_pixel(c, 63 - h + x, w + y);
                w += 1;
            }
            h += 1;
        }

        // rounded corners
        self.rectangle(0, x, y, 2, 2);
        self.rectangle(0, x, y, 4, 1);
        self.rectangle(0, x, y, 1, 4);
        self.rectangle(0, x, y + 64 - 2, 2, 2);
        self.rectangle(0, x, y + 64 - 2 + 1, 4, 1);
        self.rectangle(0, x, y + 64 - 2 - 2, 1, 4);
        self.rectangle(0, x + 64 - 2, y, 2, 2);
        self.rectangle(0, x + 64 - 2 - 2, y, 4, 1);
        self.rectangle(0, x + 64 - 2 + 1, y, 1, 4);
        self.rectangle(0, x + 64 - 2, y + 64 - 2, 2, 2);
        self.rectangle(0, x + 64 - 2 - 2, y + 64 - 2 + 1, 4, 1);
        self.rectangle(0, x + 64 - 2 + 1, y + 64 - 2 - 2, 1, 4);
    }

    pub fn update(mut self, _t: Instant, _timer: &Timer) -> Self {
        if !self.timer.wait().is_ok() {
            return self;
        }

        let s = _timer.get_counter();
        // using multiple channels did not meaningfully improve performance.
        self.dma_ch0 = {
            let (dma_ch0, _, _) =
                single_buffer::Config::new(self.dma_ch0, CLEAR_VAL, unsafe { &mut FBDATA })
                    .start()
                    .wait();

            dma_ch0
        };
        let elapse_clear_fb = _timer.get_counter() - s;

        let s = _timer.get_counter();
        for y in 0..3 {
            for x in 0..4 {
                self.draw_icon(&ICON_BASE[y * 4 + x], 2 + (64 + 6) * y, 23 + (64 + 6) * x);
            }
        }
        let elapse_draw_icons = _timer.get_counter() - s;

        let time_start = _timer.get_counter();
        self.st.push_framebuffer(unsafe { &FBDATA });
        let elapse_push_fb = _timer.get_counter() - time_start;
        self.st.backlight_on();

        info!(
            "times:\nclear-fb={}us\ndraw-icons={}us\npush-fb={}us\ntotal={}",
            elapse_clear_fb.to_micros(),
            elapse_draw_icons.to_micros(),
            elapse_push_fb.to_micros(),
            (elapse_clear_fb + elapse_draw_icons + elapse_push_fb).to_micros()
        );

        self
    }
}
