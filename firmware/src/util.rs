//! Utility functions

use core::cmp::{max, min};

use embassy_rp::spinlock_mutex::SpinlockRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
use jukebox_util::{
    input::InputEvent,
    peripheral::JBInputs,
    rgb::RgbProfile,
    screen::{ProfileName, ScreenProfile},
    stats::SystemStats,
};
use usbd_human_interface_device::{
    device::{keyboard::NKROBootKeyboardReport, mouse::WheelMouseReport},
    page::Keyboard,
};

use crate::{keypad::get_inputs, screen::SCREEN_ICONS, usb::INPUT_EVENTS};

pub fn bootsel() {
    // TODO: make peripherals go dark before rebooting.
    embassy_rp::rom_data::reboot(0x0002, 0, 0x01, 0);
}

// Spinlock Mutexes
// We keep them here so we don't accidentally overlap spinlocks.
pub type IdentifyMutex = Mutex<SpinlockRawMutex<1>, Instant>;

pub type InputEventsMutex = Mutex<SpinlockRawMutex<2>, [InputEvent; 16]>;
pub type DefaultInputEventsMutex = Mutex<SpinlockRawMutex<3>, (bool, [InputEvent; 16])>;
pub type RgbProfileMutex = Mutex<SpinlockRawMutex<4>, RgbProfile>;
pub type DefaultRgbProfileMutex = Mutex<SpinlockRawMutex<5>, (bool, RgbProfile)>;
pub type ScreenProfileMutex = Mutex<SpinlockRawMutex<6>, (bool, ScreenProfile)>;
pub type DefaultScreenProfileMutex = Mutex<SpinlockRawMutex<7>, (bool, ScreenProfile)>;
pub type ScreenProfileNameMutex = Mutex<SpinlockRawMutex<8>, (bool, ProfileName)>;
pub type ScreenSystemStatsMutex = Mutex<SpinlockRawMutex<9>, (bool, SystemStats)>;
pub type ScreenIconsMutex = Mutex<SpinlockRawMutex<10>, [[u16; 32 * 32]; 12]>;

pub async fn get_keyboard_events() -> NKROBootKeyboardReport {
    let mut keys = [Keyboard::NoEventIndicated; 16 * 6];

    let input_events = INPUT_EVENTS.lock().await.clone();
    let mut f = |k: bool, o: usize| match &input_events[o] {
        InputEvent::Keyboard(e) => {
            if k {
                keys[o * 6 + 0] = e.keys[0].into();
                keys[o * 6 + 1] = e.keys[1].into();
                keys[o * 6 + 2] = e.keys[2].into();
                keys[o * 6 + 3] = e.keys[3].into();
                keys[o * 6 + 4] = e.keys[4].into();
                keys[o * 6 + 5] = e.keys[5].into();
            }
        }
        InputEvent::Mouse(_) => {}
    };

    match get_inputs() {
        JBInputs::KeyPad(i) => {
            f(i.key1.is_down(), 0);
            f(i.key2.is_down(), 1);
            f(i.key3.is_down(), 2);
            f(i.key4.is_down(), 3);
            f(i.key5.is_down(), 4);
            f(i.key6.is_down(), 5);
            f(i.key7.is_down(), 6);
            f(i.key8.is_down(), 7);
            f(i.key9.is_down(), 8);
            f(i.key10.is_down(), 9);
            f(i.key11.is_down(), 10);
            f(i.key12.is_down(), 11);
        }
        JBInputs::KnobPad(_i) => defmt::todo!(),
        JBInputs::PedalPad(i) => {
            f(i.left.is_down(), 0);
            f(i.middle.is_down(), 1);
            f(i.right.is_down(), 2);
        }
    }

    NKROBootKeyboardReport::new(keys)
}

pub async fn get_mouse_events() -> WheelMouseReport {
    let mut buttons = 0u8;
    let mut x = 0isize;
    let mut y = 0isize;
    let mut scroll_y = 0isize;
    let mut scroll_x = 0isize;

    let input_events = INPUT_EVENTS.lock().await.clone();
    let mut f = |k: bool, o: usize| match &input_events[o] {
        InputEvent::Keyboard(_) => {}
        InputEvent::Mouse(e) => {
            if k {
                buttons |= e.buttons;
                x += e.x as isize;
                y += e.y as isize;
                scroll_y += e.scroll_y as isize;
                scroll_x += e.scroll_x as isize;
            }
        }
    };

    match get_inputs() {
        JBInputs::KeyPad(i) => {
            f(i.key1.is_down(), 0);
            f(i.key2.is_down(), 1);
            f(i.key3.is_down(), 2);
            f(i.key4.is_down(), 3);
            f(i.key5.is_down(), 4);
            f(i.key6.is_down(), 5);
            f(i.key7.is_down(), 6);
            f(i.key8.is_down(), 7);
            f(i.key9.is_down(), 8);
            f(i.key10.is_down(), 9);
            f(i.key11.is_down(), 10);
            f(i.key12.is_down(), 11);
        }
        JBInputs::KnobPad(_i) => defmt::todo!(),
        JBInputs::PedalPad(i) => {
            f(i.left.is_down(), 0);
            f(i.middle.is_down(), 1);
            f(i.right.is_down(), 2);
        }
    }

    WheelMouseReport {
        buttons: buttons,
        x: min(max(x, i8::MIN as isize), i8::MAX as isize) as i8,
        y: min(max(y, i8::MIN as isize), i8::MAX as isize) as i8,
        vertical_wheel: min(max(scroll_y, i8::MIN as isize), i8::MAX as isize) as i8,
        horizontal_wheel: min(max(scroll_x, i8::MIN as isize), i8::MAX as isize) as i8,
    }
}

macro_rules! load_bmp {
    ($path:literal) => {{
        let (_, bmp) = include_bytes!($path).split_at(0x7A);
        if bmp.len() != (32 * 32 * 2) {
            core::panic!()
        }
        let mut bytes = [0u16; 32 * 32];

        let mut i = 0;
        while i < (32 * 32) {
            bytes[i] = ((bmp[i * 2 + 1] as u16) << 8) | (bmp[i * 2] as u16);
            i += 1;
        }
        bytes
    }};
}

const DEFAULT_ICONS: &[[u16; 32 * 32]] = &[
    load_bmp!("../../assets/action-icons/F13.bmp"),
    load_bmp!("../../assets/action-icons/F14.bmp"),
    load_bmp!("../../assets/action-icons/F15.bmp"),
    load_bmp!("../../assets/action-icons/F16.bmp"),
    load_bmp!("../../assets/action-icons/F17.bmp"),
    load_bmp!("../../assets/action-icons/F18.bmp"),
    load_bmp!("../../assets/action-icons/F19.bmp"),
    load_bmp!("../../assets/action-icons/F20.bmp"),
    load_bmp!("../../assets/action-icons/F21.bmp"),
    load_bmp!("../../assets/action-icons/F22.bmp"),
    load_bmp!("../../assets/action-icons/F23.bmp"),
    load_bmp!("../../assets/action-icons/F24.bmp"),
];

pub async fn reset_icons() {
    let mut icons = SCREEN_ICONS.lock().await;

    let mut i = 0;
    let len = icons.len();
    while i < len {
        let mut y = 0;
        while y < 32 {
            let mut x = 0;
            while x < 32 {
                // TODO: use dma to swap out the icons
                icons[i][32 * y + x] = DEFAULT_ICONS[i][32 * y + x];
                x += 1;
            }
            y += 1;
        }
        i += 1;
    }
}
