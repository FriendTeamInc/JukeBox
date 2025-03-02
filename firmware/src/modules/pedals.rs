//! Pedal processing module

#![allow(dead_code)]

use embedded_hal::digital::v2::InputPin;
use embedded_hal::timer::CountDown as _;
use rp2040_hal::{
    fugit::ExtU32,
    gpio::{DynPinId, FunctionSioInput, Pin, PullUp},
    timer::CountDown,
};

const POLL_RATE: u32 = 5;
pub const PEDAL_COUNT: usize = 3;

pub struct PedalMod {
    pedal_pins: [Pin<DynPinId, FunctionSioInput, PullUp>; PEDAL_COUNT],
    poll_timer: CountDown,
    pressed_pedals: [bool; PEDAL_COUNT],
}

impl PedalMod {
    pub fn new(
        pedal_pins: [Pin<DynPinId, FunctionSioInput, PullUp>; PEDAL_COUNT],
        mut count_down: CountDown,
    ) -> Self {
        count_down.start(POLL_RATE.millis());

        PedalMod {
            pedal_pins: pedal_pins,
            poll_timer: count_down,
            pressed_pedals: [false; PEDAL_COUNT],
        }
    }

    fn check_pressed_pedals(&mut self) {
        for p in 0..PEDAL_COUNT {
            self.pressed_pedals[p] = self.pedal_pins[p].is_low().unwrap_or(false);
        }
    }

    pub fn update(&mut self) {
        if !self.poll_timer.wait().is_ok() {
            return;
        }

        self.check_pressed_pedals();
    }

    pub fn get_pressed_pedals(&self) -> [bool; PEDAL_COUNT] {
        self.pressed_pedals
    }
}
