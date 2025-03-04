// Config

use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::{gui::DeviceType, input::InputKey, reactions::types::Reaction};

#[derive(Serialize, Deserialize, Clone)]
pub struct JukeBoxConfig {
    // Profile Name
    pub current_profile: String,
    pub profiles: HashMap<String, HashMap<String, HashMap<InputKey, Box<dyn Reaction>>>>,
    // Profile Name -> ( Device UID -> ( Input Key -> Reaction Config ) )

    // Device UID -> (Device Type, Device Nickname)
    pub devices: HashMap<String, (DeviceType, String)>,

    pub enable_splash: bool,
}
impl Default for JukeBoxConfig {
    fn default() -> Self {
        JukeBoxConfig {
            current_profile: "Default Profile".to_string(),
            profiles: HashMap::from([("Default Profile".to_string(), HashMap::new())]),
            devices: HashMap::new(),
            enable_splash: true,
        }
    }
}
impl JukeBoxConfig {
    fn get_path() -> PathBuf {
        let mut p = dirs::config_dir().expect("failed to find config directory");
        p.push("JukeBoxDesktop");
        create_dir_all(&p).expect("failed to create config directory");
        p.push("config.json");
        p
    }

    pub fn load() -> Self {
        let path = Self::get_path();
        let file = match File::open(path) {
            Err(e) => {
                log::error!("failed to open config file: {}", e);
                // TODO: panic?
                return JukeBoxConfig::default();
            }
            Ok(f) => f,
        };

        let conf = match serde_json::from_reader(file) {
            Err(e) => {
                log::error!("failed to parse config file: {}", e);
                // TODO: rename old config so that it isnt overwritten by new default config
                return JukeBoxConfig::default();
            }
            Ok(c) => c,
        };

        // TODO: serde_validate the config?

        conf
    }

    pub fn save(&self) {
        let path = Self::get_path();
        let file = File::create(path).expect("failed to create config file");
        serde_json::to_writer(file, &self).expect("failed to write config file");
    }
}
