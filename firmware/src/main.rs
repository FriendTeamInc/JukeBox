//! Firmware for JukeBox

#![no_std]
#![no_main]

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [binary_info::EntryAddr; 6] = [
    binary_info::rp_program_name!(c"JukeBox Firmware"),
    binary_info::rp_cargo_version!(),
    binary_info::rp_program_build_attribute!(),
    binary_info::rp_boot2_name(c"boot2_w25q080").addr(),
    binary_info::rp_program_description!(c"Firmware for JukeBox V5."),
    binary_info::rp_program_url!(c"https://jukebox.friendteam.biz"),
];

mutually_exclusive_features::exactly_one_of!("keypad", "knobpad", "pedalpad");

mod mutex;
mod st7789;
mod uid;
mod util;
mod modules {
    pub mod eeprom;
    pub mod keyboard;
    pub mod led;
    pub mod pedals;
    pub mod rgb;
    pub mod screen;
    pub mod serial;
}
use modules::*;

use embedded_hal::timer::CountDown as _;

use rp2040_hal::{
    binary_info,
    clocks::init_clocks_and_plls,
    dma::DMAExt,
    entry,
    fugit::ExtU32,
    fugit::RateExtU32,
    gpio::{FunctionI2C, Pin, Pins},
    multicore::{Multicore, Stack},
    pac::Peripherals,
    pio::PIOExt,
    pwm::Slices,
    rom_data::reset_to_usb_boot,
    sio::Sio,
    usb,
    watchdog::Watchdog,
    Clock, Timer,
};

use usb_device::{class_prelude::*, prelude::*};
use usbd_hid::prelude::*;
use usbd_human_interface_device::{
    self as usbd_hid,
    device::{
        keyboard::NKROBootKeyboard,
        mouse::{WheelMouse, WheelMouseReport},
    },
};
use usbd_serial::SerialPort;

use crate::util::USB_SUSPENDED;
#[allow(unused_imports)]
use defmt::*;

use {defmt_rtt as _, panic_probe as _};

use jukebox_util::peripheral::JBInputs;

static CORE1_STACK: Stack<16384> = Stack::new();

#[entry]
fn main() -> ! {
    // set up hardware interfaces
    let mut pac = Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        12_000_000u32,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();
    let mut sio = Sio::new(pac.SIO);
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let core1 = &mut mc.cores()[1];

    // set up timers
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let mut hid_tick = timer.count_down();
    hid_tick.start(4.millis());
    let mut nkro_tick = timer.count_down();
    nkro_tick.start(1.millis());

    // load unique flash id
    let ver = env!("CARGO_PKG_VERSION");
    let uid = uid::get_flash_uid();
    info!("ver:{}, uid:{}", ver, uid);

    // set up usb
    let usb_bus = UsbBusAllocator::new(usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    let mut usb_hid = UsbHidClassBuilder::new()
        .add_device(usbd_hid::device::keyboard::NKROBootKeyboardConfig::default())
        .add_device(usbd_hid::device::mouse::WheelMouseConfig::default())
        .build(&usb_bus);
    let mut usb_serial = SerialPort::new_with_store(
        &usb_bus,
        [0u8; serial::SERIAL_READ_SIZE],
        [0u8; serial::SERIAL_WRITE_SIZE],
    );
    let usb_pid = if cfg!(feature = "keypad") {
        0xF20A
    } else if cfg!(feature = "knobpad") {
        0xF20B
    } else if cfg!(feature = "pedalpad") {
        0xF20C
    } else {
        0xF209
    };
    let usb_product = if cfg!(feature = "keypad") {
        "JukeBox KeyPad"
    } else if cfg!(feature = "knobpad") {
        "JukeBox KnobPad"
    } else if cfg!(feature = "pedalpad") {
        "JukeBox PedalPad"
    } else {
        "JukeBox Unknown Device"
    };
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, usb_pid))
        .strings(&[StringDescriptors::default()
            .manufacturer("Friend Team Inc.")
            .product(usb_product)
            .serial_number(&uid)])
        .unwrap()
        .max_packet_size_0(64)
        .unwrap()
        .max_power(500)
        .unwrap()
        .supports_remote_wakeup(true)
        .device_release(0x0500)
        .composite_with_iads()
        .build();

    // set up serial module
    let mut serial_mod = serial::SerialMod::new(timer.count_down());
    // set up icon buffers
    util::reset_icons();

    // set up peripherals we need
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let dma = pac.DMA.split(&mut pac.RESETS);
    let pwm_slices = Slices::new(pac.PWM, &mut pac.RESETS);

    // core 1 event loop (GPIO)
    core1
        .spawn(CORE1_STACK.take().unwrap(), move || {
            // set up i2c eeprom defaults
            let mut eeprom_mod = {
                let eeprom_sda: Pin<_, FunctionI2C, _> = pins.gpio4.reconfigure();
                let eeprom_scl: Pin<_, FunctionI2C, _> = pins.gpio5.reconfigure();
                let eeprom_i2c = rp2040_hal::I2C::i2c0(
                    pac.I2C0,
                    eeprom_sda,
                    eeprom_scl,
                    1.MHz(),
                    &mut pac.RESETS,
                    &clocks.system_clock,
                );

                eeprom::EepromMod::new(eeprom_i2c, timer.count_down(), timer.count_down())
            };

            // set up GPIO and modules
            #[cfg(feature = "keypad")]
            let mut keyboard_mod = {
                let kb_col_pins = [
                    pins.gpio12.reconfigure().into_dyn_pin(),
                    pins.gpio13.reconfigure().into_dyn_pin(),
                    pins.gpio14.reconfigure().into_dyn_pin(),
                    pins.gpio15.reconfigure().into_dyn_pin(),
                ];
                let kb_row_pins = [
                    pins.gpio9.reconfigure().into_dyn_pin(),
                    pins.gpio10.reconfigure().into_dyn_pin(),
                    pins.gpio11.reconfigure().into_dyn_pin(),
                ];
                keyboard::KeyboardMod::new(kb_col_pins, kb_row_pins, timer.count_down())
            };

            #[cfg(feature = "keypad")]
            let mut screen_mod = {
                let screen_pins = (
                    pins.gpio21.reconfigure().into_dyn_pin(), // data
                    pins.gpio20.reconfigure().into_dyn_pin(), // clock
                    pins.gpio19.reconfigure().into_dyn_pin(), // cs
                    pins.gpio18.reconfigure().into_dyn_pin(), // dc
                    pins.gpio17.reconfigure().into_dyn_pin(), // rst
                );
                let (mut pio1, _, sm1, _, _) = pac.PIO1.split(&mut pac.RESETS);
                let bl = {
                    let mut pwm = pwm_slices.pwm0;
                    pwm.set_ph_correct();
                    pwm.enable();
                    let mut channel = pwm.channel_a;
                    channel.output_to(pins.gpio16);
                    channel
                };
                let mut st = st7789::St7789::new(
                    &mut pio1,
                    sm1,
                    screen_pins.0,
                    screen_pins.1,
                    screen_pins.2,
                    screen_pins.3,
                    screen_pins.4,
                    bl,
                    timer.count_down(),
                );
                st.init(screen::SCR_W as u16, screen::SCR_H as u16);
                screen::ScreenMod::new(st, dma.ch0, timer.count_down())
            };

            #[cfg(feature = "keypad")]
            let mut rgb_mod = {
                let rgb_pin = pins.gpio2.into_function().into_dyn_pin().into_pull_type();
                let (mut pio0, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
                let ws = ws2812_pio::Ws2812::new(
                    rgb_pin,
                    &mut pio0,
                    sm0,
                    clocks.peripheral_clock.freq(),
                    timer.count_down(),
                );
                rgb::RgbMod::new(ws, timer.count_down())
            };

            #[cfg(feature = "pedalpad")]
            let mut pedal_mod = {
                let pedal_pins = [
                    pins.gpio2.reconfigure().into_dyn_pin(),
                    pins.gpio3.reconfigure().into_dyn_pin(),
                    pins.gpio4.reconfigure().into_dyn_pin(),
                ];
                pedals::PedalMod::new(pedal_pins, timer.count_down())
            };

            let mut led_mod = {
                let mut pwm = pwm_slices.pwm4;
                pwm.set_ph_correct();
                pwm.enable();
                let mut channel = pwm.channel_b;
                channel.output_to(pins.gpio25);
                led::LedMod::new(channel)
            };

            loop {
                // update input devices
                #[cfg(feature = "keypad")]
                keyboard_mod.update();

                #[cfg(feature = "pedalpad")]
                pedal_mod.update();

                // update accessories
                eeprom_mod.update();
                led_mod.update(timer.get_counter());

                #[cfg(feature = "keypad")]
                rgb_mod.update(timer.get_counter());

                #[cfg(feature = "keypad")]
                {
                    screen_mod =
                        screen_mod.update(&keyboard_mod.get_pressed_keys(), &timer, uid, ver);
                }

                // update mutexes
                util::PERIPHERAL_INPUTS.with_mut_lock(|i| {
                    #[cfg(feature = "keypad")]
                    {
                        *i = JBInputs::KeyPad(keyboard_mod.get_pressed_keys().into());
                    }
                    #[cfg(feature = "pedalpad")]
                    {
                        *i = JBInputs::PedalPad(pedal_mod.get_pressed_pedals().into());
                    }
                });

                // check if we need to shutdown "cleanly" for update
                util::UPDATE_TRIGGER.with_lock(|u| {
                    if *u {
                        #[cfg(feature = "keypad")]
                        screen_mod.clear();

                        #[cfg(feature = "keypad")]
                        rgb_mod.clear();

                        led_mod.clear();

                        // wait a few cycles for the IO to finish
                        for _ in 0..100 {
                            cortex_m::asm::nop();
                        }

                        reset_to_usb_boot(0, 1);
                    }
                });
            }
        })
        .expect("failed to start core1");

    // main event loop (USB comms)
    let mut prev_mouse_report = WheelMouseReport::default();
    loop {
        // tick for hid devices
        if hid_tick.wait().is_ok() {
            // handle keyboard
            match usb_hid
                .device::<NKROBootKeyboard<'_, _>, _>()
                .write_report(util::get_keyboard_events())
            {
                Ok(_) => {}
                Err(UsbHidError::Duplicate) => {}
                Err(UsbHidError::WouldBlock) => {}
                Err(e) => {
                    defmt::error!("Failed to write keyboard report: {:?}", e);
                }
            }

            // handle mouse
            let mouse_report = util::get_mouse_events();
            if mouse_report != prev_mouse_report {
                match usb_hid
                    .device::<WheelMouse<'_, _>, _>()
                    .write_report(&mouse_report)
                {
                    Ok(_) => {
                        prev_mouse_report = mouse_report;
                    }
                    Err(UsbHidError::Duplicate) => {}
                    Err(UsbHidError::WouldBlock) => {}
                    Err(e) => {
                        defmt::error!("Failed to write mouse report: {:?}", e);
                    }
                }
            }
        }

        // tick for n-key rollover
        if nkro_tick.wait().is_ok() {
            match usb_hid.tick() {
                Ok(_) => {}
                Err(UsbHidError::WouldBlock) => {}
                Err(e) => {
                    defmt::error!("Failed to process keyboard tick: {:?}", e);
                }
            };
        }

        // update usb devices
        if usb_dev.poll(&mut [&mut usb_hid, &mut usb_serial]) {
            // handle serial
            serial_mod.update(&mut usb_serial, ver, uid);
            // dequeue any incoming reports, so we can process suspend packets properly
            let _ = usb_hid.device::<NKROBootKeyboard<'_, _>, _>().read_report();
        }

        USB_SUSPENDED.store(
            match usb_dev.state() {
                UsbDeviceState::Configured => false,
                _ => true,
            },
            core::sync::atomic::Ordering::Relaxed,
        );
    }
}
