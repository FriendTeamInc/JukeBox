use jukebox_util::{rgb::RgbProfile, screen::ScreenProfile};
use rp2040_hal::{
    gpio::{
        bank0::{Gpio4, Gpio5},
        FunctionI2c, Pin, PullUp,
    },
    pac::I2C0,
    I2C,
};

use crate::util::{DEFAULT_RGB_PROFILE, DEFAULT_SCREEN_PROFILE};

const DEFAULTS_PAGE_SIZE: usize = 512;
const FULL_PAGE_SIZE: usize = DEFAULTS_PAGE_SIZE + 2; // marker + crc + page
const EEPROM_SIZE: u32 = 65536;

// Page Layout
// 0x000-0x010 - Reserved
// 0x010-0x090 - Default USB HID Events (7*16=112)
// 0x090-0x0D0 - Default RGB Profile (64)
// 0x0D0-0x1D0 - Default Screen Profile (256)
// 0x1D0-0x200 - Reserved
const USB_HID_EVENTS_RANGE_START: usize = 0x010;
const USB_HID_EVENTS_RANGE_END: usize = 0x090;
const RGB_PROFILE_RANGE_START: usize = 0x090;
const RGB_PROFILE_RANGE_END: usize = 0x0D0;
const SCREEN_PROFILE_RANGE_START: usize = 0x0D0;
const SCREEN_PROFILE_RANGE_END: usize = 0x1D0;

type EepromI2c = I2C<
    I2C0,
    (
        Pin<Gpio4, FunctionI2c, PullUp>,
        Pin<Gpio5, FunctionI2c, PullUp>,
    ),
>;

type EepromDev = eeprom24x::Eeprom24x<
    EepromI2c,
    eeprom24x::page_size::B128,
    eeprom24x::addr_size::TwoBytes,
    eeprom24x::unique_serial::No,
>;

pub struct EepromMod {
    eeprom_dev: EepromDev,
    defaults_address: u32,
}

impl EepromMod {
    pub fn new(eeprom_i2c: EepromI2c) -> Self {
        let eeprom_dev =
            eeprom24x::Eeprom24x::new_24x512(eeprom_i2c, eeprom24x::SlaveAddr::Default);

        let mut eeprom_mod = Self {
            eeprom_dev,
            defaults_address: 0,
        };

        eeprom_mod.initialize();

        eeprom_mod
    }

    fn initialize(&mut self) {
        // Check that the first page is 0xFB, if not then we need to initialize the device
        // TODO: check against a version?
        let mut read = [0x0u8; 1];
        let _ = self.eeprom_dev.read_data(0x0, &mut read);
        if read != [0xFBu8; 1] {
            self.wipe_eeprom();
            return;
        }

        // TODO: read eeprom values into defaults
        self.load_defaults();
    }

    fn wipe_eeprom(&mut self) {
        for i in 0..512 {
            let _ = self.eeprom_dev.write_page(i * 128, &[0xFFu8; 128]);
        }
        let _ = self.eeprom_dev.write_page(0x0, &[0xFBu8; 1]);

        self.defaults_address = 0;

        self.save_eeprom();
    }

    fn save_eeprom(&mut self) {
        // wipes the original page (if it exists) and writes a new page
        let page = Self::encode_defaults_page();
        let crc = Self::checksum(&page);

        if self.defaults_address == 0 {
            // save new page
            self.defaults_address = 1;
            let _ = self.eeprom_dev.write_page(0x1, &[0x0u8; 1]);
            let _ = self.eeprom_dev.write_page(0x2, &[crc; 1]);
            let _ = self.eeprom_dev.write_page(0x3, &page);
        } else {
            // mark old page as old
            let _ = self.eeprom_dev.write_byte(self.defaults_address, 0xFF);

            // new page start address, wrap around if page would overrun
            self.defaults_address += FULL_PAGE_SIZE as u32;
            if self.defaults_address + (FULL_PAGE_SIZE as u32) - 1 > EEPROM_SIZE {
                self.defaults_address = 1;
            }

            // save new page
            let _ = self
                .eeprom_dev
                .write_page(self.defaults_address + 0x0, &[0x0u8; 1]);
            let _ = self
                .eeprom_dev
                .write_page(self.defaults_address + 0x1, &[crc; 1]);
            let _ = self
                .eeprom_dev
                .write_page(self.defaults_address + 0x2, &page);
        }
    }

    fn load_defaults(&mut self) {
        // search for start of page, and read it in
        // since pages are fixed size we can search specific spots for where the current page is
        for i in 0..((EEPROM_SIZE / (FULL_PAGE_SIZE as u32)) - 1) {
            let addr = i * (FULL_PAGE_SIZE as u32) + 0x1;
            let marker = self.eeprom_dev.read_byte(addr).unwrap();

            if marker != 0 {
                continue;
            }

            let eeprom_crc = self.eeprom_dev.read_byte(addr + 0x1).unwrap();

            let mut data = [0u8; DEFAULTS_PAGE_SIZE];
            let _ = self.eeprom_dev.read_data(addr + 0x2, &mut data).unwrap();
            let data_crc = Self::checksum(&data);

            if eeprom_crc != data_crc {
                continue;
            }

            self.defaults_address = addr;
            // TODO: add usb hid events
            let new_default_rgb_profile =
                RgbProfile::decode(&data[RGB_PROFILE_RANGE_START..RGB_PROFILE_RANGE_END]);
            let new_default_screen_profile =
                ScreenProfile::decode(&data[SCREEN_PROFILE_RANGE_START..SCREEN_PROFILE_RANGE_END]);

            DEFAULT_RGB_PROFILE.with_mut_lock(|p| *p = (true, new_default_rgb_profile));
            DEFAULT_SCREEN_PROFILE.with_mut_lock(|p| *p = (true, new_default_screen_profile));
        }
    }

    fn encode_defaults_page() -> [u8; DEFAULTS_PAGE_SIZE] {
        let mut data = [0u8; DEFAULTS_PAGE_SIZE];

        // TODO: add usb hid events
        DEFAULT_RGB_PROFILE.with_lock(|p| {
            let d = p.1.clone().encode();
            data[RGB_PROFILE_RANGE_START..RGB_PROFILE_RANGE_END].copy_from_slice(&d);
        });
        DEFAULT_SCREEN_PROFILE.with_lock(|p| {
            let d = p.1.clone().encode();
            data[SCREEN_PROFILE_RANGE_START..SCREEN_PROFILE_RANGE_END].copy_from_slice(&d);
        });

        data
    }

    fn checksum(bytes: &[u8]) -> u8 {
        let mut crc = 0u8;
        for b in bytes {
            crc ^= b;
        }
        crc
    }
}
