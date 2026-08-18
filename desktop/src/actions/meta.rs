use std::sync::Arc;

use eframe::egui::{include_image, ComboBox, ImageSource, Ui};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};

use crate::{
    actions::types::{ActionModuleConfig, ActionResult, ActionTrait},
    input::InputKey,
};

use super::types::Action;

pub const AMID_META: &str = "JB.Meta";
pub const AID_META_NO_ACTION: &str = "NoAction";
pub const AID_META_SWITCH_PROFILE: &str = "SwitchProfile";
// pub const AID_META_COPY_FROM_PROFILE: &str = "CopyFromProfile";

const ICON_NO_ACTION: ImageSource =
    include_image!("../../../assets/action-icons/meta-noaction.bmp");
const ICON_SWITCH_PROFILE: ImageSource =
    include_image!("../../../assets/action-icons/meta-switchprofile.bmp");
// const ICON_COPY_FROM_PROFILE: ImageSource =
//     include_image!("../../../assets/action-icons/meta-copyfromprofile.bmp");

pub fn init_actions_meta(_config: ActionModuleConfig) -> (String, Vec<Action>) {
    (
        t!("action.meta.title", icon = phos::GEAR).into(),
        vec![
            Arc::new(MetaNoAction::default()),
            Arc::new(MetaSwitchProfile::default()),
        ],
    )
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MetaNoAction {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for MetaNoAction {
    fn get_type(&self) -> &'static str {
        AID_META_NO_ACTION
    }
    fn get_module(&self) -> &'static str {
        AMID_META
    }
    fn get_title(&self) -> &'static str {
        "action.meta.no_action.title"
    }
    fn get_description(&self) -> &'static str {
        "action.meta.no_action.help"
    }

    async fn on_press(
        &mut self,
        _module_config: &mut ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        log::info!(
            "META NO ACTION: Device {} Pressed {:?} !",
            device_uid,
            input_key
        );
        Ok(())
    }

    async fn on_release(
        &mut self,
        _module_config: &mut ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        log::info!(
            "META NO ACTION: Device {} Released {:?} !",
            device_uid,
            input_key
        );
        Ok(())
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_NO_ACTION]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MetaSwitchProfile {
    pub profile: String,
}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for MetaSwitchProfile {
    fn get_type(&self) -> &'static str {
        AID_META_SWITCH_PROFILE
    }
    fn get_module(&self) -> &'static str {
        AMID_META
    }
    fn get_title(&self) -> &'static str {
        "action.meta.switch_profile.title"
    }
    fn get_description(&self) -> &'static str {
        "action.meta.switch_profile.help"
    }

    async fn on_release(
        &mut self,
        _module_config: &mut ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
    ) -> ActionResult {
        // let mut config = config.lock().await;
        // if config.profiles.contains_key(&self.profile) {
        //     config.current_profile = self.profile.clone();
        //     Ok((input_key, false))
        // } else {
        //     if self.profile.len() == 0 {
        //         Err(ActionError::new(
        //             device_uid,
        //             input_key,
        //             t!("action.meta.switch_profile.err.empty_profile"),
        //         ))
        //     } else {
        //         Err(ActionError::new(
        //             device_uid,
        //             input_key,
        //             t!(
        //                 "action.meta.switch_profile.err.profile_not_found",
        //                 profile = self.profile
        //             ),
        //         ))
        //     }
        // }
        todo!()
    }

    fn edit_ui(&mut self, _module_config: &mut ActionModuleConfig, _ui: &mut Ui) {
        // ui.label(t!("action.meta.switch_profile.profile_select"));
        // ComboBox::from_id_salt("MetaSwitchProfileSelect")
        //     .selected_text(self.profile.clone())
        //     .width(228.0)
        //     .show_ui(ui, |ui| {
        //         let config = config.blocking_lock();
        //         for k in config.profiles.keys() {
        //             if *k == config.current_profile {
        //                 continue;
        //             }

        //             if ui.selectable_label(*k == self.profile, k.clone()).clicked() {
        //                 self.profile = k.clone();
        //             }
        //         }
        //     });
        todo!()
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_SWITCH_PROFILE]
    }
}
