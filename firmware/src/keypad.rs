//! Keypad
//!
//! Push buttons get money (maybe)

use core::sync::atomic::AtomicBool;

use defmt::*;

use embassy_futures::yield_now;
use embassy_rp::gpio::{Input, Output};
use embassy_time::{Duration, Instant};
use jukebox_util::peripheral::JBInputs;

static KEYPAD_KEYS: [AtomicBool; 12] = [const { AtomicBool::new(false) }; 12];
pub fn get_raw_inputs() -> [bool; 16] {
    let mut inputs = [false; 16];
    KEYPAD_KEYS.iter().enumerate().for_each(|(i, k)| {
        inputs[i] = k.load(core::sync::atomic::Ordering::Relaxed);
    });
    inputs
}
pub fn get_inputs() -> JBInputs {
    JBInputs::KeyPad(get_raw_inputs().into())
}

const POLL_TIME: Duration = Duration::from_millis(10);

type RowPins = [Output<'static>; 3];
type ColPins = [Input<'static>; 4];

struct KeypadMod {
    row_pins: RowPins,
    col_pins: ColPins,
    poll_time: Instant,
}
impl KeypadMod {
    fn new(row_pins: RowPins, col_pins: ColPins) -> Self {
        Self {
            row_pins,
            col_pins,
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

            for row in 0..3 {
                self.row_pins[row].set_high();
                nop_loop(100);

                for col in 0..4 {
                    let i = row * 4 + col;
                    KEYPAD_KEYS[i].store(
                        self.col_pins[col].is_high(),
                        core::sync::atomic::Ordering::Relaxed,
                    );
                }

                self.row_pins[row].set_low();
            }

            self.poll_time = unwrap!(now.checked_add(POLL_TIME));
        }
    }
}

fn nop_loop(n: u8) {
    for _ in 0..n {
        cortex_m::asm::nop();
    }
}

#[embassy_executor::task]
pub async fn keypad_task(row_pins: RowPins, col_pins: ColPins) -> ! {
    KeypadMod::new(row_pins, col_pins).task().await;
}
