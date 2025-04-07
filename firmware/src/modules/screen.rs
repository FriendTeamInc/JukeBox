//! Screen for fun graphics

#![allow(dead_code)]

#[allow(unused_imports)]
use defmt::*;

use embedded_dma::Word;
use embedded_graphics::{pixelcolor::Bgr565, prelude::Point};
use embedded_graphics_framebuf::FrameBuf;
use embedded_hal::timer::CountDown as _;
use rp2040_hal::{
    dma::{Channel, CH0},
    fugit::ExtU32,
    gpio::{DynPinId, FunctionPio1, Pin, PullDown},
    pac::PIO1,
    pio::SM1,
    timer::CountDown,
    Timer,
};

use crate::{
    st7789::St7789,
    util::{time_func, ICONS},
};

const REFRESH_RATE: u32 = 33;
pub const SCR_W: usize = 240;
pub const SCR_H: usize = 320;
const BG_COLOR: u16 = 0x1082;
const BG_COLOR_EG: Bgr565 = Bgr565::new(
    (BG_COLOR >> 11) as u8,
    (BG_COLOR >> 5 & 0b111111) as u8,
    (BG_COLOR & 0b11111) as u8,
);
static mut FBDATA: [Bgr565; SCR_W * SCR_H] = [BG_COLOR_EG; SCR_W * SCR_H];
static CLEAR_VAL: RepeatReadTarget<u16> = RepeatReadTarget(BG_COLOR);
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

    fb: FrameBuf<Bgr565, &'static mut [Bgr565; SCR_W * SCR_H]>,

    dma_ch0: Channel<CH0>,
    timer: CountDown,

    keys_status: [u8; 12],
    keys_previous_frame: [u8; 12],
}

impl ScreenMod {
    pub fn new(
        st: St7789<PIO1, SM1, Pin<DynPinId, FunctionPio1, PullDown>>,
        dma_ch0: Channel<CH0>,
        mut timer: CountDown,
    ) -> Self {
        timer.start(REFRESH_RATE.millis());

        ScreenMod {
            st,

            #[allow(static_mut_refs)] // This is probably bad. LOL.
            fb: FrameBuf::new(unsafe { &mut FBDATA }, SCR_W, SCR_H),

            dma_ch0,
            timer,

            keys_status: [0; 12],
            keys_previous_frame: [1; 12],
        }
    }

    pub fn clear(&mut self) {
        self.st.backlight_off();
    }

    fn put_pixel(&mut self, color: Bgr565, x: usize, y: usize) {
        if x >= SCR_W || y >= SCR_H {
            return;
        }

        self.fb.set_color_at(Point::new(x as i32, y as i32), color);
    }

    fn rectangle(&mut self, color: Bgr565, x: usize, y: usize, w: usize, h: usize) {
        for h in 0..h {
            for w in 0..w {
                self.put_pixel(color, x + w, y + h);
            }
        }
    }

    fn rounded_rect(&mut self, color: Bgr565, x: usize, y: usize, s: usize, r: usize) {
        self.rectangle(color, x, y, 2 * r, 2 * r);
        self.rectangle(color, x, y, 4 * r, 1 * r);
        self.rectangle(color, x, y, 1 * r, 4 * r);

        self.rectangle(color, x, y + s - 2 * r, 2 * r, 2 * r);
        self.rectangle(color, x, y + s - 2 * r + 1 * r, 4 * r, 1 * r);
        self.rectangle(color, x, y + s - 2 * r - 2 * r, 1 * r, 4 * r);

        self.rectangle(color, x + s - 2 * r, y, 2 * r, 2 * r);
        self.rectangle(color, x + s - 2 * r - 2 * r, y, 4 * r, 1 * r);
        self.rectangle(color, x + s - 2 * r + 1 * r, y, 1 * r, 4 * r);

        self.rectangle(color, x + s - 2 * r, y + s - 2 * r, 2 * r, 2 * r);
        self.rectangle(
            color,
            x + s - 2 * r - 2 * r,
            y + s - 2 * r + 1 * r,
            4 * r,
            1 * r,
        );
        self.rectangle(
            color,
            x + s - 2 * r + 1 * r,
            y + s - 2 * r - 2 * r,
            1 * r,
            4 * r,
        );
    }

    fn draw_icon(&mut self, icon: &[Bgr565], key: u8, x: usize, y: usize) {
        // icon drawing
        let mut h = 0;
        while h < 32 {
            let mut w = 0;
            while w < 32 {
                let c = icon[32 * h + w];
                self.rectangle(c, 64 - h * 2 + x - 2, w * 2 + y, 2, 2);
                w += 1;
            }
            h += 1;
        }

        // rounded corners
        if key > 0 {
            self.rounded_rect(BG_COLOR_EG, x, y, 64, key as usize);
        } else {
            self.rounded_rect(BG_COLOR_EG, x, y, 64, 1);
        }
    }

    pub fn update(mut self, keys: &[bool], t: &Timer) -> Self {
        for i in 0..12 {
            if keys[i] {
                self.keys_status[i] = 5;
            }
        }

        if !self.timer.wait().is_ok() {
            return self;
        }

        let _elapse_clear_fb = time_func(t, || {
            // // using multiple channels did not meaningfully improve performance.
            // self.dma_ch0 = {
            //     let (dma_ch0, _, _) =
            //         single_buffer::Config::new(self.dma_ch0, CLEAR_VAL, unsafe { &mut FBDATA })
            //             .start()
            //             .wait();
            //     dma_ch0
            // };
        });

        let _elapse_draw_icons = time_func(t, || {
            ICONS.with_mut_lock(|i| {
                for y in 0..3 {
                    for x in 0..4 {
                        let idx = y * 4 + x;

                        if self.keys_status[idx] == self.keys_previous_frame[idx] && !i[idx].0 {
                            continue;
                        }

                        self.draw_icon(
                            &i[idx].1,
                            self.keys_status[idx],
                            4 + (64 + 6) * y,
                            23 + (64 + 6) * x,
                        );

                        i[idx].0 = false;
                    }
                }
            });
        });

        let _elapse_push_fb = time_func(t, || {
            #[allow(static_mut_refs)]
            self.st.push_framebuffer(unsafe { &FBDATA });
            self.st.backlight_on();
        });

        // info!(
        //     "times:\nclear-fb={}us\ndraw-icons={}us\npush-fb={}us\ntotal={}",
        //     _elapse_clear_fb.to_micros(),
        //     _elapse_draw_icons.to_micros(),
        //     _elapse_push_fb.to_micros(),
        //     (_elapse_clear_fb + _elapse_draw_icons + _elapse_push_fb).to_micros()
        // );

        self.keys_previous_frame = self.keys_status;
        for k in self.keys_status.iter_mut() {
            if *k > 0 {
                *k -= 1;
            }
        }

        self
    }
}
