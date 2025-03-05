use eframe::egui::{ComboBox, Ui};
use serde::{Deserialize, Serialize};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Reaction, ReactionType};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct MetaNoAction {}
#[typetag::serde]
impl Reaction for MetaNoAction {
    fn on_press(&self, device_uid: String, key: InputKey, _config: &mut JukeBoxConfig) -> () {
        log::info!("META NO ACTION: Device {} Pressed {:?} !", device_uid, key);
    }

    fn on_release(&self, device_uid: String, key: InputKey, _config: &mut JukeBoxConfig) -> () {
        log::info!("META NO ACTION: Device {} Released {:?} !", device_uid, key);
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::MetaNoAction
    }

    fn edit_ui(
        &mut self,
        _ui: &mut Ui,
        _device_uid: String,
        _key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
    }

    fn help(&self) -> String {
        "Does nothing!".to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct MetaSwitchProfile {
    profile: String,
}
#[typetag::serde]
impl Reaction for MetaSwitchProfile {
    fn on_press(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {}

    fn on_release(&self, _device_uid: String, _key: InputKey, config: &mut JukeBoxConfig) -> () {
        log::info!(
            "switching profile: {} -> {}",
            config.current_profile,
            self.profile
        );
        config.current_profile = self.profile.clone();
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::MetaSwitchProfile
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: String,
        _key: InputKey,
        config: &mut JukeBoxConfig,
    ) {
        ui.label("Profile:");
        ComboBox::from_id_salt("MetaSwitchProfileSelect")
            .selected_text(self.profile.clone())
            .width(228.0)
            .show_ui(ui, |ui| {
                for k in config.profiles.keys() {
                    if *k == config.current_profile {
                        continue;
                    }

                    if ui.selectable_label(*k == self.profile, k.clone()).clicked() {
                        self.profile = k.clone();
                    }
                }
            });
    }

    fn help(&self) -> String {
        "Switches to specified profile on release.".to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct MetaCopyFromProfile {
    profile: String,
}
#[typetag::serde]
impl Reaction for MetaCopyFromProfile {
    fn on_press(&self, device_uid: String, key: InputKey, config: &mut JukeBoxConfig) -> () {
        if let Some(p) = config.clone().profiles.get(&self.profile) {
            if let Some(d) = p.get(&device_uid) {
                if let Some(k) = d.get(&key) {
                    log::info!(
                        "COPY PRESSED: {} -> {}",
                        config.current_profile,
                        self.profile
                    );
                    k.on_press(device_uid, key, config);
                } else {
                    log::error!(
                        "failed to find action (profile {}, device {}, key {:?})",
                        self.profile,
                        device_uid,
                        key
                    )
                }
            } else {
                log::error!(
                    "failed to find device (profile {}, device {}, key {:?})",
                    self.profile,
                    device_uid,
                    key
                )
            }
        } else {
            log::error!(
                "failed to find profile (profile {}, device {}, key {:?})",
                self.profile,
                device_uid,
                key
            )
        }
    }

    fn on_release(&self, device_uid: String, key: InputKey, config: &mut JukeBoxConfig) -> () {
        if let Some(p) = config.clone().profiles.get(&self.profile) {
            if let Some(d) = p.get(&device_uid) {
                if let Some(k) = d.get(&key) {
                    log::info!(
                        "COPY RELEASED: {} -> {}",
                        config.current_profile,
                        self.profile
                    );
                    k.on_release(device_uid, key, &mut config.clone());
                } else {
                    log::error!(
                        "failed to find action (profile {}, device {}, key {:?})",
                        self.profile,
                        device_uid,
                        key
                    )
                }
            } else {
                log::error!(
                    "failed to find device (profile {}, device {}, key {:?})",
                    self.profile,
                    device_uid,
                    key
                )
            }
        } else {
            log::error!(
                "failed to find profile (profile {}, device {}, key {:?})",
                self.profile,
                device_uid,
                key
            )
        }
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::MetaCopyFromProfile
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: String,
        key: InputKey,
        config: &mut JukeBoxConfig,
    ) {
        ui.label("Profile:");
        ComboBox::from_id_salt("MetaCopyFromProfile")
            .selected_text(self.profile.clone())
            .width(228.0)
            .show_ui(ui, |ui| {
                for (k, v) in &config.profiles {
                    if *k == config.current_profile {
                        continue;
                    }
                    if v.get(&device_uid).unwrap().get(&key).unwrap().get_type()
                        == ReactionType::MetaCopyFromProfile
                    {
                        continue;
                    }

                    if ui.selectable_label(*k == self.profile, k.clone()).clicked() {
                        self.profile = k.clone();
                    }
                }
            });
    }

    fn help(&self) -> String {
        "Copies action on the same key from specified profile.".to_string()
    }
}
