use core::{
    cmp::{max, min},
    i8,
};

use embedded_graphics::{pixelcolor::Bgr565, prelude::RgbColor};
use jukebox_util::{
    color::split_to_rgb565,
    input::{KeyboardEvent, MouseEvent},
    peripheral::{Connection, JBInputs, KeyInputs, KnobInputs, PedalInputs},
    rgb::RgbProfile,
    screen::{ProfileName, ScreenProfile},
    smallstr::SmallStr,
    stats::SystemStats,
};
use usbd_human_interface_device::{device::mouse::WheelMouseReport, page::Keyboard};

use crate::mutex::Mutex;

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

// inter-core mutexes
type PeripheralInputs = Mutex<1, JBInputs>;
type UpdateTrigger = Mutex<2, bool>;
type IdentifyTrigger = Mutex<3, bool>;
type RgbControls = Mutex<4, (bool, RgbProfile)>; // (changed, settings)
type ConnectionStatus = Mutex<5, Connection>;
type Icons = Mutex<6, [(bool, [Bgr565; 32 * 32]); 12]>;
type KeyboardEvents = Mutex<7, [KeyboardEvent; 12]>;
type MouseEvents = Mutex<8, [MouseEvent; 12]>;
type ProfileNameControl = Mutex<9, (bool, ProfileName)>;
type ScreenSystemStats = Mutex<10, (bool, SystemStats)>;
type ScreenControls = Mutex<11, (bool, ScreenProfile)>;

pub const DEFAULT_INPUTS: JBInputs = inputs_default();
pub const DEFAULT_KEYBOARD_EVENTS: [KeyboardEvent; 12] = KeyboardEvent::default_events();
pub const DEFAULT_MOUSE_EVENTS: [MouseEvent; 12] = MouseEvent::default_events();
pub const DEFAULT_PROFILE_NAME: ProfileName = SmallStr::default();
pub const DEFAULT_RGB_PROFILE: RgbProfile = RgbProfile::default_device_profile();
pub const DEFAULT_SCREEN_PROFILE: ScreenProfile = ScreenProfile::default_profile();
pub const DEFAULT_SYSTEM_STATS: SystemStats = SystemStats::default();

pub static CONNECTION_STATUS: ConnectionStatus = Mutex::new(Connection::NotConnected(true));
pub static PERIPHERAL_INPUTS: PeripheralInputs = Mutex::new(DEFAULT_INPUTS);
pub static UPDATE_TRIGGER: UpdateTrigger = Mutex::new(false);
pub static IDENTIFY_TRIGGER: IdentifyTrigger = Mutex::new(false);
pub static RGB_CONTROLS: RgbControls = Mutex::new((false, DEFAULT_RGB_PROFILE));
pub static ICONS: Icons = Mutex::new([(false, [Bgr565::BLACK; 32 * 32]); 12]);
pub static KEYBOARD_EVENTS: KeyboardEvents = Mutex::new(DEFAULT_KEYBOARD_EVENTS);
pub static MOUSE_EVENTS: MouseEvents = Mutex::new(DEFAULT_MOUSE_EVENTS);
pub static PROFILE_NAME: ProfileNameControl = Mutex::new((true, DEFAULT_PROFILE_NAME));
pub static SCREEN_SYSTEM_STATS: ScreenSystemStats = Mutex::new((false, DEFAULT_SYSTEM_STATS));
pub static SCREEN_CONTROLS: ScreenControls = Mutex::new((false, DEFAULT_SCREEN_PROFILE));

#[macro_export]
macro_rules! time_func {
    ($t:ident, $expr:expr) => {{
        let s = $t.get_counter();
        {
            $expr
        };
        $t.get_counter() - s
    }};
}

pub fn reset_icons() {
    ICONS.with_mut_lock(|icons| {
        let mut i = 0;
        while i < icons.len() {
            let mut y = 0;
            while y < 32 {
                let mut x = 0;
                while x < 32 {
                    // TODO: use dma to swap out the icons
                    let (r, g, b) = split_to_rgb565(!DEFAULT_ICONS[i][32 * y + x]);
                    icons[i].1[32 * y + x] = Bgr565::new(r, g, b);
                    x += 1;
                }
                y += 1;
            }
            icons[i].0 = true;
            i += 1;
        }
    });
}

pub const fn inputs_default() -> JBInputs {
    if cfg!(feature = "keypad") {
        JBInputs::KeyPad(KeyInputs::default())
    } else if cfg!(feature = "knobpad") {
        JBInputs::KnobPad(KnobInputs::default())
    } else if cfg!(feature = "pedalpad") {
        JBInputs::PedalPad(PedalInputs::default())
    } else {
        JBInputs::KeyPad(KeyInputs::default())
    }
}

pub fn reset_peripherals(s: bool) {
    CONNECTION_STATUS.with_mut_lock(|c| *c = Connection::NotConnected(s));
    PROFILE_NAME.with_mut_lock(|p| {
        p.0 = true;
        p.1 = DEFAULT_PROFILE_NAME;
    });
    RGB_CONTROLS.with_mut_lock(|c| {
        c.0 = true;
        c.1 = DEFAULT_RGB_PROFILE;
    });
    SCREEN_CONTROLS.with_mut_lock(|s| {
        s.0 = true;
        s.1 = DEFAULT_SCREEN_PROFILE;
    });
    SCREEN_SYSTEM_STATS.with_mut_lock(|s| {
        s.0 = true;
        s.1 = DEFAULT_SYSTEM_STATS;
    });
    KEYBOARD_EVENTS.with_mut_lock(|e| {
        *e = DEFAULT_KEYBOARD_EVENTS;
    });
    MOUSE_EVENTS.with_mut_lock(|e| {
        *e = DEFAULT_MOUSE_EVENTS;
    });
    reset_icons();
}

pub fn get_keyboard_events() -> [Keyboard; 12 * 6] {
    let mut keys = [Keyboard::NoEventIndicated; 12 * 6];

    KEYBOARD_EVENTS.with_lock(|e| {
        let mut f = |k: bool, o: usize| {
            if k {
                keys[o * 6 + 0] = e[o].keys[0].into();
                keys[o * 6 + 1] = e[o].keys[1].into();
                keys[o * 6 + 2] = e[o].keys[2].into();
                keys[o * 6 + 3] = e[o].keys[3].into();
                keys[o * 6 + 4] = e[o].keys[4].into();
                keys[o * 6 + 5] = e[o].keys[5].into();
            }
        };
        PERIPHERAL_INPUTS.with_lock(|i| match i {
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
            JBInputs::KnobPad(_i) => {
                defmt::todo!();
            }
            JBInputs::PedalPad(i) => {
                f(i.left.is_down(), 0);
                f(i.middle.is_down(), 1);
                f(i.right.is_down(), 2);
            }
        });
    });

    keys
}

pub fn _get_mouse_events() -> WheelMouseReport {
    let mut buttons = 0u8;
    let mut x = 0isize;
    let mut y = 0isize;
    let mut scroll_y = 0isize;
    let mut scroll_x = 0isize;

    MOUSE_EVENTS.with_lock(|e| {
        let mut f = |k: bool, o: usize| {
            if k {
                buttons |= e[o].buttons;
                x += e[o].x as isize;
                y += e[o].y as isize;
                scroll_y += e[o].scroll_y as isize;
                scroll_x += e[o].scroll_x as isize;
            }
        };
        PERIPHERAL_INPUTS.with_lock(|i| match i {
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
            JBInputs::KnobPad(_i) => {
                defmt::todo!();
            }
            JBInputs::PedalPad(i) => {
                f(i.left.is_down(), 0);
                f(i.middle.is_down(), 1);
                f(i.right.is_down(), 2);
            }
        });
    });

    WheelMouseReport {
        buttons: buttons,
        x: min(max(x, i8::MIN as isize), i8::MAX as isize) as i8,
        y: min(max(y, i8::MIN as isize), i8::MAX as isize) as i8,
        vertical_wheel: min(max(scroll_y, i8::MIN as isize), i8::MAX as isize) as i8,
        horizontal_wheel: min(max(scroll_x, i8::MIN as isize), i8::MAX as isize) as i8,
    }
}
