// A desktop application for interfacing with a JukeBox over serial.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // disables console spawning for release build

use reqwest::Client;

#[macro_use]
extern crate rust_i18n;
i18n!("locales", fallback = "en", minify_key = true);

mod actions;
mod config;
mod firmware_update;
mod gui;
mod input;
mod serial;
mod software_update;
mod splash;
mod system;

// static http client for various api calls
// we also set the user agent to something specific to this software
// (some api's reject requests with no user agent)
static REQWEST_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
pub fn get_reqwest_client() -> &'static reqwest::Client {
    REQWEST_CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent(format!("JukeBoxDesktop/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap()
    })
}

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
