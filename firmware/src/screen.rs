//! Screen
//!
//! See amazing things

use defmt::*;

use embassy_futures::yield_now;
use embassy_rp::{
    Peri, bind_interrupts,
    gpio::Output,
    peripherals::{
        DMA_CH1, PIN_19, PIN_20, PIN_21, PIN_22, PIN_23, PIN_24, PIN_25, PIN_26, PIN_27, PIO1,
    },
    pio::{Config, InterruptHandler, Pio, StateMachine, program::pio_asm},
    pwm::{Pwm, SetDutyCycle},
};
use embassy_time::{Duration, Instant, Timer};

use crate::usb::usb_suspended;

type ScrPio = Peri<'static, PIO1>;
type ScrPioSm = StateMachine<'static, PIO1, 0>;
type ScrDma = Peri<'static, DMA_CH1>;
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
    dma: ScrDma,
    cs: ScrCsPin,
    dc: ScrDcPin,
    bl: ScrBlPin,
    _rst: ScrRstPin,
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
            mut common,
            mut sm0,
            ..
        } = Pio::new(pio, Irqs);

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
        let datas = [
            &common.make_pio_pin(data.0),
            &common.make_pio_pin(data.1),
            &common.make_pio_pin(data.2),
            &common.make_pio_pin(data.3),
            &common.make_pio_pin(data.4),
            &common.make_pio_pin(data.5),
            &common.make_pio_pin(data.6),
            &common.make_pio_pin(data.7),
        ];
        cfg.set_out_pins(&datas);
        cfg.fifo_join = embassy_rp::pio::FifoJoin::TxOnly;
        cfg.shift_out.threshold = 16;
        cfg.shift_out.direction = embassy_rp::pio::ShiftDirection::Left;
        cfg.shift_out.auto_fill = true;
        cfg.clock_divider = 4u8.into();
        cfg.use_program(&common.load_program(&program.program), &[&clk]);

        sm0.set_config(&cfg);
        sm0.set_pin_dirs(embassy_rp::pio::Direction::Out, &[&clk]);
        sm0.set_pin_dirs(embassy_rp::pio::Direction::Out, &datas);

        sm0.set_enable(true);

        Self {
            sm: sm0,
            dma,
            cs,
            dc,
            bl,
            _rst: rst,
        }
    }

    fn set_backlight(&mut self, percent: u8) {
        self.bl.set_duty_cycle_percent(percent).unwrap();
    }

    async fn set_dc_cs(&mut self, dc: bool, cs: bool) {
        Timer::after_micros(1).await;

        if dc {
            self.dc.set_high();
        } else {
            self.dc.set_low();
        }
        if cs {
            self.cs.set_high();
        } else {
            self.cs.set_low();
        }

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

    async fn start_pixels(&mut self) {
        self.write_cmd(&[0x002C]).await;
        self.set_dc_cs(true, false).await;
    }

    pub async fn push_framebuffer(&mut self, fb: &'static [u16; SCR_W * SCR_H]) {
        self.start_pixels().await;
        self.sm.tx().dma_push(self.dma.reborrow(), fb, false).await;
    }
}

const POLL_TIME: Duration = Duration::from_millis(100);
pub const SCR_W: usize = 320;
pub const SCR_H: usize = 240;
static mut FB: [u16; SCR_W * SCR_H] = [0xFFFF; SCR_W * SCR_H];

struct ScreenMod {
    scr: St7789_8080,
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
    ) -> Self {
        let mut scr = St7789_8080::new(pio, dma, data, clk, cs, dc, bl, rst);

        scr.init(SCR_W as u16, SCR_H as u16).await;

        Self {
            scr,
            poll_time: unwrap!(Instant::now().checked_add(POLL_TIME)),
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

            if !usb_suspended() {
                self.scr
                    .push_framebuffer(unsafe { &mut *core::ptr::addr_of_mut!(FB) })
                    .await;
                self.scr.set_backlight(100);
            } else {
                self.scr.set_backlight(0);
            }

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
) -> ! {
    ScreenMod::new(pio, dma, data, clk, cs, dc, bl, rst)
        .await
        .task()
        .await;
}
