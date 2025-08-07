//! JukeBox Async Firmware
//!
//! Built with Embassy

#![no_std]
#![no_main]

mod serial;
mod uid;
mod usb;
mod util;

use crate::serial::serial_task;

use defmt::*;
use {defmt_rtt as _, panic_probe as _};

use embassy_executor::Executor;
use embassy_rp::{
    // gpio::{Input, Level, Output, Pull},
    multicore::{Stack, spawn_core1},
    pwm::{Config, Pwm, SetDutyCycle},
};
use static_cell::StaticCell;

static mut CORE1_STACK: Stack<16384> = Stack::new();
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

    // // Break out pins for peripherals
    // // EEPROM
    // let eeprom_sda = Output::new(p.PIN_4, Level::Low);
    // let eeprom_scl = Output::new(p.PIN_5, Level::Low);
    // // LED
    // let led_pin = Pwm::new_output_b(p.PWM_SLICE6, p.PIN_29, Config::default());
    let led_pin = Pwm::new_output_b(p.PWM_SLICE4, p.PIN_25, Config::default());
    // // Keys
    // let kb_rows = [
    //     Output::new(p.PIN_6, Level::Low),
    //     Output::new(p.PIN_7, Level::Low),
    //     Output::new(p.PIN_8, Level::Low),
    // ];
    // let kb_cols = [
    //     Input::new(p.PIN_9, Pull::Up),
    //     Input::new(p.PIN_10, Pull::Up),
    //     Input::new(p.PIN_11, Pull::Up),
    //     Input::new(p.PIN_12, Pull::Up),
    // ];
    // // RGB
    // let rgb_pin = Output::new(p.PIN_2, Level::Low);
    // // Screen
    // let scr_data = [
    //     Output::new(p.PIN_19, Level::Low),
    //     Output::new(p.PIN_20, Level::Low),
    //     Output::new(p.PIN_21, Level::Low),
    //     Output::new(p.PIN_22, Level::Low),
    //     Output::new(p.PIN_23, Level::Low),
    //     Output::new(p.PIN_24, Level::Low),
    //     Output::new(p.PIN_25, Level::Low),
    //     Output::new(p.PIN_26, Level::Low),
    // ];
    // let scr_clk = Output::new(p.PIN_27, Level::Low);
    // let scr_cs = Output::new(p.PIN_14, Level::Low);
    // let scr_dc = Output::new(p.PIN_15, Level::Low);
    // let scr_bl = Pwm::new_output_a(p.PWM_SLICE0, p.PIN_16, Config::default());
    // let scr_rst = Output::new(p.PIN_13, Level::Low);

    // Set up tasks
    let usb = usb::UsbMod::new(p.USB);
    // TODO!

    // Run all peripherals on core1
    // Peripheral tasks recieve data from the serial task
    // Only the keyboard task sends data to the USB task, for USB HID
    spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        move || {
            let executor1 = EXECUTOR1.init(Executor::new());
            executor1.run(|spawner| {
                unwrap!(spawner.spawn(core1_task(led_pin)));
            });
        },
    );

    // Run all USB and serial processing on core0
    // USB task sends serial data to serial task, and pulls key info from keyboard peripheral for USB HID
    // Serial task processes all commands and sends relevant data to the other core for use
    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        usb.run(&spawner);
        unwrap!(spawner.spawn(serial_task()));
    });
}

#[embassy_executor::task]
async fn core1_task(mut led: Pwm<'static>) {
    info!("Hello from core 1");
    loop {
        let _ = match usb::usb_suspended() {
            false => led.set_duty_cycle_fully_on(),
            true => led.set_duty_cycle_percent(10),
        };
    }
}
