pub const KEYBOARD_SCAN_CODES: [(&str, u8); 169] = [
    ("A", 0x04),
    ("B", 0x05),
    ("C", 0x06),
    ("D", 0x07),
    ("E", 0x08),
    ("F", 0x09),
    ("G", 0x0A),
    ("H", 0x0B),
    ("I", 0x0C),
    ("J", 0x0D),
    ("K", 0x0E),
    ("L", 0x0F),
    ("M", 0x10),
    ("N", 0x11),
    ("O", 0x12),
    ("P", 0x13),
    ("Q", 0x14),
    ("R", 0x15),
    ("S", 0x16),
    ("T", 0x17),
    ("U", 0x18),
    ("V", 0x19),
    ("W", 0x1A),
    ("X", 0x1B),
    ("Y", 0x1C),
    ("Z", 0x1D),
    ("1", 0x1E),
    ("2", 0x1F),
    ("3", 0x20),
    ("4", 0x21),
    ("5", 0x22),
    ("6", 0x23),
    ("7", 0x24),
    ("8", 0x25),
    ("9", 0x26),
    ("0", 0x27),
    ("Enter", 0x28),
    ("Escape", 0x29),
    ("Backspace", 0x2A),
    ("Tab", 0x2B),
    ("Space", 0x2C),
    ("-", 0x2D),
    ("=", 0x2E),
    ("[", 0x2F),
    ("]", 0x30),
    ("\\", 0x31),
    ("Non-US #", 0x32),
    (";", 0x33),
    ("'", 0x34),
    ("`", 0x35),
    (",", 0x36),
    (".", 0x37),
    ("/", 0x38),
    ("Caps Lock", 0x39),
    ("F1", 0x3A),
    ("F2", 0x3B),
    ("F3", 0x3C),
    ("F4", 0x3D),
    ("F5", 0x3E),
    ("F6", 0x3F),
    ("F7", 0x40),
    ("F8", 0x41),
    ("F9", 0x42),
    ("F10", 0x43),
    ("F11", 0x44),
    ("F12", 0x45),
    ("Print Screen", 0x46),
    ("Scroll Lock", 0x47),
    ("Pause", 0x48),
    ("Insert", 0x49),
    ("Home", 0x4A),
    ("Page Up", 0x4B),
    ("Delete", 0x4C),
    ("End", 0x4D),
    ("Page Down", 0x4E),
    ("Right Arrow", 0x4F),
    ("Left Arrow", 0x50),
    ("Down Arrow", 0x51),
    ("Up Arrow", 0x52),
    ("Keypad Num Lock / Clear", 0x53),
    ("Keypad /", 0x54),
    ("Keypad *", 0x55),
    ("Keypad -", 0x56),
    ("Keypad +", 0x57),
    ("Keypad Enter", 0x58),
    ("Keypad 1", 0x59),
    ("Keypad 2", 0x5A),
    ("Keypad 3", 0x5B),
    ("Keypad 4", 0x5C),
    ("Keypad 5", 0x5D),
    ("Keypad 6", 0x5E),
    ("Keypad 7", 0x5F),
    ("Keypad 8", 0x60),
    ("Keypad 9", 0x61),
    ("Keypad 0", 0x62),
    ("Keypad .", 0x63),
    ("Non-US \\", 0x64),
    ("Application", 0x65),
    ("Power", 0x66),
    ("Keypad =", 0x67),
    ("F13", 0x68),
    ("F14", 0x69),
    ("F15", 0x6A),
    ("F16", 0x6B),
    ("F17", 0x6C),
    ("F18", 0x6D),
    ("F19", 0x6E),
    ("F20", 0x6F),
    ("F21", 0x70),
    ("F22", 0x71),
    ("F23", 0x72),
    ("F24", 0x73),
    ("Execute", 0x74),
    ("Help", 0x75),
    ("Menu", 0x76),
    ("Select", 0x77),
    ("Stop", 0x78),
    ("Again", 0x79),
    ("Undo", 0x7A),
    ("Cut", 0x7B),
    ("Copy", 0x7C),
    ("Paste", 0x7D),
    ("Find", 0x7E),
    ("Mute", 0x7F),
    ("Volume Up", 0x80),
    ("Volume Down", 0x81),
    ("Locking Caps Lock", 0x82),
    ("Locking Num Lock", 0x83),
    ("Locking Scroll Lock", 0x84),
    ("Keypad ,", 0x85),
    ("Keypad = Sign", 0x86),
    ("Kanji 1", 0x87),
    ("Kanji 2", 0x88),
    ("Kanji 3", 0x89),
    ("Kanji 4", 0x8A),
    ("Kanji 5", 0x8B),
    ("Kanji 6", 0x8C),
    ("Kanji 7", 0x8D),
    ("Kanji 8", 0x8E),
    ("Kanji 9", 0x8F),
    ("LANG 1", 0x90),
    ("LANG 2", 0x91),
    ("LANG 3", 0x92),
    ("LANG 4", 0x93),
    ("LANG 5", 0x94),
    ("LANG 6", 0x95),
    ("LANG 7", 0x96),
    ("LANG 8", 0x97),
    ("LANG 9", 0x98),
    ("Alternate Erase", 0x99),
    ("SysReq / Attention", 0x9A),
    ("Cancel", 0x9B),
    ("Clear", 0x9C),
    ("Prior", 0x9D),
    ("Return", 0x9E),
    ("Separator", 0x9F),
    ("Out", 0xA0),
    ("Oper", 0xA1),
    ("Clear Again", 0xA2),
    ("CrSel / Props", 0xA3),
    ("ExSel", 0xA4),
    //0xA5-0xAF Reserved
    //0xB0-0xDF May Not Work
    ("Left Control", 0xE0),
    ("Left Shift", 0xE1),
    ("Left Alt", 0xE2),
    ("Left Super", 0xE3), // aka GUI
    ("Right Control", 0xE4),
    ("Right Shift", 0xE5),
    ("Right Alt", 0xE6),
    ("Right Super", 0xE7), // aka GUI
]; //0xE8-0xFFFF Reserved

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
}

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
