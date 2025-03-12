// A desktop application for interfacing with a JukeBox over serial.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // disables console spawning for release build

#[macro_use]
extern crate rust_i18n;
i18n!("locales", fallback = "en", minify_key = true);

mod action;
mod config;
mod gui;
mod input;
mod serial;
mod splash;
mod update;
mod actions {
    #[cfg(feature = "discord")]
    pub mod discord;
    pub mod input;
    pub mod meta;
    pub mod obs;
    pub mod soundboard;
    pub mod system;
    pub mod types;
}

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();

    gui::basic_gui();

    Ok(())
}
