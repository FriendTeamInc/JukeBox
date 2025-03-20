// Types of actions and their associations

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use dyn_clone::{clone_trait_object, DynClone};
use eframe::egui::Ui;
use egui_phosphor::regular as phos;
use jukebox_util::peripheral::DeviceType;
use tokio::sync::Mutex;

use crate::{
    actions::{input::*, meta::*, obs::*, soundboard::*, system::*},
    config::{ActionConfig, ActionIcon, JukeBoxConfig},
    input::InputKey,
};

#[cfg(feature = "discord")]
use crate::actions::discord::*;

#[async_trait::async_trait]
#[typetag::serde(tag = "type")]
pub trait Action: Sync + Send + DynClone {
    async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()>;

    async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()>;

    fn get_type(&self) -> String;

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    );

    fn help(&self) -> String;
}
clone_trait_object!(Action);

pub struct ActionMap {
    ui_list: Vec<(String, Vec<(String, String)>)>,
    enum_map: HashMap<String, Box<dyn Action>>,
}
impl ActionMap {
    pub fn new() -> Self {
        let l = vec![
            meta_action_list(),
            input_action_list(),
            system_action_list(),
            soundboard_action_list(),
            #[cfg(feature = "discord")]
            discord_action_list(),
            obs_action_list(),
        ];

        let ui_list = l
            .iter()
            .map(|(title, l)| {
                (
                    title.clone(),
                    l.iter().map(|(at, _, s)| (at.clone(), s.clone())).collect(),
                )
            })
            .collect();

        let enum_map = l
            .iter()
            .map(|(_, l)| l)
            .flatten()
            .map(|(at, a, _)| (at.clone(), a.clone()))
            .collect();

        Self { ui_list, enum_map }
    }

    pub fn ui_list(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.ui_list.clone()
    }

    pub fn enum_new(&self, t: String) -> Box<dyn Action> {
        self.enum_map.get(&t).unwrap().clone()
    }

    pub fn default_action_config(&self, d: DeviceType) -> HashMap<InputKey, ActionConfig> {
        use InputKey as IK;
        let keys = match d {
            DeviceType::Unknown => &[][..],
            DeviceType::KeyPad => &[
                (IK::KeySwitch1, "F13"),
                (IK::KeySwitch2, "F14"),
                (IK::KeySwitch3, "F15"),
                (IK::KeySwitch4, "F16"),
                (IK::KeySwitch5, "F17"),
                (IK::KeySwitch6, "F18"),
                (IK::KeySwitch7, "F19"),
                (IK::KeySwitch8, "F20"),
                (IK::KeySwitch9, "F21"),
                (IK::KeySwitch10, "F22"),
                (IK::KeySwitch11, "F23"),
                (IK::KeySwitch12, "F24"),
            ][..],
            DeviceType::KnobPad => &[
                (IK::KnobLeftSwitch, phos::ARROW_CIRCLE_DOWN),
                (IK::KnobLeftClockwise, phos::ARROW_CLOCKWISE),
                (IK::KnobLeftCounterClockwise, phos::ARROW_COUNTER_CLOCKWISE),
                (IK::KnobRightSwitch, phos::ARROW_CIRCLE_DOWN),
                (IK::KnobRightClockwise, phos::ARROW_CLOCKWISE),
                (IK::KnobRightCounterClockwise, phos::ARROW_COUNTER_CLOCKWISE),
            ][..],
            DeviceType::PedalPad => &[
                (IK::PedalLeft, phos::ALIGN_LEFT_SIMPLE),
                (IK::PedalMiddle, phos::ALIGN_BOTTOM_SIMPLE),
                (IK::PedalRight, phos::ALIGN_RIGHT_SIMPLE),
            ][..],
        };

        let mut c = HashMap::new();
        for k in keys {
            c.insert(
                k.0,
                ActionConfig {
                    action: self.enum_map.get("MetaNoAction").unwrap().clone(),
                    icon: ActionIcon::GlyphIcon(k.1.into()),
                },
            );
        }

        c
    }
}
