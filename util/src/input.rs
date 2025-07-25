use bincode::{decode_from_slice, encode_into_slice, Decode, Encode};
use serde::{Deserialize, Serialize};

pub const KEYBOARD_SCAN_CODES: [(u8, &str); 169] = [
    (0x04, "A"),
    (0x05, "B"),
    (0x06, "C"),
    (0x07, "D"),
    (0x08, "E"),
    (0x09, "F"),
    (0x0A, "G"),
    (0x0B, "H"),
    (0x0C, "I"),
    (0x0D, "J"),
    (0x0E, "K"),
    (0x0F, "L"),
    (0x10, "M"),
    (0x11, "N"),
    (0x12, "O"),
    (0x13, "P"),
    (0x14, "Q"),
    (0x15, "R"),
    (0x16, "S"),
    (0x17, "T"),
    (0x18, "U"),
    (0x19, "V"),
    (0x1A, "W"),
    (0x1B, "X"),
    (0x1C, "Y"),
    (0x1D, "Z"),
    (0x1E, "1"),
    (0x1F, "2"),
    (0x20, "3"),
    (0x21, "4"),
    (0x22, "5"),
    (0x23, "6"),
    (0x24, "7"),
    (0x25, "8"),
    (0x26, "9"),
    (0x27, "0"),
    (0x28, "Enter"),
    (0x29, "Escape"),
    (0x2A, "Backspace"),
    (0x2B, "Tab"),
    (0x2C, "Space"),
    (0x2D, "-"),
    (0x2E, "="),
    (0x2F, "["),
    (0x30, "]"),
    (0x31, "\\"),
    (0x32, "Non-US #"),
    (0x33, ";"),
    (0x34, "'"),
    (0x35, "`"),
    (0x36, ","),
    (0x37, "."),
    (0x38, "/"),
    (0x39, "Caps Lock"),
    (0x3A, "F1"),
    (0x3B, "F2"),
    (0x3C, "F3"),
    (0x3D, "F4"),
    (0x3E, "F5"),
    (0x3F, "F6"),
    (0x40, "F7"),
    (0x41, "F8"),
    (0x42, "F9"),
    (0x43, "F10"),
    (0x44, "F11"),
    (0x45, "F12"),
    (0x46, "Print Screen"),
    (0x47, "Scroll Lock"),
    (0x48, "Pause"),
    (0x49, "Insert"),
    (0x4A, "Home"),
    (0x4B, "Page Up"),
    (0x4C, "Delete"),
    (0x4D, "End"),
    (0x4E, "Page Down"),
    (0x4F, "Right Arrow"),
    (0x50, "Left Arrow"),
    (0x51, "Down Arrow"),
    (0x52, "Up Arrow"),
    (0x53, "Keypad Num Lock / Clear"),
    (0x54, "Keypad /"),
    (0x55, "Keypad *"),
    (0x56, "Keypad -"),
    (0x57, "Keypad +"),
    (0x58, "Keypad Enter"),
    (0x59, "Keypad 1"),
    (0x5A, "Keypad 2"),
    (0x5B, "Keypad 3"),
    (0x5C, "Keypad 4"),
    (0x5D, "Keypad 5"),
    (0x5E, "Keypad 6"),
    (0x5F, "Keypad 7"),
    (0x60, "Keypad 8"),
    (0x61, "Keypad 9"),
    (0x62, "Keypad 0"),
    (0x63, "Keypad ."),
    (0x64, "Non-US \\"),
    (0x65, "Application"),
    (0x66, "Power"),
    (0x67, "Keypad ="),
    (0x68, "F13"),
    (0x69, "F14"),
    (0x6A, "F15"),
    (0x6B, "F16"),
    (0x6C, "F17"),
    (0x6D, "F18"),
    (0x6E, "F19"),
    (0x6F, "F20"),
    (0x70, "F21"),
    (0x71, "F22"),
    (0x72, "F23"),
    (0x73, "F24"),
    (0x74, "Execute"),
    (0x75, "Help"),
    (0x76, "Menu"),
    (0x77, "Select"),
    (0x78, "Stop"),
    (0x79, "Again"),
    (0x7A, "Undo"),
    (0x7B, "Cut"),
    (0x7C, "Copy"),
    (0x7D, "Paste"),
    (0x7E, "Find"),
    (0x7F, "Mute"),
    (0x80, "Volume Up"),
    (0x81, "Volume Down"),
    (0x82, "Locking Caps Lock"),
    (0x83, "Locking Num Lock"),
    (0x84, "Locking Scroll Lock"),
    (0x85, "Keypad ,"),
    (0x86, "Keypad = Sign"),
    (0x87, "Kanji 1"),
    (0x88, "Kanji 2"),
    (0x89, "Kanji 3"),
    (0x8A, "Kanji 4"),
    (0x8B, "Kanji 5"),
    (0x8C, "Kanji 6"),
    (0x8D, "Kanji 7"),
    (0x8E, "Kanji 8"),
    (0x8F, "Kanji 9"),
    (0x90, "LANG 1"),
    (0x91, "LANG 2"),
    (0x92, "LANG 3"),
    (0x93, "LANG 4"),
    (0x94, "LANG 5"),
    (0x95, "LANG 6"),
    (0x96, "LANG 7"),
    (0x97, "LANG 8"),
    (0x98, "LANG 9"),
    (0x99, "Alternate Erase"),
    (0x9A, "SysReq / Attention"),
    (0x9B, "Cancel"),
    (0x9C, "Clear"),
    (0x9D, "Prior"),
    (0x9E, "Return"),
    (0x9F, "Separator"),
    (0xA0, "Out"),
    (0xA1, "Oper"),
    (0xA2, "Clear Again"),
    (0xA3, "CrSel / Props"),
    (0xA4, "ExSel"),
    //0xA5-0xAF Reserved
    //0xB0-0xDF May Not Work
    (0xE0, "Left Control"),
    (0xE1, "Left Shift"),
    (0xE2, "Left Alt"),
    (0xE3, "Left Super"), // aka GUI
    (0xE4, "Right Control"),
    (0xE5, "Right Shift"),
    (0xE6, "Right Alt"),
    (0xE7, "Right Super"), // aka GUI
]; //0xE8-0xFFFF Reserved

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum InputEvent {
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
}
impl InputEvent {
    #[rustfmt::skip]
    pub const fn default_events() -> [Self; 16] {
        [
            Self::Keyboard(KeyboardEvent { keys: [0x68, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x69, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x6A, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x6B, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x6C, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x6D, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x6E, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x6F, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x70, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x71, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x72, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0x73, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0xE4, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0xE5, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0xE6, 0, 0, 0, 0, 0], }),
            Self::Keyboard(KeyboardEvent { keys: [0xE7, 0, 0, 0, 0, 0], }),
        ]
    }

    pub fn encode(events: [Self; 16]) -> [u8; 112] {
        let mut data = [0u8; 112];
        let _ = encode_into_slice(events, &mut data, bincode::config::standard()).unwrap();
        data
    }

    pub fn decode(events: &[u8]) -> [Self; 16] {
        decode_from_slice(events, bincode::config::standard())
            .unwrap()
            .0
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct KeyboardEvent {
    pub keys: [u8; 6],
}
impl KeyboardEvent {
    pub fn encode(self) -> [u8; 6] {
        self.keys
    }

    pub fn decode(w: &[u8]) -> Self {
        let mut keys = [0u8; 6];
        keys.copy_from_slice(w);

        Self { keys }
    }

    pub const fn default_events() -> [Self; 12] {
        [
            Self {
                keys: [0x68, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x69, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x6A, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x6B, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x6C, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x6D, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x6E, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x6F, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x70, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x71, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x72, 0, 0, 0, 0, 0],
            },
            Self {
                keys: [0x73, 0, 0, 0, 0, 0],
            },
        ]
    }

    pub const fn empty_event() -> Self {
        Self { keys: [0; 6] }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MouseEvent {
    pub buttons: u8,
    pub x: i8,
    pub y: i8,
    pub scroll_y: i8,
    pub scroll_x: i8,
}
impl MouseEvent {
    pub fn encode(self) -> [u8; 5] {
        [
            self.buttons,
            self.x as u8,
            self.y as u8,
            self.scroll_y as u8,
            self.scroll_x as u8,
        ]
    }

    pub fn decode(w: &[u8]) -> Self {
        Self {
            buttons: w[0],
            x: w[1] as i8,
            y: w[2] as i8,
            scroll_y: w[3] as i8,
            scroll_x: w[4] as i8,
        }
    }

    pub const fn default() -> Self {
        Self {
            buttons: 0,
            x: 0,
            y: 0,
            scroll_y: 0,
            scroll_x: 0,
        }
    }

    pub const fn default_events() -> [Self; 12] {
        [
            Self::default(),
            Self::default(),
            Self::default(),
            Self::default(),
            Self::default(),
            Self::default(),
            Self::default(),
            Self::default(),
            Self::default(),
            Self::default(),
            Self::default(),
            Self::default(),
        ]
    }
}
