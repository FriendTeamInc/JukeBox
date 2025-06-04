#![allow(dead_code)]

use cortex_m::prelude::_embedded_hal_timer_CountDown;
use embedded_hal::digital::v2::OutputPin as _;
use embedded_hal::PwmPin;
use rp2040_hal::{
    dma::{single_buffer, Channel as DmaChannel, HalfWord, CH0},
    fugit::{ExtU64, MicrosDurationU64},
    gpio::{AnyPin, DynPinId, FunctionSioOutput, Pin, PullDown},
    pio::{PIOBuilder, PIOExt, StateMachineIndex, Tx, UninitStateMachine, PIO},
    pwm::{Channel as PwmChannel, FreeRunning, Pwm0, Slice, A},
    timer::CountDown,
};

use crate::modules::screen::{SCR_H, SCR_W};

// pub const SCR_W: usize = 240;
// pub const SCR_H: usize = 320;
// static mut FB: [[u16; SCR_W]; SCR_H] = [[0x0000u16; SCR_W]; SCR_H];
// The framebuffer is a static so that it does not end up on core1's stack.

pub struct St7789<P, SM, I>
where
    I: AnyPin<Function = P::PinFunction>,
    SM: StateMachineIndex,
    P: PIOExt,
{
    tx: Tx<(P, SM), HalfWord>,
    _data_pin: I,
    _clock_pin: I,
    backlight_pwm: PwmChannel<Slice<Pwm0, FreeRunning>, A>,
    dc_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
    cs_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
    _rst_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
    timer: CountDown,
}

impl<P, SM, I> St7789<P, SM, I>
where
    I: AnyPin<Function = P::PinFunction>,
    P: PIOExt,
    SM: StateMachineIndex,
{
    pub fn new(
        pio: &mut PIO<P>,
        sm: UninitStateMachine<(P, SM)>,
        data_pin: I,
        clock_pin: I,
        mut cs_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
        mut dc_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
        mut rst_pin: Pin<DynPinId, FunctionSioOutput, PullDown>,
        backlight_pwm: PwmChannel<Slice<Pwm0, FreeRunning>, A>,
        timer: CountDown,
    ) -> Self {
        dc_pin.set_low().unwrap();
        cs_pin.set_high().unwrap();
        rst_pin.set_high().unwrap();

        let side_set = pio::SideSet::new(false, 1, false);
        let mut a = pio::Assembler::new_with_side_set(side_set);
        let mut wrap_target = a.label();
        let mut wrap_source = a.label();
        a.bind(&mut wrap_target);
        a.out_with_side_set(pio::OutDestination::PINS, 1, 0);
        a.nop_with_side_set(1);
        a.bind(&mut wrap_source);
        let program = a.assemble_with_wrap(wrap_source, wrap_target);
        let installed = pio.install(&program).unwrap();

        let data_pin = data_pin.into();
        let clock_pin = clock_pin.into();
        let (mut sm, _, tx) = PIOBuilder::from_installed_program(installed)
            // pin config
            .side_set_pin_base(clock_pin.id().num)
            .out_pins(data_pin.id().num, 1)
            // buffer config
            .buffers(rp2040_hal::pio::Buffers::OnlyTx)
            .out_shift_direction(rp2040_hal::pio::ShiftDirection::Left)
            .autopull(true)
            .pull_threshold(16)
            // misc config
            .clock_divisor_fixed_point(1, 0)
            .build(sm);

        sm.set_pindirs([
            (data_pin.id().num, rp2040_hal::pio::PinDir::Output),
            (clock_pin.id().num, rp2040_hal::pio::PinDir::Output),
        ]);

        sm.start();

        Self {
            tx: tx.transfer_size(HalfWord),
            _data_pin: data_pin.into(),
            _clock_pin: clock_pin.into(),
            backlight_pwm,
            dc_pin,
            cs_pin,
            _rst_pin: rst_pin,
            timer,
        }
    }

    pub fn set_backlight(&mut self, duty: u8) {
        // self.backlight_pwm.set_high().unwrap();
        self.backlight_pwm.set_duty((duty as u16) * 655);
    }

    pub fn init(&mut self, w: u16, h: u16) {
        // init sequence
        // 16bit startup sequence
        self.write_cmd(&[0x0001]); // Software reset
        self.write_cmd(&[0x0011]); // Exit sleep mode
        self.write_cmd(&[0x003A, 0x5500]); // Set color mode to 16 bit
        self.write_cmd(&[0b111101_10, 0x0000]); // Set MADCTL: bottom to top, left to right, refresh is bottom to top
        self.write_cmd(&[0x002A, 0x0000, h]); // CASET: column addresses
        self.write_cmd(&[0x002B, 0x0000, w]); // RASET: row addresses
        self.write_cmd(&[0x0021]); // Inversion on (supposedly a hack?)
        self.write_cmd(&[0x0013]); // Normal display on
        self.write_cmd(&[0x0029]); // Main screen turn on
    }

    fn wait_idle(&mut self) {
        self.tx.clear_stalled_flag();
        while !self.tx.has_stalled() {}
    }

    fn sleep(&mut self, t: MicrosDurationU64) {
        self.timer.start(t);
        loop {
            match self.timer.wait() {
                Ok(_) => break,
                Err(_) => {}
            }
        }
    }

    fn write(&mut self, word: u16) {
        while !self.tx.write_u16_replicated(word) {
            cortex_m::asm::nop();
        }
    }

    fn write_cmd(&mut self, cmd: &[u16]) {
        self.wait_idle();
        self.set_dc_cs(false, false);

        self.write(cmd[0]);
        if cmd.len() >= 2 {
            self.wait_idle();
            self.set_dc_cs(true, false);
            for c in &cmd[1..] {
                self.write(*c);
            }
        }

        self.wait_idle();
        self.set_dc_cs(true, true);
    }

    fn set_dc_cs(&mut self, dc: bool, cs: bool) {
        self.sleep(1.micros().into());

        if dc {
            self.dc_pin.set_high().unwrap();
        } else {
            self.dc_pin.set_low().unwrap();
        }
        if cs {
            self.cs_pin.set_high().unwrap();
        } else {
            self.cs_pin.set_low().unwrap();
        }

        self.sleep(1.micros().into());
    }

    fn start_pixels(&mut self) {
        self.write_cmd(&[0x002C]);
        self.set_dc_cs(true, false);
    }

    fn end_pixels(&mut self) {
        self.set_dc_cs(false, false);
    }

    pub fn push_framebuffer(
        mut self,
        dma_ch0: DmaChannel<CH0>,
        fb: &'static [u16; SCR_W * SCR_H],
    ) -> (Self, DmaChannel<CH0>) {
        self.start_pixels();

        let (dma_ch0, tx) = {
            let (dma_ch0, _, tx) = single_buffer::Config::new(dma_ch0, fb, self.tx)
                .start()
                .wait();
            (dma_ch0, tx)
        };

        self.tx = tx;

        (self, dma_ch0)
    }
}
