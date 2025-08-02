//! Screen for fun graphics

#![allow(dead_code)]

#[allow(unused_imports)]
use defmt::*;

use embedded_dma::Word;
// use embedded_dma::Word;
use embedded_graphics::{
    pixelcolor::{raw::RawU16, Bgr565, Gray4},
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};
use embedded_graphics_framebuf::{backends::FrameBufferBackend, FrameBuf};
use embedded_hal::timer::CountDown as _;
use jukebox_util::{
    peripheral::Connection,
    screen::{ProfileName, ScreenProfile},
    stats::SystemStats,
};
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
use usb_device::device::UsbDeviceState;

use crate::{
    st7789::St7789,
    util::{
        CONNECTION_STATUS, DEFAULT_PROFILE_NAME, DEFAULT_SCREEN_PROFILE, DEFAULT_SYSTEM_STATS,
        ICONS, PROFILE_NAME, SCREEN_CONTROLS, SCREEN_SYSTEM_STATS, USB_STATUS,
    },
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
static FONT2: BitmapFont<'static, Gray4, 1> = mplus!(
    code(100),
    500,
    32,
    false,
    1,
    4,
    '0'..='9',
    ["A", "N", ".", " ", "/"]
);

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
    system_stats: SystemStats,
    usb_state: UsbDeviceState,

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

        let default_screen_profile = DEFAULT_SCREEN_PROFILE.with_lock(|p| p.1.clone());

        ScreenMod {
            st,

            #[allow(static_mut_refs)] // This is probably bad. LOL.
            fb: FrameBuf::new(FBBackEnd {t: unsafe { &mut FBDATA }}, SCR_W, SCR_H),

            profile_name: DEFAULT_PROFILE_NAME,
            screen_profile: default_screen_profile,
            system_stats: DEFAULT_SYSTEM_STATS,
            usb_state: UsbDeviceState::Default,

            dma_ch0,
            timer,

            keys_status: [0; 12],
            keys_previous_frame: [1; 12],
        }
    }

    pub fn clear(&mut self) {
        self.st.set_backlight(0);
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

    fn draw_pre_tick(&mut self, uid: &'static str, ver: &'static str) {
        let c: Bgr565 = {
            let c: RawU16 = self.screen_profile.text_color().into();
            c.into()
        };

        let bgc: Bgr565 = {
            let c: RawU16 = self.screen_profile.background_color().into();
            c.into()
        };

        self.st.set_backlight(self.screen_profile.brightness());

        match self.screen_profile {
            ScreenProfile::Off => {}
            ScreenProfile::DisplayKeys {
                brightness: _,
                background_color: _,
                text_color: _,
                show_profile_name,
            } => {
                let font1_style = BitmapFontStyleBuilder::new()
                    .text_color(c)
                    .background_color(bgc)
                    .font(&FONT1)
                    .build();

                let mut serial_status = Connection::NotConnected(false);
                CONNECTION_STATUS.with_lock(|c| {
                    serial_status = *c;
                });

                if serial_status != Connection::Connected {
                    let _ = Text::with_text_style(
                        uid,
                        Point::new(160 - 1, 224),
                        font1_style.clone(),
                        CENTER_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    let _ = Text::with_text_style(
                        ver,
                        Point::new(255, 224),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    let usb_color = if self.usb_state != UsbDeviceState::Configured {
                        Bgr565::new(8, 16, 31)
                    } else {
                        Bgr565::new(8, 63, 8)
                    };

                    let _ = Rectangle::new(Point::new(26, 224 - 10), Size::new(16, 16))
                        .into_styled(PrimitiveStyle::with_fill(usb_color))
                        .draw(&mut self.fb);

                    self.rounded_rect(bgc, 26, 224 - 10, 16, 1);
                } else {
                    if show_profile_name {
                        let _ = Text::with_text_style(
                            self.profile_name.to_str(),
                            Point::new(160 - 1, 224),
                            font1_style.clone(),
                            CENTER_TEXT_STYLE,
                        )
                        .draw(&mut self.fb);
                    }
                }
            }
            ScreenProfile::DisplayStats {
                brightness: _,
                background_color: _,
                text_color: _,
                show_profile_name,
            } => {
                let font1_style = BitmapFontStyleBuilder::new()
                    .text_color(c)
                    .background_color(bgc)
                    .font(&FONT1)
                    .build();

                let font2_style = BitmapFontStyleBuilder::new()
                    .text_color(c)
                    .background_color(bgc)
                    .font(&FONT2)
                    .build();

                // Left side
                {
                    // Memory Total
                    let _ = Text::with_text_style(
                        self.system_stats.memory_total.to_str(),
                        Point::new(24, 167 + 60),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);
                    let _ = Text::with_text_style(
                        self.system_stats.memory_unit.to_str(),
                        Point::new(66, 167 + 60),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);
                    let _ = Text::with_text_style(
                        " /",
                        Point::new(4, 167 + 60),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    // Memory Used
                    let _ = Text::with_text_style(
                        self.system_stats.memory_used.to_str(),
                        Point::new(4, 142 + 60),
                        font2_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    let _ = Text::with_text_style(
                        "RAM:",
                        Point::new(45, 167 + 10),
                        font1_style.clone(),
                        CENTER_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    // CPU Temperature
                    let _ = Text::with_text_style(
                        self.system_stats.cpu_temperature.to_str(),
                        Point::new(26, 82),
                        font2_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);
                    let _ = Text::with_text_style(
                        "°C",
                        Point::new(108, 86),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    // CPU Usage
                    let _ = Text::with_text_style(
                        self.system_stats.cpu_usage.to_str(),
                        Point::new(26, 42),
                        font2_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);
                    let _ = Text::with_text_style(
                        "%",
                        Point::new(108, 46),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    // CPU name
                    let _ = Text::with_text_style(
                        self.system_stats.cpu_name.to_str(),
                        Point::new(3, 10),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);
                }

                // Right side
                {
                    // VRAM Total
                    let _ = Text::with_text_style(
                        self.system_stats.vram_total.to_str(),
                        Point::new(230 + 24, 167 + 60),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);
                    let _ = Text::with_text_style(
                        self.system_stats.vram_unit.to_str(),
                        Point::new(230 + 66, 167 + 60),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);
                    let _ = Text::with_text_style(
                        " /",
                        Point::new(230 + 4, 167 + 60),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    // VRAM Used
                    let _ = Text::with_text_style(
                        self.system_stats.vram_used.to_str(),
                        Point::new(230 + 4, 142 + 60),
                        font2_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    let _ = Text::with_text_style(
                        "VRAM:",
                        Point::new(230 + 45, 167 + 10),
                        font1_style.clone(),
                        CENTER_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    // GPU Temperature
                    let _ = Text::with_text_style(
                        self.system_stats.gpu_temperature.to_str(),
                        Point::new(160 + 26, 82),
                        font2_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);
                    let _ = Text::with_text_style(
                        "°C",
                        Point::new(160 + 108, 86),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    // GPU Usage
                    let _ = Text::with_text_style(
                        self.system_stats.gpu_usage.to_str(),
                        Point::new(160 + 26, 42),
                        font2_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);
                    let _ = Text::with_text_style(
                        "%",
                        Point::new(160 + 108, 46),
                        font1_style.clone(),
                        LEFT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);

                    // GPU name
                    let _ = Text::with_text_style(
                        self.system_stats.gpu_name.to_str(),
                        Point::new(320 - 2 - 2, 10),
                        font1_style.clone(),
                        RIGHT_TEXT_STYLE,
                    )
                    .draw(&mut self.fb);
                }

                // Profile name
                if show_profile_name {
                    let _ = Text::with_text_style(
                        self.profile_name.to_str(),
                        Point::new(160 - 1, 116),
                        font1_style.clone(),
                        TextStyleBuilder::new()
                            .alignment(Alignment::Center)
                            .baseline(Baseline::Middle)
                            .build(),
                    )
                    .draw(&mut self.fb);
                }
            }
        }
    }

    fn draw_post_tick(&mut self) {
        match self.screen_profile {
            ScreenProfile::Off => {}
            ScreenProfile::DisplayKeys {
                brightness: _,
                background_color: _,
                text_color: _,
                show_profile_name: _,
            } => {
                for y in 0..3 {
                    for x in 0..4 {
                        ICONS.with_mut_lock(|i| {
                            let idx = y * 4 + x;

                            if self.keys_status[idx] == self.keys_previous_frame[idx] && !i[idx].0 {
                                return;
                            }

                            self.draw_icon(
                                &i[idx].1,
                                self.keys_status[idx],
                                23 + (64 + 6) * x,
                                4 + (64 + 6) * y,
                                2,
                            );

                            i[idx].0 = false;
                        });
                    }
                }
            }
            ScreenProfile::DisplayStats {
                brightness: _,
                background_color: _,
                text_color: _,
                show_profile_name: _,
            } => {
                for y in 0..3 {
                    for x in 0..4 {
                        ICONS.with_mut_lock(|i| {
                            let idx = y * 4 + x;

                            if self.keys_status[idx] == self.keys_previous_frame[idx] && !i[idx].0 {
                                return;
                            }

                            self.draw_icon(
                                &i[idx].1,
                                self.keys_status[idx],
                                90 + (32 + 4) * x,
                                130 + (32 + 4) * y,
                                1,
                            );

                            i[idx].0 = false;
                        });
                    }
                }
            }
        }
    }

    pub fn update(
        mut self,
        keys: &[bool],
        _t: &Timer,
        uid: &'static str,
        ver: &'static str,
    ) -> Self {
        for i in 0..12 {
            if keys[i] {
                self.keys_status[i] = 4;
            }
        }

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
        SCREEN_SYSTEM_STATS.with_mut_lock(|p| {
            if p.0 {
                changed = true;
                self.system_stats = p.1.clone();
            }
            p.0 = false;
        });
        USB_STATUS.with_mut_lock(|p| {
            if p.0 {
                changed = true;
                self.usb_state = p.1;
            }
            p.0 = false;
        });

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

            self.draw_pre_tick(uid, ver);
            self.keys_previous_frame = [2; 12];
            self.draw_post_tick();
        }

        // if timer ticks, we need to push frame
        if !self.timer.wait().is_ok() {
            return self;
        }

        self.draw_post_tick();

        // pushing the framebuffer takes, on average, 19.7ms
        // ideally, we would cut this down further, but it may not be possible
        #[allow(static_mut_refs)]
        let (st, dma_ch0) = self.st.push_framebuffer(self.dma_ch0, unsafe { &FBDATA });
        self.st = st;
        self.dma_ch0 = dma_ch0;

        self.keys_previous_frame = self.keys_status;
        for k in self.keys_status.iter_mut() {
            if *k > 0 {
                *k -= 1;
            }
        }

        self
    }
}
