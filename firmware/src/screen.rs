//! Screen
//!
//! See amazing things

use defmt::*;

use embassy_futures::yield_now;
use embassy_rp::{
    Peri, bind_interrupts,
    dma::write_repeated,
    gpio::Output,
    peripherals::{
        DMA_CH1, DMA_CH2, PIN_19, PIN_20, PIN_21, PIN_22, PIN_23, PIN_24, PIN_25, PIN_26, PIN_27,
        PIO1,
    },
    pio::{
        Common, Config, InterruptHandler, LoadedProgram, Pin, Pio, StateMachine, program::pio_asm,
    },
    pwm::{Pwm, SetDutyCycle},
};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{
    pixelcolor::{Bgr565, Gray4, raw::RawU16},
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text, TextStyle},
};
use embedded_graphics::{prelude::*, text::TextStyleBuilder};
use embedded_graphics_framebuf::{FrameBuf, backends::FrameBufferBackend};
use jukebox_util::{
    screen::{ProfileName, ScreenProfile},
    stats::SystemStats,
};
use mplusfonts::{BitmapFont, mplus, style::BitmapFontStyleBuilder};

use crate::{
    keypad::get_raw_inputs,
    serial::SERIAL_CONNECTED,
    uid::get_uid,
    usb::usb_suspended,
    util::{
        DefaultScreenProfileMutex, ScreenIconsMutex, ScreenProfileMutex, ScreenProfileNameMutex,
        ScreenSystemStatsMutex,
    },
};

type ScrPio = Peri<'static, PIO1>;
// type ScrPioCommon = Common<'static, PIO1>;
// type ScrPioProgram = LoadedProgram<'static, PIO1>;
type ScrPioSm = StateMachine<'static, PIO1, 0>;
type ScrDma = Peri<'static, DMA_CH1>;
type FbDma = Peri<'static, DMA_CH2>;
type ScrDataPins = (
    Peri<'static, PIN_19>,
    Peri<'static, PIN_20>,
    Peri<'static, PIN_21>,
    Peri<'static, PIN_22>,
    Peri<'static, PIN_23>,
    Peri<'static, PIN_24>,
    Peri<'static, PIN_25>,
    Peri<'static, PIN_26>,
);
// type ScrPioPins = (
//     Pin<'static, PIO1>,
//     Pin<'static, PIO1>,
//     Pin<'static, PIO1>,
//     Pin<'static, PIO1>,
//     Pin<'static, PIO1>,
//     Pin<'static, PIO1>,
//     Pin<'static, PIO1>,
//     Pin<'static, PIO1>,
// );

type ScrClkPin = Peri<'static, PIN_27>;
type ScrCsPin = Output<'static>;
type ScrDcPin = Output<'static>;
type ScrBlPin = Pwm<'static>;
type ScrRstPin = Output<'static>;

bind_interrupts!(struct Irqs {
    PIO1_IRQ_0 => InterruptHandler<PIO1>;
});

struct St7789_8080 {
    sm: ScrPioSm,
    // common: ScrPioCommon,
    // program: ScrPioProgram,
    // data: ScrPioPins,
    dma: ScrDma,
    cs: ScrCsPin,
    dc: ScrDcPin,
    bl: ScrBlPin,
    // rst: ScrRstPin,
}
impl St7789_8080 {
    pub fn new(
        pio: ScrPio,
        dma: ScrDma,
        data: ScrDataPins,
        clk: ScrClkPin,
        mut cs: ScrCsPin,
        mut dc: ScrDcPin,
        mut bl: ScrBlPin,
        mut rst: ScrRstPin,
    ) -> Self {
        bl.set_duty_cycle_percent(0).unwrap();
        dc.set_low();
        cs.set_high();
        rst.set_high();

        let Pio {
            mut common, sm0, ..
        } = Pio::new(pio, Irqs);
        let mut sm = sm0;

        let program = pio_asm!(
            // ".pio_version 1"
            // ".program st7789_8080"
            ".side_set 1"
            ".wrap_target"
            "out pins, 8 side 0"
            "nop side 1"
            ".wrap"
        );

        let mut cfg = Config::default();
        let clk = common.make_pio_pin(clk);
        let data = (
            common.make_pio_pin(data.0),
            common.make_pio_pin(data.1),
            common.make_pio_pin(data.2),
            common.make_pio_pin(data.3),
            common.make_pio_pin(data.4),
            common.make_pio_pin(data.5),
            common.make_pio_pin(data.6),
            common.make_pio_pin(data.7),
        );

        let pins = [
            &data.0, &data.1, &data.2, &data.3, &data.4, &data.5, &data.6, &data.7,
        ];
        cfg.set_out_pins(&pins);
        cfg.fifo_join = embassy_rp::pio::FifoJoin::TxOnly;
        cfg.shift_out.threshold = 16;
        cfg.shift_out.direction = embassy_rp::pio::ShiftDirection::Left;
        cfg.shift_out.auto_fill = true;
        cfg.clock_divider = 4u8.into();
        let program = common.load_program(&program.program);
        cfg.use_program(&program, &[&clk]);

        sm.set_config(&cfg);
        sm.set_pin_dirs(embassy_rp::pio::Direction::Out, &pins);
        sm.set_pin_dirs(embassy_rp::pio::Direction::Out, &[&clk]);
        sm.set_enable(true);

        Self {
            sm,
            // common,
            // program,
            // data,
            dma,
            cs,
            dc,
            bl,
            // rst,
        }
    }

    fn set_backlight(&mut self, percent: u8) {
        self.bl.set_duty_cycle_percent(percent).unwrap();
    }

    async fn set_dc_cs(&mut self, dc: bool, cs: bool) {
        // Timer::after_micros(1).await;
        self.dc.set_level(dc.into());
        self.cs.set_level(cs.into());
        Timer::after_micros(1).await;
    }

    async fn write(&mut self, word: u16) {
        self.sm.tx().wait_push((word as u32) << 16).await;
    }

    async fn wait_idle(&mut self) {
        while !self.sm.tx().stalled() {
            yield_now().await;
        }
    }

    async fn write_cmd(&mut self, cmd: &[u16]) {
        self.wait_idle().await;
        self.set_dc_cs(false, false).await;

        self.write(cmd[0]).await;
        if cmd.len() >= 2 {
            self.wait_idle().await;
            self.set_dc_cs(true, false).await;
            for c in &cmd[1..] {
                self.write(*c).await;
            }
        }

        self.wait_idle().await;
        self.set_dc_cs(true, true).await;
    }

    pub async fn init(&mut self, w: u16, h: u16) {
        // init sequence
        // 16bit startup sequence
        self.write_cmd(&[0x0001]).await; // Software reset
        self.write_cmd(&[0x0011]).await; // Exit sleep mode
        self.write_cmd(&[0x003A, 0x5500]).await; // Set color mode to 16 bit
        self.write_cmd(&[0x0036, 0x0000]).await; // Set MADCTL: bottom to top, left to right, refresh is bottom to top // 0b111101_10
        self.write_cmd(&[0x002A, 0x0000, h]).await; // CASET: column addresses
        self.write_cmd(&[0x002B, 0x0000, w]).await; // RASET: row addresses
        self.write_cmd(&[0x0021]).await; // Inversion on
        self.write_cmd(&[0x0013]).await; // Normal display on
        self.write_cmd(&[0x0029]).await; // Main screen turn on
    }

    pub async fn push_framebuffer(&mut self, fb: &'static [u16]) {
        self.write_cmd(&[0x002C]).await;
        self.set_dc_cs(true, false).await;
        self.sm.tx().dma_push(self.dma.reborrow(), fb, true).await;
    }
}

pub static SCREEN_PROFILE: ScreenProfileMutex = Mutex::new(ScreenProfile::default_profile());
pub static DEFAULT_SCREEN_PROFILE: DefaultScreenProfileMutex =
    Mutex::new((false, ScreenProfile::default_profile()));
pub static SCREEN_PROFILE_NAME: ScreenProfileNameMutex = Mutex::new(ProfileName::default());
pub static SCREEN_SYSTEM_STATS: ScreenSystemStatsMutex = Mutex::new(SystemStats::default());
pub static SCREEN_ICONS: ScreenIconsMutex = Mutex::new([[0u16; 32 * 32]; 12]);

const POLL_TIME: Duration = Duration::from_millis(100);
pub const SCR_W: usize = 320;
pub const SCR_H: usize = 240;
static mut FBDATA: [u16; SCR_W * SCR_H] = [0xFFFF; SCR_W * SCR_H];
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
    'A'..='Z',
    'a'..'z',
    [" ", "-", ".", "%", "°", "/", ":"]
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

struct ScreenMod {
    scr: St7789_8080,

    fb_dma: FbDma,
    fb: FrameBuf<Bgr565, FBBackEnd>,

    screen_profile: ScreenProfile,
    screen_profile_name: ProfileName,
    screen_system_stats: SystemStats,

    keys_status: [u8; 12],

    poll_time: Instant,
}
impl ScreenMod {
    async fn new(
        pio: ScrPio,
        dma: ScrDma,
        data: ScrDataPins,
        clk: ScrClkPin,
        cs: ScrCsPin,
        dc: ScrDcPin,
        bl: ScrBlPin,
        rst: ScrRstPin,
        fb_dma: FbDma,
    ) -> Self {
        let mut scr = St7789_8080::new(pio, dma, data, clk, cs, dc, bl, rst);

        scr.init(SCR_W as u16, SCR_H as u16).await;

        Self {
            scr,

            fb_dma,
            fb: FrameBuf::new(
                FBBackEnd {
                    t: unsafe { &mut *core::ptr::addr_of_mut!(FBDATA) },
                },
                SCR_W,
                SCR_H,
            ),

            screen_profile: ScreenProfile::default_profile(),
            screen_profile_name: ProfileName::default(),
            screen_system_stats: SystemStats::default(),

            keys_status: [0; 12],

            poll_time: unwrap!(Instant::now().checked_add(POLL_TIME)),
        }
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

    fn draw_icon(&mut self, icon: &[u16], key: u8, x: usize, y: usize, s: usize) {
        // icon drawing
        let mut h = 0;
        while h < 32 {
            let mut w = 0;
            while w < 32 {
                let c = RawU16::new(icon[32 * (31 - w) + (31 - h)]).into();
                self.rectangle(c, 32 * s - h * s + x - s, w * s + y, s, s);
                w += 1;
            }
            h += 1;
        }

        // rounded corners
        let bgc = RawU16::new(self.screen_profile.background_color()).into();

        if key > 0 {
            self.rounded_rect(bgc, x, y, 32 * s, key as usize);
        } else {
            self.rounded_rect(bgc, x, y, 32 * s, 1);
        }
    }

    async fn draw_system_stats(&mut self) {
        let text_color: Bgr565 = RawU16::new(self.screen_profile.text_color()).into();
        let bgc: Bgr565 = RawU16::new(self.screen_profile.background_color()).into();

        let font1_style = BitmapFontStyleBuilder::new()
            .text_color(text_color)
            .background_color(bgc)
            .font(&FONT1)
            .build();

        let font2_style = BitmapFontStyleBuilder::new()
            .text_color(text_color)
            .background_color(bgc)
            .font(&FONT2)
            .build();

        // Left side
        {
            // Memory Total
            let _ = Text::with_text_style(
                self.screen_system_stats.memory_total.to_str(),
                Point::new(24, 167 + 60),
                font1_style.clone(),
                LEFT_TEXT_STYLE,
            )
            .draw(&mut self.fb);
            let _ = Text::with_text_style(
                self.screen_system_stats.memory_unit.to_str(),
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
                self.screen_system_stats.memory_used.to_str(),
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
                self.screen_system_stats.cpu_temperature.to_str(),
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
                self.screen_system_stats.cpu_usage.to_str(),
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
                self.screen_system_stats.cpu_name.to_str(),
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
                self.screen_system_stats.vram_total.to_str(),
                Point::new(230 + 24, 167 + 60),
                font1_style.clone(),
                LEFT_TEXT_STYLE,
            )
            .draw(&mut self.fb);
            let _ = Text::with_text_style(
                self.screen_system_stats.vram_unit.to_str(),
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
                self.screen_system_stats.vram_used.to_str(),
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
                self.screen_system_stats.gpu_temperature.to_str(),
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
                self.screen_system_stats.gpu_usage.to_str(),
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
                self.screen_system_stats.gpu_name.to_str(),
                Point::new(320 - 2 - 2, 10),
                font1_style.clone(),
                RIGHT_TEXT_STYLE,
            )
            .draw(&mut self.fb);
        }
    }

    async fn draw(&mut self) {
        let text_color: Bgr565 = RawU16::new(self.screen_profile.text_color()).into();
        let bgc: Bgr565 = RawU16::new(self.screen_profile.background_color()).into();
        let show_profile_name = self.screen_profile.show_profile_name();

        let font1_style = BitmapFontStyleBuilder::new()
            .text_color(text_color)
            .background_color(bgc)
            .font(&FONT1)
            .build();

        match self.screen_profile {
            ScreenProfile::Off => {}
            ScreenProfile::DisplayKeys { .. } => {
                let serial_connected = SERIAL_CONNECTED.load(core::sync::atomic::Ordering::Relaxed);

                // if serial_connected {
                //     if show_profile_name {
                //         let _ = Text::with_text_style(
                //             self.screen_profile_name.to_str(),
                //             Point::new(160 - 1, 224),
                //             font1_style.clone(),
                //             CENTER_TEXT_STYLE,
                //         )
                //         .draw(&mut self.fb);
                //     }
                // } else {
                //     let _ = Text::with_text_style(
                //         get_uid(),
                //         Point::new(160 - 1, 224),
                //         font1_style.clone(),
                //         CENTER_TEXT_STYLE,
                //     )
                //     .draw(&mut self.fb);

                //     let _ = Text::with_text_style(
                //         env!("CARGO_PKG_VERSION"),
                //         Point::new(255, 224),
                //         font1_style.clone(),
                //         LEFT_TEXT_STYLE,
                //     )
                //     .draw(&mut self.fb);
                // }

                for y in 0..3 {
                    for x in 0..4 {
                        let i = SCREEN_ICONS.lock().await;
                        let idx = y * 4 + x;
                        self.draw_icon(
                            &i[idx],
                            self.keys_status[idx],
                            23 + (64 + 6) * x,
                            4 + (64 + 6) * y,
                            2,
                        );
                    }
                }
            }
            ScreenProfile::DisplayStats { .. } => {
                self.draw_system_stats().await;

                // Profile name
                if show_profile_name {
                    let _ = Text::with_text_style(
                        self.screen_profile_name.to_str(),
                        Point::new(160 - 1, 116),
                        font1_style.clone(),
                        TextStyleBuilder::new()
                            .alignment(Alignment::Center)
                            .baseline(Baseline::Middle)
                            .build(),
                    )
                    .draw(&mut self.fb);
                }

                for y in 0..3 {
                    for x in 0..4 {
                        let i = SCREEN_ICONS.lock().await;
                        let idx = y * 4 + x;
                        self.draw_icon(
                            &i[idx],
                            self.keys_status[idx],
                            90 + (32 + 4) * x,
                            130 + (32 + 4) * y,
                            1,
                        );
                    }
                }
            }
        }
    }

    async fn task(mut self) -> ! {
        loop {
            let now = Instant::now();
            if self.poll_time > now {
                yield_now().await;
                // TODO: sleep?
                continue;
            }

            // Before we do anything, we exit early if the device is supposed to be asleep
            if usb_suspended() {
                // self.scr.set_backlight(0);
                yield_now().await;
                continue;
            }

            let start_draw = Instant::now();

            // First, clone out all the data we need to build the screen frame and update some animations
            self.screen_profile = SCREEN_PROFILE.lock().await.clone();
            self.screen_profile_name = SCREEN_PROFILE_NAME.lock().await.clone();
            self.screen_system_stats = SCREEN_SYSTEM_STATS.lock().await.clone();
            let keys = get_raw_inputs();
            for i in 0..12 {
                if keys[i] {
                    self.keys_status[i] = 4;
                } else {
                    self.keys_status[i] -= 1;
                }
            }

            // Second, clear the screen to the background color using a DMA
            // TODO: We actually can't do the background color right now because of embassy limitations...
            // unsafe {
            //     write_repeated(
            //         self.fb_dma.reborrow(),
            //         &mut *core::ptr::addr_of_mut!(FBDATA[0]),
            //         SCR_W * SCR_H,
            //         embassy_rp::pac::dma::vals::TreqSel::PERMANENT,
            //     )
            //     .await;
            // };
            // TODO: dont do this.
            let _ = Rectangle::new(Point::new(0, 0), Size::new(SCR_W as u32, SCR_H as u32))
                .into_styled(PrimitiveStyle::with_fill(
                    RawU16::new(self.screen_profile.background_color()).into(),
                ))
                .draw(&mut self.fb);

            // Third, draw everything on the screen
            self.draw().await;

            // Finally, push the frame to the screen using the driver
            self.scr
                .push_framebuffer(unsafe { &mut *core::ptr::addr_of_mut!(FBDATA) })
                .await;
            self.scr.set_backlight(self.screen_profile.brightness());

            let end_draw = Instant::now();
            info!("draw_time: {} us", (end_draw - start_draw).as_micros());

            self.poll_time = unwrap!(now.checked_add(POLL_TIME));
        }
    }
}

#[embassy_executor::task]
pub async fn screen_task(
    pio: ScrPio,
    dma: ScrDma,
    data: ScrDataPins,
    clk: ScrClkPin,
    cs: ScrCsPin,
    dc: ScrDcPin,
    bl: ScrBlPin,
    rst: ScrRstPin,
    fb_dma: FbDma,
) -> ! {
    ScreenMod::new(pio, dma, data, clk, cs, dc, bl, rst, fb_dma)
        .await
        .task()
        .await;
}
