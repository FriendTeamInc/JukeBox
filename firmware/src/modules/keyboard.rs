//! Keyboard processing module

#![allow(dead_code)]

use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_hal::timer::CountDown as _;
use rp2040_hal::{
    fugit::ExtU32,
    gpio::{DynPinId, FunctionSioInput, FunctionSioOutput, Pin, PullDown},
    timer::CountDown,
};

const POLL_RATE: u32 = 5;
pub const KEY_ROWS: usize = 3;
pub const KEY_COLS: usize = 4;

pub struct KeyboardMod {
    col_pins: [Pin<DynPinId, FunctionSioInput, PullDown>; KEY_COLS],
    row_pins: [Pin<DynPinId, FunctionSioOutput, PullDown>; KEY_ROWS],
    poll_timer: CountDown,
    pressed_keys: [bool; 16],
}

impl KeyboardMod {
    pub fn new(
        col_pins: [Pin<DynPinId, FunctionSioInput, PullDown>; KEY_COLS],
        row_pins: [Pin<DynPinId, FunctionSioOutput, PullDown>; KEY_ROWS],
        mut count_down: CountDown,
    ) -> Self {
        count_down.start(POLL_RATE.millis());

        KeyboardMod {
            col_pins: col_pins,
            row_pins: row_pins,
            poll_timer: count_down,
            pressed_keys: [false; 16],
        }
    }

    fn check_pressed_keys(&mut self) {
        let mut keys = [false; 16];

        for row in 0..KEY_ROWS {
            self.row_pins[row].set_high().unwrap();
            nop_loop(30);

            for col in 0..KEY_COLS {
                if self.col_pins[col].is_high().unwrap() {
                    let i = row * KEY_COLS + col;
                    keys[i] = true;
                }
            }

            self.row_pins[row].set_low().unwrap();
        }

        self.pressed_keys = keys;
    }

    pub fn update(&mut self) {
        if !self.poll_timer.wait().is_ok() {
            return;
        }

        self.check_pressed_keys();
    }

    pub fn get_pressed_keys(&self) -> [bool; 16] {
        self.pressed_keys
    }
}

fn nop_loop(n: u8) {
    for _n in 0..n {
        cortex_m::asm::nop();
    }
}
