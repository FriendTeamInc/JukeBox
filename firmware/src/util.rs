//! Utility functions

pub fn bootsel() {
    // TODO: make peripherals go dark before rebooting.
    embassy_rp::rom_data::reboot(0x0002, 0, 0x01, 0);
}
