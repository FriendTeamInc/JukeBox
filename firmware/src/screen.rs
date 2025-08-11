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
    pio::InterruptHandler,
    pwm::Pwm,
};
use embassy_time::{Duration, Instant};

type ScrPio = Peri<'static, PIO1>;
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

const POLL_TIME: Duration = Duration::from_millis(100);

struct ScreenMod {
    poll_time: Instant,
}
impl ScreenMod {
    fn new(
        pio: ScrPio,
        dma: ScrDma,
        data_pins: ScrDataPins,
        clk_pin: ScrClkPin,
        cs_pin: ScrCsPin,
        dc_pin: ScrDcPin,
        bl_pin: ScrBlPin,
        rst_pin: ScrRstPin,
    ) -> Self {
        Self {
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

            self.poll_time = unwrap!(now.checked_add(POLL_TIME));
        }
    }
}

#[embassy_executor::task]
pub async fn screen_task(
    pio: ScrPio,
    dma: ScrDma,
    data_pins: ScrDataPins,
    clk_pin: ScrClkPin,
    cs_pin: ScrCsPin,
    dc_pin: ScrDcPin,
    bl_pin: ScrBlPin,
    rst_pin: ScrRstPin,
) -> ! {
    ScreenMod::new(
        pio, dma, data_pins, clk_pin, cs_pin, dc_pin, bl_pin, rst_pin,
    )
    .task()
    .await;
}
