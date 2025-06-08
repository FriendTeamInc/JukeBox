// Config

use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    path::PathBuf,
};

use jukebox_util::{peripheral::DeviceType, rgb::RgbProfile, screen::ScreenProfile};
use serde::{Deserialize, Serialize};

use crate::{
    actions::{meta::MetaNoAction, types::Action},
    input::InputKey,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DiscordOauthAccess {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ObsAccess {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum ActionIcon {
    ImageIcon(String),
    DefaultActionIcon,
}
impl Default for ActionIcon {
    fn default() -> Self {
        Self::DefaultActionIcon
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ActionConfig {
    pub action: Action,
    pub icon: ActionIcon,
}
impl Default for ActionConfig {
    fn default() -> Self {
        Self {
            action: Action::MetaNoAction(MetaNoAction::default()),
            icon: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceConfig {
    pub key_map: HashMap<InputKey, ActionConfig>,
    pub rgb_profile: Option<RgbProfile>,
    pub screen_profile: Option<ScreenProfile>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceInfo {
    pub device_type: DeviceType,
    pub nickname: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JukeBoxConfig {
    // Profile Name
    pub current_profile: String,
    pub profiles: HashMap<String, HashMap<String, DeviceConfig>>,
    // Profile Name -> Device UID -> Device Config

    // Device UID -> (Device Type, Device Nickname)
    pub devices: HashMap<String, DeviceInfo>,

    pub discord_oauth_access: Option<DiscordOauthAccess>,
    pub obs_access: Option<ObsAccess>,

    pub enable_splash: bool,
    pub always_save_on_exit: bool,
    pub ignore_update_notifications: bool,
}
impl Default for JukeBoxConfig {
    fn default() -> Self {
        JukeBoxConfig {
            current_profile: "Default Profile".into(),
            profiles: HashMap::from([("Default Profile".into(), HashMap::new())]),
            devices: HashMap::new(),

            discord_oauth_access: None,
            obs_access: None,

            enable_splash: true,
            always_save_on_exit: false,
            ignore_update_notifications: false,
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

    pub fn get_icon_dir() -> PathBuf {
        let mut p = Self::get_dir();
        p.push("icons");
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
                            .map(|f| f.file_name().to_string_lossy().contains("config.json"))
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
