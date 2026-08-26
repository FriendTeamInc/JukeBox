// A desktop application for interfacing with a JukeBox over serial.

// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // disables console spawning for release build

use anyhow::bail;
use fd_lock::RwLock;
use reqwest::Client;
use std::fs::OpenOptions;

use crate::config::JukeBoxConfig;

#[macro_use]
extern crate rust_i18n;
i18n!("locales", fallback = "en");

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
    // setup config folder
    JukeBoxConfig::get_dir();

    // we only allow one instance of jukebox to run so lock the app file
    let mut f = RwLock::new(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(JukeBoxConfig::get_app_lock())
            .unwrap(),
    );
    let fl = f.try_write();
    if let Err(_) = fl {
        // TODO: send signal to other app to reopen window
        bail!("failed to acquire exclusive lock for application. aborting.");
    };

    // set up logging folder
    {
        use flexi_logger::{Duplicate, FileSpec, LogSpecification, Logger};

        Logger::try_with_env_or_str("info")
            .unwrap_or_else(|_| Logger::with(LogSpecification::info()))
            .log_to_file(
                FileSpec::default()
                    .directory(JukeBoxConfig::get_logs_dir())
                    .basename("jukebox_desktop"),
            )
            .duplicate_to_stderr(Duplicate::All)
            .start()
            .ok();
    }

    // For OBS websocket TLS support, currently unused.
    // rustls::crypto::aws_lc_rs::default_provider()
    //     .install_default()
    //     .expect("failed to install rustls crypto provider");

    // GUI launches all the necessary threads when started
    gui::gui::basic_gui();

    drop(fl);

    Ok(())
}
