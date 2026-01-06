//! JukeBox Async Firmware
//!
//! Built with Embassy

#![no_std]
#![no_main]

mod eeprom;
mod identify;
mod keypad;
mod rgb;
mod screen;
mod serial;
mod uid;
mod usb;
mod util;

use defmt::*;

use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Executor;
use embassy_rp::{
    gpio,
    multicore::{Stack, spawn_core1},
    pwm,
};
use static_cell::StaticCell;

static mut CORE1_STACK: Stack<40_000> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    // Hello, world!
    let p = embassy_rp::init(Default::default());

    // Set up the UID
    uid::setup_uid();
    {
        info!("Hello, world!");
        let uid = uid::get_uid();
        let ver = env!("CARGO_PKG_VERSION");
        info!("ver:{}, uid:{}", ver, uid);
    }

    // Break out pins for peripherals
    // // EEPROM
    // let eeprom_sda = Output::new(p.PIN_4, Level::Low);
    // let eeprom_scl = Output::new(p.PIN_5, Level::Low);
    // LED
    let led_pin = pwm::Pwm::new_output_b(p.PWM_SLICE6, p.PIN_29, pwm::Config::default());
    // Keypad
    let kp_rows = [
        gpio::Output::new(p.PIN_6, gpio::Level::Low),
        gpio::Output::new(p.PIN_7, gpio::Level::Low),
        gpio::Output::new(p.PIN_8, gpio::Level::Low),
    ];
    let kp_cols = [
        gpio::Input::new(p.PIN_9, gpio::Pull::None),
        gpio::Input::new(p.PIN_10, gpio::Pull::None),
        gpio::Input::new(p.PIN_11, gpio::Pull::None),
        gpio::Input::new(p.PIN_12, gpio::Pull::None),
    ];
    // RGB
    let rgb_pio = p.PIO0;
    let rgb_dma = p.DMA_CH0;
    let rgb_pin = p.PIN_2;
    // Screen
    let scr_pio = p.PIO1;
    let scr_dma = p.DMA_CH1;
    let fb_dma = p.DMA_CH2;
    let scr_data = (
        p.PIN_19, p.PIN_20, p.PIN_21, p.PIN_22, p.PIN_23, p.PIN_24, p.PIN_25, p.PIN_26,
    );
    let scr_clk = p.PIN_27;
    let scr_rd = gpio::Output::new(p.PIN_18, gpio::Level::High);
    let scr_cs = gpio::Output::new(p.PIN_14, gpio::Level::High);
    let scr_dc = gpio::Output::new(p.PIN_15, gpio::Level::Low);
    let scr_bl = pwm::Pwm::new_output_a(p.PWM_SLICE0, p.PIN_16, pwm::Config::default());
    let scr_rst = gpio::Output::new(p.PIN_13, gpio::Level::High);

    // Run all peripherals on core1
    // Peripheral tasks recieve data from the serial task
    // Only the keyboard task sends data to the USB task, for USB HID
    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                unwrap!(spawner.spawn(identify::identify_task(led_pin)));
                unwrap!(spawner.spawn(rgb::rgb_task(rgb_pio, rgb_dma, rgb_pin)));
                unwrap!(spawner.spawn(screen::screen_task(
                    scr_pio, scr_dma, scr_data, scr_clk, scr_rd, scr_cs, scr_dc, scr_bl, scr_rst,
                    fb_dma
                )));
                unwrap!(spawner.spawn(keypad::keypad_task(kp_rows, kp_cols)));
            });
        },
    );

    // Run all USB and serial processing on core0
    // USB task sends serial data to serial task, and pulls key info from keyboard peripheral for USB HID
    // Serial task processes all commands and sends relevant data to the other core for use
    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        unwrap!(usb::usb_task(p.USB, &spawner));
        unwrap!(spawner.spawn(serial::serial_task()));
    });
}
