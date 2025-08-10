//! Utility functions

use embassy_rp::spinlock_mutex::SpinlockRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
use jukebox_util::rgb::RgbProfile;

pub fn bootsel() {
    // TODO: make peripherals go dark before rebooting.
    embassy_rp::rom_data::reboot(0x0002, 0, 0x01, 0);
}

// Spinlock Mutexes
// We keep them here so we don't accidentally overlap spinlocks.
pub type IdentifyMutex = Mutex<SpinlockRawMutex<1>, Instant>;
pub type RgbProfileMutex = Mutex<SpinlockRawMutex<2>, RgbProfile>;
