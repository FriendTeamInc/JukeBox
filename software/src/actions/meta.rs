use std::sync::Arc;

use anyhow::{bail, Result};
use eframe::egui::{ComboBox, Ui};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Action, ActionType as AT};

#[rustfmt::skip]
pub fn meta_action_list() -> (String, Vec<(AT, Box<dyn Action>, String)>) {
    (
        t!("action.meta.title", icon = phos::GEAR).to_string(),
        vec![
            (AT::MetaNoAction,        Box::new(MetaNoAction::default()),        t!("action.meta.no_action.title").to_string()),
            (AT::MetaSwitchProfile,   Box::new(MetaSwitchProfile::default()),   t!("action.meta.switch_profile.title").to_string()),
            (AT::MetaCopyFromProfile, Box::new(MetaCopyFromProfile::default()), t!("action.meta.copy_from_profile.title").to_string()),
        ],
    )
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct MetaNoAction {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for MetaNoAction {
    async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        log::info!(
            "META NO ACTION: Device {} Pressed {:?} !",
            device_uid,
            input_key
        );
        Ok(())
    }

    async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        log::info!(
            "META NO ACTION: Device {} Released {:?} !",
            device_uid,
            input_key
        );
        Ok(())
    }

    fn get_type(&self) -> AT {
        AT::MetaNoAction
    }

    fn edit_ui(
        &mut self,
        _ui: &mut Ui,
        _device_uid: &String,
        _input_input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
    }

    fn help(&self) -> String {
        t!("action.meta.no_action.help").to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct MetaSwitchProfile {
    profile: String,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for MetaSwitchProfile {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let mut config = config.lock().await;
        if config.profiles.contains_key(&self.profile) {
            config.current_profile = self.profile.clone();
            Ok(())
        } else {
            bail!(t!(
                "action.meta.switch_profile.error_msg",
                profile = self.profile
            ))
        }
    }

    fn get_type(&self) -> AT {
        AT::MetaSwitchProfile
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.label(t!("action.meta.switch_profile.profile_select"));
        ComboBox::from_id_salt("MetaSwitchProfileSelect")
            .selected_text(self.profile.clone())
            .width(228.0)
            .show_ui(ui, |ui| {
                let config = config.blocking_lock();
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
        t!("action.meta.switch_profile.help").to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct MetaCopyFromProfile {
    profile: String,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for MetaCopyFromProfile {
    async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let k = config
            .lock()
            .await
            .profiles
            .get(&self.profile)
            .and_then(|p| p.get(device_uid))
            .and_then(|d| d.get(&input_key))
            .and_then(|k| Some(k.clone()));
        if let Some(k) = k {
            k.on_press(device_uid, input_key, config).await
        } else {
            bail!(t!(
                "action.meta.copy_from_profile.error_profile_msg",
                profile = self.profile,
                uid = device_uid,
                key = input_key
            ))
        }
    }

    async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let k = config
            .lock()
            .await
            .profiles
            .get(&self.profile)
            .and_then(|p| p.get(device_uid))
            .and_then(|d| d.get(&input_key))
            .and_then(|k| Some(k.clone()));
        if let Some(k) = k {
            k.on_release(device_uid, input_key, config).await
        } else {
            bail!(t!(
                "action.meta.copy_from_profile.error_profile_msg",
                profile = self.profile,
                uid = device_uid,
                key = input_key
            ))
        }
    }

    fn get_type(&self) -> AT {
        AT::MetaCopyFromProfile
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.label(t!("action.meta.copy_from_profile.profile_select"));
        ComboBox::from_id_salt("MetaCopyFromProfile")
            .selected_text(self.profile.clone())
            .width(228.0)
            .show_ui(ui, |ui| {
                let config = config.blocking_lock();
                for (k, v) in &config.profiles {
                    if *k == config.current_profile {
                        continue;
                    }
                    if v.get(device_uid)
                        .unwrap()
                        .get(&input_key)
                        .unwrap()
                        .get_type()
                        == AT::MetaCopyFromProfile
                    {
                        continue;
                    }
                    ui.selectable_value(&mut self.profile, k.clone(), k.clone());

                    // if ui.selectable_label(*k == self.profile, k.clone()).clicked() {
                    //     self.profile = k.clone();
                    // }
                }
            });
    }

    fn help(&self) -> String {
        t!("action.meta.copy_from_profile.help").to_string()
    }
}
