use core::{
    cmp::{max, min},
    i8,
};

use embedded_graphics::{pixelcolor::Bgr565, prelude::RgbColor};
use jukebox_util::{
    color::split_to_rgb565,
    input::InputEvent,
    peripheral::{Connection, JBInputs, KeyInputs, KnobInputs, PedalInputs},
    rgb::RgbProfile,
    screen::{ProfileName, ScreenProfile},
    smallstr::SmallStr,
    stats::SystemStats,
};
// use rp2040_hal::dma::{single_buffer, Channel, CH1};
use usb_device::device::UsbDeviceState;
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
type InputEvents = Mutex<7, [InputEvent; 16]>;
// type KeyboardEvents = Mutex<7, [KeyboardEvent; 12]>;
// type MouseEvents = Mutex<8, [MouseEvent; 12]>;
type ProfileNameControl = Mutex<9, (bool, ProfileName)>;
type ScreenSystemStats = Mutex<10, (bool, SystemStats)>;
type ScreenControls = Mutex<11, (bool, ScreenProfile)>;
type UsbStatus = Mutex<12, UsbDeviceState>;

type DefaultInputEvents = Mutex<13, (bool, [InputEvent; 16])>;
type DefaultRgbProfile = Mutex<14, (bool, RgbProfile)>;
type DefaultScreenProfile = Mutex<15, (bool, ScreenProfile)>;

pub const DEFAULT_INPUTS: JBInputs = inputs_default();
pub const DEFAULT_PROFILE_NAME: ProfileName = SmallStr::default();
pub const DEFAULT_SYSTEM_STATS: SystemStats = SystemStats::default();

pub static DEFAULT_INPUT_EVENTS: DefaultInputEvents =
    Mutex::new((false, InputEvent::default_all()));
pub static DEFAULT_RGB_PROFILE: DefaultRgbProfile =
    Mutex::new((false, RgbProfile::default_device_profile()));
pub static DEFAULT_SCREEN_PROFILE: DefaultScreenProfile =
    Mutex::new((false, ScreenProfile::default_profile()));

pub static CONNECTION_STATUS: ConnectionStatus = Mutex::new(Connection::NotConnected(true));
pub static PERIPHERAL_INPUTS: PeripheralInputs = Mutex::new(DEFAULT_INPUTS);
pub static UPDATE_TRIGGER: UpdateTrigger = Mutex::new(false);
pub static IDENTIFY_TRIGGER: IdentifyTrigger = Mutex::new(false);
pub static RGB_CONTROLS: RgbControls = Mutex::new((false, RgbProfile::default_device_profile()));
pub static ICONS: Icons = Mutex::new([(false, [Bgr565::BLACK; 32 * 32]); 12]);
pub static INPUT_EVENTS: InputEvents = Mutex::new(InputEvent::default_all());
pub static PROFILE_NAME: ProfileNameControl = Mutex::new((true, DEFAULT_PROFILE_NAME));
pub static SCREEN_SYSTEM_STATS: ScreenSystemStats = Mutex::new((false, DEFAULT_SYSTEM_STATS));
pub static SCREEN_CONTROLS: ScreenControls = Mutex::new((false, ScreenProfile::default_profile()));
pub static USB_STATUS: UsbStatus = Mutex::new(UsbDeviceState::Default);

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
        // let mut dma_ch1 = dma_ch1;
        let len = icons.len();
        while i < len {
            icons[i].0 = true;
            // dma_ch1 = single_buffer::Config::new(dma_ch1, &DEFAULT_ICONS[i], &mut icons[i].1)
            //     .start()
            //     .wait()
            //     .0;

            let mut y = 0;
            while y < 32 {
                let mut x = 0;
                while x < 32 {
                    // TODO: use dma to swap out the icons
                    let (r, g, b) = split_to_rgb565(DEFAULT_ICONS[i][32 * y + x]);
                    icons[i].1[32 * y + x] = Bgr565::new(r, g, b);
                    x += 1;
                }
                y += 1;
            }
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
    PROFILE_NAME.with_mut_lock(|p| *p = (true, DEFAULT_PROFILE_NAME));
    RGB_CONTROLS.with_mut_lock(|c| *c = (true, DEFAULT_RGB_PROFILE.with_lock(|p| p.1.clone())));
    SCREEN_CONTROLS
        .with_mut_lock(|s| *s = (true, DEFAULT_SCREEN_PROFILE.with_lock(|p| p.1.clone())));
    SCREEN_SYSTEM_STATS.with_mut_lock(|s| *s = (true, DEFAULT_SYSTEM_STATS));
    INPUT_EVENTS.with_mut_lock(|e| *e = DEFAULT_INPUT_EVENTS.with_lock(|p| p.1.clone()));
    reset_icons();
}

pub fn get_keyboard_events() -> [Keyboard; 16 * 6] {
    let mut keys = [Keyboard::NoEventIndicated; 16 * 6];

    INPUT_EVENTS.with_lock(|e| {
        let mut f = |k: bool, o: usize| match &e[o] {
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

pub fn get_mouse_events() -> WheelMouseReport {
    let mut buttons = 0u8;
    let mut x = 0isize;
    let mut y = 0isize;
    let mut scroll_y = 0isize;
    let mut scroll_x = 0isize;

    INPUT_EVENTS.with_lock(|e| {
        let mut f = |k: bool, o: usize| match &e[o] {
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
