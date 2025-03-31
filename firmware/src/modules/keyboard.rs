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
    pressed_keys: [bool; 16], // TODO: change to KEY_ROWS * KEY_COLS
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

    // pub fn get_default_hardware_inputs() -> [Keyboard; 12 * 6] {
    //     let mut pressed = [Keyboard::NoEventIndicated; 12 * 6];

    //     let keys = [
    //         Keyboard::F13,
    //         Keyboard::F14,
    //         Keyboard::F15,
    //         Keyboard::F16,
    //         Keyboard::F17,
    //         Keyboard::F18,
    //         Keyboard::F19,
    //         Keyboard::F20,
    //         Keyboard::F21,
    //         Keyboard::F22,
    //         Keyboard::F23,
    //         Keyboard::F24,
    //     ];
    //     let mut i = [false; 12];
    //     PERIPHERAL_INPUTS.with_lock(|k| {
    //         match k {
    //             JBInputs::KeyPad(key_inputs) => {
    //                 i[0] = key_inputs.key1.into();
    //                 i[1] = key_inputs.key2.into();
    //                 i[2] = key_inputs.key3.into();
    //                 i[3] = key_inputs.key4.into();
    //                 i[4] = key_inputs.key5.into();
    //                 i[5] = key_inputs.key6.into();
    //                 i[6] = key_inputs.key7.into();
    //                 i[7] = key_inputs.key8.into();
    //                 i[8] = key_inputs.key9.into();
    //                 i[9] = key_inputs.key10.into();
    //                 i[10] = key_inputs.key11.into();
    //                 i[11] = key_inputs.key12.into();
    //             }
    //             _ => todo!(),
    //         };
    //     });
    //     for (i, (k, j)) in keys.iter().zip(i).enumerate() {
    //         if j {
    //             pressed[i] = *k;
    //         }
    //     }

    //     pressed
    // }
}

fn nop_loop(n: u8) {
    for _n in 0..n {
        cortex_m::asm::nop();
    }
}
