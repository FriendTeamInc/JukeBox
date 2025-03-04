// A desktop application for interfacing with a JukeBox over serial.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // disables console spawning for release build

mod config;
mod gui;
mod input;
mod reaction;
mod serial;
mod splash;
mod reactions {
    pub mod meta;
    pub mod system;
    pub mod types;
}

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();

    gui::basic_gui();

    Ok(())
}
