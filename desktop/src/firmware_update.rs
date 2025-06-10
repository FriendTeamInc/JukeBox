use std::{fmt, thread::sleep, time::Duration};

use picoboot_rs::{
    PicobootConnection, FLASH_START, PAGE_SIZE, PICOBOOT_PID_RP2040, PICOBOOT_VID, SECTOR_SIZE,
    STACK_POINTER_RP2040,
};
use rusb::Context;
use tokio::sync::mpsc::UnboundedSender;
use uf2_decode::convert_from_uf2;

#[derive(Clone, PartialEq)]
pub struct UpdateError {
    pub msg: String,
}
impl UpdateError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }
}
impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

#[derive(Clone, PartialEq)]
pub enum FirmwareUpdateStatus {
    Start,
    Connecting,
    PreparingFirmware,
    ErasingOldFirmware(f32),
    WritingNewFirmware(f32),
    End,
    Error(UpdateError),
}

// creates a vector of vectors of u8's that map to flash pages sequentially
fn uf2_pages(bytes: Vec<u8>) -> Result<Vec<Vec<u8>>, UpdateError> {
    // loads the uf2 file into a binary
    let fw = convert_from_uf2(&bytes)
        .map_err(|e| UpdateError::new(t!("update.error.firmware_parse", e = format!("{:?}", e))))?
        .0;

    let mut fw_pages: Vec<Vec<u8>> = vec![];
    let len = fw.len();

    // splits the binary into sequential pages
    for i in (0..len).step_by(PAGE_SIZE as usize) {
        let size = std::cmp::min(len - i, PAGE_SIZE as usize);
        let mut page = fw[i..i + size].to_vec();
        page.resize(PAGE_SIZE as usize, 0);
        fw_pages.push(page);
    }

    Ok(fw_pages)
}

pub async fn firmware_update_task(
    fw: Vec<u8>,
    status: UnboundedSender<FirmwareUpdateStatus>,
) -> Result<(), UpdateError> {
    let ctx =
        Context::new().map_err(|e| UpdateError::new(t!("update.error.usb_context_fail", e = e)))?;

    status.send(FirmwareUpdateStatus::Connecting).unwrap();

    let mut conn = None;

    log::debug!("picoboot update: looking for device to update");
    for i in 0..3 {
        match PicobootConnection::new(ctx.clone(), Some((PICOBOOT_VID, PICOBOOT_PID_RP2040))) {
            Ok(c) => {
                conn = Some(c);
                log::debug!("picoboot update: found device to update");
                break;
            }
            Err(e) => {
                log::warn!("picoboot update: {}", e)
            }
        }
        sleep(Duration::from_secs(i));
    }

    if let None = conn {
        log::error!("picoboot update: could not establish picoboot connection");
        return Err(UpdateError::new(t!("update.error.picoboot_connect_fail")));
    }

    let mut conn = conn.unwrap();
    log::debug!("picoboot update: beginning update process");

    conn.reset_interface()
        .map_err(|e| UpdateError::new(t!("update.error.picoboot_reset_interface_fail", e = e)))?;
    log::debug!("picoboot update: resetting interface");
    conn.access_exclusive_eject()
        .map_err(|e| UpdateError::new(t!("update.error.picoboot_reset_exclusive_fail", e = e)))?;
    log::debug!("picoboot update: access exclusive eject");
    conn.exit_xip()
        .map_err(|e| UpdateError::new(t!("update.error.picoboot_exit_xip_fail", e = e)))?;
    log::debug!("picoboot update: exitting xip");

    log::debug!("picoboot update: loading uf2");
    status
        .send(FirmwareUpdateStatus::PreparingFirmware)
        .unwrap();
    let fw_pages = uf2_pages(fw)?;

    status
        .send(FirmwareUpdateStatus::ErasingOldFirmware(0.0))
        .unwrap();

    // erase space on flash
    log::debug!("picoboot update: beginning erase process");
    let page_count = fw_pages.len();
    for (i, _) in fw_pages.iter().enumerate() {
        let addr = (i as u32) * PAGE_SIZE + FLASH_START;
        if (addr % SECTOR_SIZE) == 0 {
            conn.flash_erase(addr, SECTOR_SIZE).map_err(|e| {
                UpdateError::new(t!("update.error.flash_erase_fail", addr = addr, e = e))
            })?;

            status
                .send(FirmwareUpdateStatus::ErasingOldFirmware(
                    (i as f32) / (page_count as f32),
                ))
                .unwrap();

            log::debug!("picoboot update: erased {} / {}", i, page_count);
        }
    }

    log::debug!("picoboot update: erase complete");
    status
        .send(FirmwareUpdateStatus::WritingNewFirmware(0.0))
        .unwrap();

    log::debug!("picoboot update: beginning write process");
    for (i, page) in fw_pages.iter().enumerate() {
        let addr = (i as u32) * PAGE_SIZE + FLASH_START;
        let size = PAGE_SIZE as u32;

        // write page to flash
        conn.flash_write(addr, page).map_err(|e| {
            UpdateError::new(t!("update.error.flash_write_fail", addr = addr, e = e))
        })?;
        log::debug!("picoboot update: writed {} / {}", i, page_count);

        // confirm flash write was successful
        let read = conn.flash_read(addr, size).map_err(|e| {
            UpdateError::new(t!("update.error.flash_read_fail", addr = addr, e = e))
        })?;
        let matching = page.iter().zip(&read).all(|(&a, &b)| a == b);
        log::debug!("picoboot update: write check: {}", matching);
        if !matching {
            return Err(UpdateError::new(t!(
                "update.error.flash_check_fail",
                addr = addr
            )));
        }

        status
            .send(FirmwareUpdateStatus::WritingNewFirmware(
                (i as f32) / (page_count as f32),
            ))
            .unwrap();
    }

    status.send(FirmwareUpdateStatus::End).unwrap();

    conn.reboot(0x0, STACK_POINTER_RP2040, 500)
        .map_err(|e| UpdateError::new(t!("update.error.device_reboot_fail", e = e)))?;

    Ok(())
}
