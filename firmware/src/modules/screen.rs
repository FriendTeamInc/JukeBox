//! Screen for fun graphics

#![allow(dead_code)]

#[allow(unused_imports)]
use defmt::*;

use embedded_dma::Word;
use embedded_graphics::{
    pixelcolor::Bgr565,
    prelude::{Point, Primitive, RgbColor, *},
    primitives::{PrimitiveStyleBuilder, Rectangle, RoundedRectangle, StrokeAlignment},
};
use embedded_graphics_framebuf::FrameBuf;
use embedded_hal::timer::CountDown as _;
use rp2040_hal::{
    dma::{single_buffer, Channel, CH0},
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

const REFRESH_RATE: u32 = 100;
pub const SCR_W: usize = 240;
pub const SCR_H: usize = 320;
static mut FBDATA: [Bgr565; SCR_W * SCR_H] = [Bgr565::BLACK; SCR_W * SCR_H];

static CLEAR_VAL: RepeatReadTarget<u8> = RepeatReadTarget(0);
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
    timer: CountDown,
    connection_status: &'static ConnectionStatus,
    fb: FrameBuf<Bgr565, &'static mut [Bgr565; SCR_W * SCR_H]>,
}

impl ScreenMod {
    pub fn new(
        mut st: St7789<PIO1, SM1, Pin<DynPinId, FunctionPio1, PullDown>>,
        dma_ch0: Channel<CH0>,
        mut timer: CountDown,
        connection_status: &'static ConnectionStatus,
    ) -> Self {
        timer.start(REFRESH_RATE.millis());

        st.backlight_on();

        ScreenMod {
            st,
            dma_ch0,
            timer,
            connection_status,
            #[allow(static_mut_refs)] // This is probably bad. LOL.
            fb: FrameBuf::new(unsafe { &mut FBDATA }, SCR_W, SCR_H),
        }
    }

    fn backlight_off(&mut self) {
        self.st.backlight_off();
    }

    pub fn clear(&mut self) {
        // self.clear_fb();
        // self.push_fb();
        self.backlight_off();
    }

    // fn push_fb(&mut self) {
    //     self.st.push_framebuffer(&self.fb);
    // }

    fn draw_icon(&mut self, icon: &[Bgr565], x: i32, y: i32) {
        let mut h = 0i32;
        while h < 32 {
            let mut w = 0i32;
            while w < 32 {
                let c = icon[(64 * (h * 2) + (w * 2)) as usize];
                let p1 = Point::new(63 - (h * 2) + x, (w * 2) + y);
                let p2 = Point::new(63 - (h * 2) + x - 1, (w * 2) + y);
                let p3 = Point::new(63 - (h * 2) + x, (w * 2) + y + 1);
                let p4 = Point::new(63 - (h * 2) + x - 1, (w * 2) + y + 1);
                self.fb.set_color_at(p1, c);
                self.fb.set_color_at(p2, c);
                self.fb.set_color_at(p3, c);
                self.fb.set_color_at(p4, c);
                w += 1;
            }
            h += 1;
        }
    }

    pub fn update(mut self, keys: [bool; 16], _t: Instant, timer: &Timer) -> Self {
        if !self.timer.wait().is_ok() {
            return self;
        }

        let time_start = timer.get_counter();
        (self.dma_ch0, self.fb) = {
            let (dma_ch0, _, fb) = single_buffer::Config::new(self.dma_ch0, CLEAR_VAL, self.fb)
                .start()
                .wait();

            (dma_ch0, fb)
        };
        let elapse_clear_fb = timer.get_counter() - time_start;

        let time_start = timer.get_counter();
        for y in 0..3 {
            for x in 0..4 {
                self.draw_icon(
                    &ICON_BASE[(y * 4 + x) as usize],
                    2 + (64 + 6) * y,
                    23 + (64 + 6) * x,
                );

                let _ = RoundedRectangle::with_equal_corners(
                    Rectangle::new(
                        Point::new(2 + (64 + 6) * y, 23 + (64 + 6) * x),
                        Size::new(64, 64),
                    ),
                    if keys[(y * 4 + x) as usize] {
                        Size::new(28, 28)
                    } else {
                        Size::new(6, 6)
                    },
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .stroke_color(Bgr565::WHITE)
                        .stroke_width(4)
                        .stroke_alignment(StrokeAlignment::Inside)
                        .build(),
                )
                .draw(&mut self.fb);

                let _ = RoundedRectangle::with_equal_corners(
                    Rectangle::new(
                        Point::new(2 + (64 + 6) * y, 23 + (64 + 6) * x),
                        Size::new(64, 64),
                    ),
                    if keys[(y * 4 + x) as usize] {
                        Size::new(28, 28)
                    } else {
                        Size::new(6, 6)
                    },
                )
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .stroke_color(Bgr565::BLACK)
                        .stroke_width(4)
                        .stroke_alignment(StrokeAlignment::Outside)
                        .build(),
                )
                .draw(&mut self.fb);
            }
        }
        let elapse_draw_icons = timer.get_counter() - time_start;

        let time_start = timer.get_counter();
        (self.st, self.dma_ch0, self.fb) = self.st.push_fb(self.dma_ch0, self.fb);
        let elapse_push_fb = timer.get_counter() - time_start;

        info!(
            "\ntimes:\nclear-fb={}us\ndraw-icons={}ms\npush-fb={}ms\ntotal={}ms",
            elapse_clear_fb.to_micros(),
            elapse_draw_icons.to_millis(),
            elapse_push_fb.to_millis(),
            (elapse_clear_fb + elapse_draw_icons + elapse_push_fb).to_millis(),
        );

        self
    }
}
