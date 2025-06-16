use std::sync::Arc;

use eframe::egui::{include_image, ComboBox, ImageSource, Ui};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Action, ActionError};

pub const AID_META_NO_ACTION: &str = "MetaNoAction";
pub const AID_META_SWITCH_PROFILE: &str = "MetaSwitchProfile";
#[allow(dead_code)]
pub const AID_META_COPY_FROM_PROFILE: &str = "MetaCopyFromProfile";

const ICON_NO_ACTION: ImageSource =
    include_image!("../../../assets/action-icons/meta-noaction.bmp");
const ICON_SWITCH_PROFILE: ImageSource =
    include_image!("../../../assets/action-icons/meta-switchprofile.bmp");
#[allow(dead_code)]
const ICON_COPY_FROM_PROFILE: ImageSource =
    include_image!("../../../assets/action-icons/meta-copyfromprofile.bmp");

#[rustfmt::skip]
pub fn meta_action_list() -> (String, Vec<(String, Action, String)>) {
    (
        t!("action.meta.title", icon = phos::GEAR).into(),
        vec![
            (AID_META_NO_ACTION.into(),         Action::MetaNoAction(MetaNoAction::default()),               t!("action.meta.no_action.title").into()),
            (AID_META_SWITCH_PROFILE.into(),    Action::MetaSwitchProfile(MetaSwitchProfile::default()),     t!("action.meta.switch_profile.title").into()),
            // (AID_META_COPY_FROM_PROFILE.into(), Action::MetaCopyFromProfile(MetaCopyFromProfile::default()), t!("action.meta.copy_from_profile.title").into()),
        ],
    )
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MetaNoAction {}
impl MetaNoAction {
    pub async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        log::info!(
            "META NO ACTION: Device {} Pressed {:?} !",
            device_uid,
            input_key
        );
        Ok(())
    }

    pub async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        log::info!(
            "META NO ACTION: Device {} Released {:?} !",
            device_uid,
            input_key
        );
        Ok(())
    }

    pub fn get_type(&self) -> String {
        AID_META_NO_ACTION.into()
    }

    pub fn edit_ui(
        &mut self,
        _ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
    }

    pub fn help(&self) -> String {
        t!("action.meta.no_action.help").into()
    }

    pub fn icon_source(&self) -> ImageSource {
        ICON_NO_ACTION
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MetaSwitchProfile {
    pub profile: String,
}
impl MetaSwitchProfile {
    pub async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    pub async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        let mut config = config.lock().await;
        if config.profiles.contains_key(&self.profile) {
            config.current_profile = self.profile.clone();
            // TODO: send command to device to change hardware inputs?
            Ok(())
        } else {
            if self.profile.len() == 0 {
                Err(ActionError::new(
                    device_uid,
                    input_key,
                    t!("action.meta.switch_profile.err.empty_profile"),
                ))
            } else {
                Err(ActionError::new(
                    device_uid,
                    input_key,
                    t!(
                        "action.meta.switch_profile.err.profile_not_found",
                        profile = self.profile
                    ),
                ))
            }
        }
    }

    pub fn get_type(&self) -> String {
        AID_META_SWITCH_PROFILE.into()
    }

    pub fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
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

    pub fn help(&self) -> String {
        t!("action.meta.switch_profile.help").into()
    }

    pub fn icon_source(&self) -> ImageSource {
        ICON_SWITCH_PROFILE
    }
}

// #[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
// pub struct MetaCopyFromProfile {
//     profile: String,
// }
// impl MetaCopyFromProfile {
//     pub async fn on_press(
//         &self,
//         device_uid: &String,
//         input_key: InputKey,
//         config: Arc<Mutex<JukeBoxConfig>>,
//     ) -> Result<(), ActionError> {
//         let k = config
//             .lock()
//             .await
//             .profiles
//             .get(&self.profile)
//             .and_then(|p| p.get(device_uid))
//             .and_then(|d| d.key_map.get(&input_key))
//             .map(|k| k.clone());
//         if let Some(k) = k {
//             k.action.on_press(device_uid, input_key, config).await
//         } else {
//             Err(ActionError::new(
//                 device_uid,
//                 input_key,
//                 t!(
//                     "action.meta.copy_from_profile.err.action_not_found",
//                     profile = self.profile
//                 ),
//             ))
//         }
//     }

//     pub async fn on_release(
//         &self,
//         device_uid: &String,
//         input_key: InputKey,
//         config: Arc<Mutex<JukeBoxConfig>>,
//     ) -> Result<(), ActionError> {
//         let k = config
//             .lock()
//             .await
//             .profiles
//             .get(&self.profile)
//             .and_then(|p| p.get(device_uid))
//             .and_then(|d| d.key_map.get(&input_key))
//             .map(|k| k.clone());
//         if let Some(k) = k {
//             k.action.on_release(device_uid, input_key, config).await
//         } else {
//             Err(ActionError::new(
//                 device_uid,
//                 input_key,
//                 t!(
//                     "action.meta.copy_from_profile.err.action_not_found",
//                     profile = self.profile
//                 ),
//             ))
//         }
//     }

//     pub fn get_type(&self) -> String {
//         AID_META_COPY_FROM_PROFILE.into()
//     }

//     pub fn edit_ui(
//         &mut self,
//         ui: &mut Ui,
//         device_uid: &String,
//         input_key: InputKey,
//         config: Arc<Mutex<JukeBoxConfig>>,
//     ) {
//         ui.label(t!("action.meta.copy_from_profile.profile_select"));
//         ComboBox::from_id_salt("MetaCopyFromProfile")
//             .selected_text(self.profile.clone())
//             .width(228.0)
//             .show_ui(ui, |ui| {
//                 let config = config.blocking_lock();
//                 for (k, v) in &config.profiles {
//                     if *k == config.current_profile {
//                         continue;
//                     }
//                     if v.get(device_uid)
//                         .unwrap()
//                         .key_map
//                         .get(&input_key)
//                         .unwrap()
//                         .action
//                         .get_type()
//                         == AID_META_COPY_FROM_PROFILE
//                     {
//                         continue;
//                     }
//                     ui.selectable_value(&mut self.profile, k.clone(), k.clone());

//                     // if ui.selectable_label(*k == self.profile, k.clone()).clicked() {
//                     //     self.profile = k.clone();
//                     // }
//                 }
//             });
//     }

//     pub fn help(&self) -> String {
//         t!("action.meta.copy_from_profile.help").into()
//     }

//     pub fn icon_source(&self) -> ImageSource {
//         ICON_COPY_FROM_PROFILE
//     }
// }
