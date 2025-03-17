// Config

use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    path::PathBuf,
};

use jukebox_util::{color::RgbProfile, peripheral::DeviceType};
use serde::{Deserialize, Serialize};

use crate::{actions::types::Action, input::InputKey};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiscordOauthAccess {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JukeBoxConfig {
    // Profile Name
    pub current_profile: String,
    pub profiles:
        HashMap<String, HashMap<String, (HashMap<InputKey, Box<dyn Action>>, Option<RgbProfile>)>>,
    // Profile Name -> ( Device UID -> ( Input Key -> Action Config ) )

    // Device UID -> (Device Type, Device Nickname)
    pub devices: HashMap<String, (DeviceType, String)>,

    pub enable_splash: bool,

    pub discord_oauth_access: Option<DiscordOauthAccess>,
}
impl Default for JukeBoxConfig {
    fn default() -> Self {
        JukeBoxConfig {
            current_profile: "Default Profile".to_string(),
            profiles: HashMap::from([("Default Profile".to_string(), HashMap::new())]),
            devices: HashMap::new(),
            enable_splash: true,
            discord_oauth_access: None,
        }
    }
}
impl JukeBoxConfig {
    fn get_dir() -> PathBuf {
        let mut p = dirs::config_dir().expect("failed to find config directory");
        p.push("JukeBoxDesktop");
        create_dir_all(&p).expect("failed to create config directory");
        p
    }

    fn get_path() -> PathBuf {
        let mut p = Self::get_dir();
        p.push("config.json");
        p
    }

    pub fn load() -> Self {
        let path = Self::get_path();

        let file = match File::open(path) {
            Err(e) => {
                log::error!("failed to open config file: {}", e);
                return JukeBoxConfig::default();
            }
            Ok(f) => f,
        };

        let conf = match serde_json::from_reader(file) {
            Err(e) => {
                log::error!("failed to parse config file: {}", e);

                let paths: Vec<_> = std::fs::read_dir(Self::get_dir())
                    .unwrap()
                    .filter(|f| {
                        f.as_ref()
                            .and_then(|f| {
                                Ok(f.file_name().to_string_lossy().contains("config.json"))
                            })
                            .unwrap_or(false)
                    })
                    .collect();

                let mut p = Self::get_dir();
                p.push(format!("config.json.old.{}", paths.len()));

                log::error!("saving old config as {:?}...", p);

                std::fs::rename(Self::get_path(), p).expect("failed to save old config");

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
