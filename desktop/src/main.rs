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
mod system;
mod update;

#[allow(dead_code)]
static REQWEST_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // For OBS websocket TLS support, currently unused.
    // rustls::crypto::aws_lc_rs::default_provider()
    //     .install_default()
    //     .expect("failed to install rustls crypto provider");

    // GUI launches all the necessary threads when started
    gui::gui::basic_gui();

    Ok(())
}
