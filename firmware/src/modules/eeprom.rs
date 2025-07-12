use rp2040_hal::{
    gpio::{
        bank0::{Gpio4, Gpio5},
        FunctionI2c, Pin, PullUp,
    },
    pac::I2C0,
    I2C,
};

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
}

impl EepromMod {
    pub fn new(eeprom_i2c: EepromI2c) -> Self {
        let eeprom_dev =
            eeprom24x::Eeprom24x::new_24x512(eeprom_i2c, eeprom24x::SlaveAddr::Default);

        let mut eeprom_mod = Self { eeprom_dev };

        eeprom_mod.initialize();

        eeprom_mod
    }

    fn initialize(&mut self) {
        // Check that the first page is 0xFB, if not then we need to initialize the device
        let mut read = [0x0u8; 128];
        let _ = self.eeprom_dev.read_data(0x0, &mut read);
        if read != [0xFBu8; 128] {
            self.wipe_eeprom();
        }

        // TODO: read eeprom values into defaults
    }

    fn wipe_eeprom(&mut self) {
        let _ = self.eeprom_dev.write_page(0x0, &[0xFBu8; 128]);
        for i in 1..512 {
            let _ = self.eeprom_dev.write_page(i * 128, &[0x0u8; 128]);
        }

        // TODO: update relevant pages to defaults
    }
}
