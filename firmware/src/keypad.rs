//! Keypad
//!
//!

use core::sync::atomic::AtomicBool;

use defmt::*;

use embassy_futures::yield_now;
use embassy_rp::gpio::{Input, Output};
use embassy_time::{Duration, Instant};

pub static KEYPAD_KEYS: [AtomicBool; 12] = [const { AtomicBool::new(false) }; 12];

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
            if self.poll_time < now {
                yield_now().await;
                // TODO: sleep?
                continue;
            }

            for row in 0..3 {
                self.row_pins[row].set_high();
                nop_loop(30);

                for col in 0..4 {
                    let i = row * 3 + col;
                    if self.col_pins[col].is_high() {
                        KEYPAD_KEYS[i].store(true, core::sync::atomic::Ordering::Relaxed);
                    } else {
                        KEYPAD_KEYS[i].store(false, core::sync::atomic::Ordering::Relaxed);
                    }
                }

                self.row_pins[row].set_low();
            }

            self.poll_time = unwrap!(Instant::now().checked_add(POLL_TIME));
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
