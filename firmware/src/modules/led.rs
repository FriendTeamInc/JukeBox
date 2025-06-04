//! Blinken Light for debugging module

use core::u16;

use embedded_hal::PwmPin;
use rp2040_hal::{
    fugit::Duration,
    pwm::{Channel, FreeRunning, Pwm4, Slice, B},
    timer::Instant,
};

use crate::util::IDENTIFY_TRIGGER;

const BLINK_TIME: u64 = 3_000_000;
const BLINK_PERIOD: u64 = 1_000_000;
const TIMER_NOM: u32 = 1;
const TIMER_DENOM: u32 = 1_000_000;

fn pingpong(x: i64) -> u16 {
    let p = BLINK_PERIOD as i64;
    let d = BLINK_PERIOD as f64;
    (((((p / 2) - ((p / 2) - (x % p)).abs()) as f64) / d) * (u16::MAX as f64)) as u16
}

pub struct LedMod {
    led_pwm: Channel<Slice<Pwm4, FreeRunning>, B>,
    blink_goal: Option<u64>,
}

impl LedMod {
    pub fn new(led_pwm: Channel<Slice<Pwm4, FreeRunning>, B>) -> Self {
        LedMod {
            led_pwm,
            blink_goal: None,
        }
    }

    pub fn clear(&mut self) {
        self.led_pwm.set_duty(0);
    }

    pub fn update(&mut self, t: Instant) {
        let t = t.duration_since_epoch();

        IDENTIFY_TRIGGER.with_mut_lock(|i| {
            if *i {
                *i = false;
                self.blink_goal = Some(
                    t.clone()
                        .checked_add(Duration::<u64, TIMER_NOM, TIMER_DENOM>::from_ticks(
                            BLINK_TIME,
                        ))
                        .unwrap()
                        .ticks(),
                );
            }
        });

        if let Some(g) = self.blink_goal {
            let t = t.ticks();
            if t >= g {
                self.blink_goal = None;
                return;
            }
            self.led_pwm.set_duty(pingpong(t as i64));
        } else {
            self.clear();
        }
    }
}
