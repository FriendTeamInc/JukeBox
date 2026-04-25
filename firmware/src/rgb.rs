//! RGB
//!
//! For all the pretty lights under the keys

use defmt::*;

use embassy_futures::yield_now;
use embassy_rp::{
    Peri,
    peripherals::{DMA_CH0, PIN_2, PIO0},
    pio::Pio,
    pio_programs::ws2812::{Grb, PioWs2812, PioWs2812Program},
};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant};
use jukebox_util::rgb::{RgbProfile, rgb_brightness};

use crate::{
    usb::usb_suspended,
    util::{DefaultRgbProfileMutex, Irqs, RgbProfileMutex},
};

const POLL_TIME: Duration = Duration::from_millis(10);

pub static RGB_PROFILE: RgbProfileMutex = Mutex::new(RgbProfile::default_device_profile());
pub static DEFAULT_RGB_PROFILE: DefaultRgbProfileMutex =
    Mutex::new((false, RgbProfile::default_device_profile()));

type RgbPio = Peri<'static, PIO0>;
type RgbDma = Peri<'static, DMA_CH0>;
type RgbPin = Peri<'static, PIN_2>;

struct RgbMod {
    ws2812: PioWs2812<'static, PIO0, 0, 12, Grb>,

    brightness: f32,
    brightness_target: f32,

    poll_time: Instant,
}
impl RgbMod {
    fn new(pio: RgbPio, dma: RgbDma, pin: RgbPin) -> Self {
        let Pio {
            mut common, sm0, ..
        } = Pio::new(pio, Irqs);
        let program = PioWs2812Program::new(&mut common);

        Self {
            ws2812: PioWs2812::new(&mut common, sm0, dma, Irqs, pin, &program),

            brightness: 0f32,
            brightness_target: 0f32,

            poll_time: unwrap!(Instant::now().checked_add(POLL_TIME)),
        }
    }

    fn update_brightness(&mut self) {
        let diff = self.brightness_target - self.brightness;
        if diff.abs() < 1.0 {
            self.brightness = self.brightness_target;
        } else {
            self.brightness += diff / 30000.0;
        }
    }

    fn set_brightness_target(&mut self, new_brightness: u8) {
        self.brightness_target = new_brightness as f32;
    }

    async fn task(mut self) -> ! {
        loop {
            self.update_brightness();

            let now = Instant::now();
            if self.poll_time > now {
                yield_now().await;
                // TODO: sleep?
                continue;
            }

            let profile = RGB_PROFILE.lock().await.clone();

            if usb_suspended() {
                self.set_brightness_target(0);
            } else {
                self.set_brightness_target(profile.brightness());
            }

            let b = self.brightness as u8;
            if b == 0 {
                let buffer = RgbProfile::Off.calculate_matrix(0);
                self.ws2812.write(&buffer).await;
            } else {
                let t = Instant::now().as_ticks();
                let buffer = rgb_brightness(profile.calculate_matrix(t), self.brightness as u8);
                self.ws2812.write(&buffer).await;
            }

            self.poll_time = unwrap!(now.checked_add(POLL_TIME));
        }
    }
}

#[embassy_executor::task]
pub async fn rgb_task(pio: RgbPio, dma: RgbDma, pin: RgbPin) -> ! {
    RgbMod::new(pio, dma, pin).task().await;
}
