// A desktop application for interfacing with a JukeBox over serial.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // disables console spawning for release build

#[macro_use]
extern crate rust_i18n;
i18n!("locales", fallback = "en", minify_key = true);

mod actions;
mod config;
mod gui;
mod input;
mod serial;
mod splash;
mod update;

use anyhow::Result;

fn main() -> Result<()> {
    // TODO: add CPU and Memory monitoring support through sysinfo crate
    // TODO: add GPU monitoring support to Rust version through:
    // - nvml-wrapper crate (NVIDIA)
    // - rocm_smi_lib crate (AMD)
    // - Intel Graphics Control Library through Rust wrappers

    env_logger::init();

    // For OBS websocket TLS support, currently unused.
    // rustls::crypto::aws_lc_rs::default_provider()
    //     .install_default()
    //     .expect("failed to install rustls crypto provider");

    gui::gui::basic_gui();

    Ok(())
}
