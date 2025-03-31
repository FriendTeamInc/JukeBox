use std::{thread::sleep, time::Duration};

use anyhow::{bail, Result};
use picoboot_rs::{
    PicobootConnection, FLASH_START, PAGE_SIZE, PICOBOOT_PID_RP2040, PICOBOOT_VID, SECTOR_SIZE,
    STACK_POINTER_RP2040,
};
use rusb::Context;
use tokio::sync::mpsc::UnboundedSender;
use uf2_decode::convert_from_uf2;

#[derive(Clone, Copy, PartialEq)]
pub enum UpdateStatus {
    Start,
    Connecting,
    PreparingFirmware,
    ErasingOldFirmware(f32),
    WritingNewFirmware(f32),
    End,
}

// creates a vector of vectors of u8's that map to flash pages sequentially
fn uf2_pages(bytes: Vec<u8>) -> Vec<Vec<u8>> {
    // loads the uf2 file into a binary
    let fw = convert_from_uf2(&bytes).expect("failed to parse uf2").0;

    let mut fw_pages: Vec<Vec<u8>> = vec![];
    let len = fw.len();

    // splits the binary into sequential pages
    for i in (0..len).step_by(PAGE_SIZE as usize) {
        let size = std::cmp::min(len - i, PAGE_SIZE as usize);
        let mut page = fw[i..i + size].to_vec();
        page.resize(PAGE_SIZE as usize, 0);
        fw_pages.push(page);
    }

    fw_pages
}

// TODO: change to async
fn update_device(
    ctx: Context,
    fw_path: String,
    status: UnboundedSender<UpdateStatus>,
) -> Result<()> {
    // TODO: add context()'s

    status.send(UpdateStatus::Connecting)?;

    let mut conn = None;

    for i in 0..3 {
        match PicobootConnection::new(ctx.clone(), Some((PICOBOOT_VID, PICOBOOT_PID_RP2040))) {
            Ok(c) => {
                conn = Some(c);
                break;
            }
            Err(e) => {
                log::warn!("{}", e)
            }
        }
        sleep(Duration::from_secs(i));
    }

    if let None = conn {
        bail!("could not establish picoboot connection");
    }

    let mut conn = conn.unwrap();

    conn.reset_interface()?;
    conn.access_exclusive_eject()?;
    conn.exit_xip()?;

    status.send(UpdateStatus::PreparingFirmware)?;

    let fw = std::fs::read(fw_path)?;
    let fw_pages = uf2_pages(fw);

    status.send(UpdateStatus::ErasingOldFirmware(0.0))?;

    // erase space on flash
    let page_count = fw_pages.len();
    for (i, _) in fw_pages.iter().enumerate() {
        let addr = (i as u32) * PAGE_SIZE + FLASH_START;
        if (addr % SECTOR_SIZE) == 0 {
            conn.flash_erase(addr, SECTOR_SIZE)?;

            status.send(UpdateStatus::ErasingOldFirmware(
                (i as f32) / (page_count as f32),
            ))?;
        }
    }

    status.send(UpdateStatus::WritingNewFirmware(0.0))?;

    for (i, page) in fw_pages.iter().enumerate() {
        let addr = (i as u32) * PAGE_SIZE + FLASH_START;
        let size = PAGE_SIZE as u32;

        // write page to flash
        conn.flash_write(addr, page)?;

        // confirm flash write was successful
        let read = conn.flash_read(addr, size)?;
        let matching = page.iter().zip(&read).all(|(&a, &b)| a == b);
        assert!(matching, "page does not match flash"); // TODO: change to Err()

        status.send(UpdateStatus::WritingNewFirmware(
            (i as f32) / (page_count as f32),
        ))?;
    }

    status.send(UpdateStatus::End)?;

    conn.reboot(0x0, STACK_POINTER_RP2040, 500)?;

    Ok(())
}

pub async fn update_task(
    device_uid: String,
    fw_path: String,
    update_status: UnboundedSender<UpdateStatus>,
) {
    let ctx = Context::new().expect("failed to get USB context");

    match update_device(ctx, fw_path, update_status) {
        Ok(_) => {}
        Err(e) => {
            log::error!("Failed to update device \"{}\": {}", device_uid, e)
        }
    }
}
