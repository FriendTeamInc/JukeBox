//! Screen for fun graphics

#![allow(dead_code)]

#[allow(unused_imports)]
use defmt::*;

use embedded_dma::Word;
// use embedded_dma::Word;
use embedded_graphics::{
    pixelcolor::{raw::RawU16, Bgr565, Gray4},
    prelude::*,
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};
use embedded_graphics_framebuf::{backends::FrameBufferBackend, FrameBuf};
use embedded_hal::timer::CountDown as _;
use jukebox_util::screen::{ProfileName, ScreenProfile};
use mplusfonts::{mplus, style::BitmapFontStyleBuilder, BitmapFont};
use rp2040_hal::{
    dma::{single_buffer, Channel, CH0},
    fugit::ExtU32,
    gpio::{DynPinId, FunctionPio1, Pin, PullDown},
    pac::PIO1,
    pio::SM1,
    timer::CountDown,
    Timer,
};

use crate::{
    st7789::St7789,
    util::{DEFAULT_PROFILE_NAME, DEFAULT_SCREEN_PROFILE, ICONS, PROFILE_NAME, SCREEN_CONTROLS},
};

const REFRESH_RATE: u32 = 50;
pub const SCR_W: usize = 320;
pub const SCR_H: usize = 240;
static mut FBDATA: [u16; SCR_W * SCR_H] = [0; SCR_W * SCR_H];
struct FBBackEnd {
    t: &'static mut [u16; SCR_W * SCR_H],
}
impl FBBackEnd {
    const fn transpose(idx: usize) -> usize {
        let x = SCR_W - (idx % SCR_W) - 1;
        let y = idx / SCR_W;

        y + x * SCR_H
    }
}
impl FrameBufferBackend for FBBackEnd {
    type Color = Bgr565;

    fn set(&mut self, index: usize, color: Self::Color) {
        self.t[Self::transpose(index)] = color.into_storage();
    }

    fn get(&self, index: usize) -> Self::Color {
        let i: RawU16 = self.t[Self::transpose(index)].into();
        i.into()
    }

    fn nr_elements(&self) -> usize {
        SCR_W * SCR_H
    }
}

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

static FONT1: BitmapFont<'static, Gray4, 1> = mplus!(
    code(100),
    500,
    16,
    false,
    1,
    4,
    '0'..='9',
    'A'..='Z',
    'a'..'z',
    [" ", "-", ".", "%", "°", "/", ":"]
);
static FONT2: BitmapFont<'static, Gray4, 1> =
    mplus!(code(100), 500, 32, false, 1, 4, '0'..='9', [".", " "]);

const LEFT_TEXT_STYLE: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Left)
    .baseline(Baseline::Middle)
    .build();
const CENTER_TEXT_STYLE: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Center)
    .baseline(Baseline::Middle)
    .build();
const RIGHT_TEXT_STYLE: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Right)
    .baseline(Baseline::Middle)
    .build();

pub struct ScreenMod {
    st: St7789<PIO1, SM1, Pin<DynPinId, FunctionPio1, PullDown>>,

    fb: FrameBuf<Bgr565, FBBackEnd>,

    profile_name: ProfileName,
    screen_profile: ScreenProfile,

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
            fb: FrameBuf::new(FBBackEnd {t: unsafe { &mut FBDATA }}, SCR_W, SCR_H),

            profile_name: DEFAULT_PROFILE_NAME,
            screen_profile: DEFAULT_SCREEN_PROFILE,

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

    fn draw_icon(&mut self, icon: &[Bgr565], key: u8, x: usize, y: usize, s: usize) {
        // icon drawing
        let mut h = 0;
        while h < 32 {
            let mut w = 0;
            while w < 32 {
                let c = icon[32 * (31 - w) + (31 - h)];
                self.rectangle(c, 32 * s - h * s + x - s, w * s + y, s, s);
                w += 1;
            }
            h += 1;
        }

        // rounded corners
        let bgc: Bgr565 = {
            let c: RawU16 = self.screen_profile.background_color().into();
            c.into()
        };

        if key > 0 {
            self.rounded_rect(bgc, x, y, 32 * s, key as usize);
        } else {
            self.rounded_rect(bgc, x, y, 32 * s, 1);
        }
    }

    fn draw_pre_tick(&mut self) {
        match self.screen_profile {
            ScreenProfile::Off => {
                self.st.backlight_off();
            }
            ScreenProfile::DisplayKeys {
                brightness: _,
                background_color: _,
                profile_name_color: _,
            } => {
                // TODO: brightness control? via pwm?

                let c: Bgr565 = {
                    let c: RawU16 = self.screen_profile.profile_name_color().into();
                    c.into()
                };

                let bgc: Bgr565 = {
                    let c: RawU16 = self.screen_profile.background_color().into();
                    c.into()
                };

                let font1_style = BitmapFontStyleBuilder::new()
                    .text_color(c)
                    .background_color(bgc)
                    .font(&FONT1)
                    .build();

                let _ = Text::with_text_style(
                    self.profile_name.to_str(),
                    Point::new(160 - 1, 224),
                    font1_style.clone(),
                    CENTER_TEXT_STYLE,
                )
                .draw(&mut self.fb);
            }
            ScreenProfile::DisplayStats {
                brightness: _,
                background_color: _,
                profile_name_color: _,
            } => {
                core::todo!();
            }
        }
    }

    fn draw_post_tick(&mut self) {
        match self.screen_profile {
            ScreenProfile::Off => {}
            ScreenProfile::DisplayKeys {
                brightness: _,
                background_color: _,
                profile_name_color: _,
            } => {
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
                                23 + (64 + 6) * x,
                                4 + (64 + 6) * y,
                                2,
                            );

                            i[idx].0 = false;
                        }
                    }
                });
            }
            ScreenProfile::DisplayStats {
                brightness: _,
                background_color: _,
                profile_name_color: _,
            } => {
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
                                90 + (32 + 4) * x,
                                130 + (32 + 4) * y,
                                1,
                            );

                            i[idx].0 = false;
                        }
                    }
                });

                core::todo!();
            }
        }
    }

    pub fn update(mut self, keys: &[bool], _t: &Timer) -> Self {
        for i in 0..12 {
            if keys[i] {
                self.keys_status[i] = 4;
            }
        }

        // let font2_style =
        // BitmapFontStyleBuilder::new()
        //     .text_color(Bgr565::WHITE)
        //     .background_color(BG_COLOR_EG)
        //     .font(&FONT2)
        //     .build();

        let mut changed = false;
        PROFILE_NAME.with_mut_lock(|p| {
            if p.0 {
                changed = true;
                self.profile_name = p.1.clone();
            }
            p.0 = false;
        });
        SCREEN_CONTROLS.with_mut_lock(|p| {
            if p.0 {
                changed = true;
                self.screen_profile = p.1.clone();
            }
            p.0 = false;
        });

        // if timer ticks, we need to push frame
        if !self.timer.wait().is_ok() {
            if changed {
                // using multiple channels did not meaningfully improve performance.
                // use dma to clear framebuffer
                self.dma_ch0 = {
                    let (dma_ch0, _, _) = single_buffer::Config::new(
                        self.dma_ch0,
                        RepeatReadTarget(self.screen_profile.background_color()),
                        #[allow(static_mut_refs)]
                        unsafe {
                            &mut FBDATA
                        },
                    )
                    .start()
                    .wait();
                    dma_ch0
                };

                self.draw_pre_tick();
            }

            return self;
        }

        self.draw_post_tick();

        // pushing the framebuffer takes, on average, 19.7ms
        // ideally, we would cut this down further, but it may not be possible
        #[allow(static_mut_refs)]
        let (st, dma_ch0) = self.st.push_framebuffer(self.dma_ch0, unsafe { &FBDATA });
        self.st = st;
        self.dma_ch0 = dma_ch0;

        match self.screen_profile {
            ScreenProfile::Off => (),
            _ => self.st.backlight_on(),
        }

        // info!(
        //     "times:\nclear-fb={}us\ndraw-profile-name={}\ndraw-icons={}us\npush-fb={}us\ntotal={}",
        //     _elapse_clear_fb.to_micros(),
        //     _elapse_draw_profile_name.to_micros(),
        //     _elapse_draw_icons.to_micros(),
        //     _elapse_push_fb.to_micros(),
        //     (_elapse_clear_fb + _elapse_draw_profile_name + _elapse_draw_icons + _elapse_push_fb).to_micros()
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
