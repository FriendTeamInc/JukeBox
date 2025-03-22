//! Screen for fun graphics

#![allow(dead_code)]

#[allow(unused_imports)]
use defmt::*;

use embedded_graphics::{
    pixelcolor::Bgr565,
    prelude::{Point, Primitive, RgbColor, *},
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
};
use embedded_graphics_framebuf::FrameBuf;
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
        let mut bytes = [Bgr565::BLACK; 64 * 64];

        let mut i = 0;
        while i < (64 * 64) {
            let c = ((bmp[i * 2 + 1] as u16) << 8) | (bmp[i * 2] as u16);
            let b = ((c & 0b11111_000000_00000) >> 11) as u8;
            let g = ((c & 0b00000_111111_00000) >> 5) as u8;
            let r = ((c & 0b00000_000000_11111) >> 0) as u8;
            bytes[i] = Bgr565::new(r, g, b);
            i += 1;
        }
        bytes
    }};
}

const ICON_BASE: &[[Bgr565; 64 * 64]] = &[
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

const REFRESH_RATE: u32 = 16;
pub const SCR_W: usize = 240;
pub const SCR_H: usize = 320;
static mut FBDATA: [Bgr565; SCR_W * SCR_H] = [Bgr565::BLACK; SCR_W * SCR_H];

pub struct ScreenMod {
    st: St7789<PIO1, SM1, Pin<DynPinId, FunctionPio1, PullDown>>,
    timer: CountDown,
    connection_status: &'static ConnectionStatus,
    fb: FrameBuf<Bgr565, &'static mut [Bgr565; SCR_W * SCR_H]>,
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
            #[allow(static_mut_refs)] // This is probably bad. LOL.
            fb: FrameBuf::new(unsafe { &mut FBDATA }, SCR_W, SCR_H),
        }
    }

    fn backlight_off(&mut self) {
        self.st.backlight_off();
    }

    pub fn clear(&mut self) {
        self.clear_fb();
        self.push_fb();
        self.backlight_off();
    }

    fn clear_fb(&mut self) {
        let _ = Rectangle::new(Point::new(0, 0), self.fb.size())
            .into_styled(PrimitiveStyle::with_fill(Bgr565::BLACK))
            .draw(&mut self.fb);
    }

    fn push_fb(&mut self) {
        self.st.push_framebuffer(&self.fb);
    }

    fn draw_icon(&mut self, icon: &[Bgr565], x: usize, y: usize) {
        let mut h = 0;
        while h < 64 {
            let mut w = 0;
            while w < 64 {
                let c = icon[64 * h + w];
                self.fb
                    .set_color_at(Point::new((63 - h + x) as i32, (w + y) as i32), c);
                w += 1;
            }
            h += 1;
        }
    }

    pub fn update(&mut self, keys: [bool; 16], _t: Instant, timer: &Timer) {
        if !self.timer.wait().is_ok() {
            return;
        }

        // let t = ((t.duration_since_epoch().ticks() >> 14) % 360) as f32;

        let time_start = timer.get_counter();

        // clear framebuffer
        self.clear_fb();

        for y in 0..3 {
            for x in 0..4 {
                self.draw_icon(&ICON_BASE[y * 4 + x], 2 + (64 + 6) * y, 23 + (64 + 6) * x);
            }
        }

        for y in 0..3 {
            for x in 0..4 {
                let _ = RoundedRectangle::with_equal_corners(
                    Rectangle::new(
                        Point::new(2 + (64 + 6) * y, 23 + (64 + 6) * x),
                        Size::new(64, 64),
                    ),
                    if keys[(y * 4 + x) as usize] {
                        Size::new(30, 30)
                    } else {
                        Size::new(4, 4)
                    },
                )
                .into_styled(PrimitiveStyle::with_stroke(Bgr565::WHITE, 4))
                .draw(&mut self.fb);
            }
        }

        let elapse1 = (timer.get_counter() - time_start).to_micros();

        let time_start = timer.get_counter();
        self.push_fb();
        let elapse2 = (timer.get_counter() - time_start).to_micros();

        info!("times: draw-fb={}us, push-fb={}us", elapse1, elapse2);
    }
}
