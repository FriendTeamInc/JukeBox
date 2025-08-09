//! Identify
//!
//! Software can trigger this LED to flash, making it easy to identify where
//! your device is.

use defmt::*;

use embassy_futures::yield_now;
use embassy_rp::{
    pwm::{Pwm, SetDutyCycle},
    spinlock_mutex::SpinlockRawMutex,
};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant};

const POLL_TIME: Duration = Duration::from_millis(100);

static IDENTIFY_GOAL: Mutex<SpinlockRawMutex<1>, Instant> = Mutex::new(Instant::MIN);
const BLINK_TIME: Duration = Duration::from_secs(3);
const BLINK_PERIOD: u64 = 1_000_000;

fn pingpong(x: i64) -> u16 {
    let p = BLINK_PERIOD as i64;
    let d = BLINK_PERIOD as f64;
    (((((p / 2) - ((p / 2) - (x % p)).abs()) as f64) / d) * (u16::MAX as f64)) as u16
}

struct IdentifyMod {
    led_pin: Pwm<'static>,
    poll_time: Instant,
}
impl IdentifyMod {
    fn new(led_pin: Pwm<'static>) -> Self {
        Self {
            led_pin,
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

            {
                let g = IDENTIFY_GOAL.lock().await;
                if *g <= now {
                    let t = now.as_ticks();
                    let g = g.as_ticks();
                    self.led_pin
                        .set_duty_cycle(pingpong((g - t) as i64))
                        .unwrap();
                } else {
                    self.led_pin
                        .set_duty_cycle(self.led_pin.max_duty_cycle())
                        .unwrap();
                }
            }

            self.poll_time = unwrap!(Instant::now().checked_add(POLL_TIME));
        }
    }
}

pub async fn start_identify() {
    let mut ig = IDENTIFY_GOAL.lock().await;
    *ig = unwrap!(Instant::now().checked_add(BLINK_TIME));
}

#[embassy_executor::task]
pub async fn identify_task(led_pin: Pwm<'static>) -> ! {
    IdentifyMod::new(led_pin).task().await;
}
