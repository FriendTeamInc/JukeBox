use core::cmp::min;

use defmt::error;
use embedded_hal::timer::CountDown as _;
use jukebox_util::{input::InputEvent, rgb::RgbProfile, screen::ScreenProfile};
use rp2040_hal::{
    fugit::ExtU32,
    gpio::{
        bank0::{Gpio4, Gpio5},
        FunctionI2c, Pin, PullUp,
    },
    pac::I2C0,
    timer::CountDown,
    I2C,
};

use crate::util::{DEFAULT_INPUT_EVENTS, DEFAULT_RGB_PROFILE, DEFAULT_SCREEN_PROFILE};

const UPDATE_RATE: u32 = 500;

const DEFAULTS_PAGE_SIZE: usize = 512;
const FULL_PAGE_SIZE: usize = DEFAULTS_PAGE_SIZE + 2; // marker + crc + page
const EEPROM_SIZE: u32 = 65536 / 4;
const EEPROM_PAGE_SIZE: u32 = 128;
const EEPROM_PAGE_COUNT: u32 = EEPROM_SIZE / EEPROM_PAGE_SIZE;
const EEPROM_MAGIC_BYTE: u8 = 0xFB;

// Page Layout
// 0x000-0x010 - Reserved
// 0x010-0x090 - Default USB HID Events (7*16=112, round up to 128)
// 0x090-0x0D0 - Default RGB Profile (64)
// 0x0D0-0x1D0 - Default Screen Profile (256)
// 0x1D0-0x200 - Reserved
const INPUT_EVENTS_RANGE_START: usize = 0x010;
const INPUT_EVENTS_RANGE_END: usize = 0x090 - 16;
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
    delay: CountDown,
    timer: CountDown,
}

impl EepromMod {
    pub fn new(eeprom_i2c: EepromI2c, delay: CountDown, mut timer: CountDown) -> Self {
        let eeprom_dev =
            eeprom24x::Eeprom24x::new_24x512(eeprom_i2c, eeprom24x::SlaveAddr::Default);

        timer.start(UPDATE_RATE.millis());

        let mut eeprom_mod = Self {
            eeprom_dev,
            defaults_address: 0,
            delay,
            timer,
        };

        eeprom_mod.initialize();

        eeprom_mod
    }

    fn initialize(&mut self) {
        // Check that the first page is 0xFB, if not then we need to initialize the device
        let read = self.eeprom_dev.read_byte(0x0).unwrap();
        if read != EEPROM_MAGIC_BYTE {
            self.wipe_eeprom();
            return;
        }

        self.load_defaults();
    }

    fn wipe_eeprom(&mut self) {
        for i in 0..EEPROM_PAGE_COUNT {
            self.write_page(i * 128, &[0xFFu8; 128]);
        }

        self.defaults_address = 0;
        self.save_eeprom();

        self.write_page(0x0, &[EEPROM_MAGIC_BYTE; 1]);
    }

    fn write_page(&mut self, addr: u32, data: &[u8]) {
        let mut idx = 0;
        let mut addr = addr;
        let addr_end = addr + data.len() as u32;
        while addr < addr_end {
            let page_len = min(
                (data.len() - idx) as u32,
                EEPROM_PAGE_SIZE - (addr % EEPROM_PAGE_SIZE),
            );

            let idx_start = idx;
            let idx_end = idx + page_len as usize;

            let page_addr_start = addr;
            let page_addr_end = addr + page_len;

            match self
                .eeprom_dev
                .write_page(page_addr_start, &data[idx_start..idx_end])
            {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        "eeprom error (addr, l1, l2): {} ({}, {}..{})",
                        e, addr, idx_start, idx_end
                    );
                    defmt::panic!();
                }
            };

            addr = page_addr_end;
            idx = idx_end;

            // let eeprom finish writing
            self.delay.start(5.millis());
            while self.delay.wait().is_err() {}
        }
    }

    fn save_eeprom(&mut self) {
        // wipes the original page (if it exists) and writes a new page
        let page = Self::encode_defaults_page();
        let crc = Self::checksum(&page);

        if self.defaults_address == 0 {
            // save new page
            self.defaults_address = 1;
            self.write_page(0x1, &[0x0u8; 1]);
            self.write_page(0x2, &[crc; 1]);
            self.write_page(0x3, &page);
        } else {
            // mark old page as old
            self.eeprom_dev
                .write_byte(self.defaults_address, 0xFF)
                .unwrap();

            // new page start address, wrap around if page would overrun
            self.defaults_address += FULL_PAGE_SIZE as u32;
            if self.defaults_address + (FULL_PAGE_SIZE as u32) - 1 > EEPROM_SIZE {
                self.defaults_address = 1;
            }

            // save new page
            self.write_page(self.defaults_address + 0x0, &[0x0u8; 1]);
            self.write_page(self.defaults_address + 0x1, &[crc; 1]);
            self.write_page(self.defaults_address + 0x2, &page);
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
            self.eeprom_dev.read_data(addr + 0x2, &mut data).unwrap();
            let data_crc = Self::checksum(&data);

            if eeprom_crc != data_crc {
                error!("crc did not match");
                continue;
            }

            self.defaults_address = addr;
            let new_usb_hid_events =
                InputEvent::decode_all(&data[INPUT_EVENTS_RANGE_START..INPUT_EVENTS_RANGE_END]);
            let new_default_rgb_profile =
                RgbProfile::decode(&data[RGB_PROFILE_RANGE_START..RGB_PROFILE_RANGE_END]);
            let new_default_screen_profile =
                ScreenProfile::decode(&data[SCREEN_PROFILE_RANGE_START..SCREEN_PROFILE_RANGE_END]);

            DEFAULT_INPUT_EVENTS.with_mut_lock(|p| *p = (false, new_usb_hid_events));
            DEFAULT_RGB_PROFILE.with_mut_lock(|p| *p = (false, new_default_rgb_profile));
            DEFAULT_SCREEN_PROFILE.with_mut_lock(|p| *p = (false, new_default_screen_profile));

            break;
        }
    }

    fn encode_defaults_page() -> [u8; DEFAULTS_PAGE_SIZE] {
        let mut data = [0u8; DEFAULTS_PAGE_SIZE];

        DEFAULT_INPUT_EVENTS.with_mut_lock(|p| {
            p.0 = false;
            let d = InputEvent::encode_all(p.1.clone());
            data[INPUT_EVENTS_RANGE_START..INPUT_EVENTS_RANGE_END].copy_from_slice(&d);
        });
        DEFAULT_RGB_PROFILE.with_mut_lock(|p| {
            p.0 = false;
            let d = p.1.clone().encode();
            data[RGB_PROFILE_RANGE_START..RGB_PROFILE_RANGE_END].copy_from_slice(&d);
        });
        DEFAULT_SCREEN_PROFILE.with_mut_lock(|p| {
            p.0 = false;
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

    pub fn update(&mut self) {
        if !self.timer.wait().is_ok() {
            return;
        }

        // if defaults have changed, update the eeprom
        let ie = DEFAULT_INPUT_EVENTS.with_lock(|i| i.0);
        let rp = DEFAULT_RGB_PROFILE.with_lock(|p| p.0);
        let sp = DEFAULT_SCREEN_PROFILE.with_lock(|p| p.0);
        if ie || rp || sp {
            self.save_eeprom();
        }
    }
}
