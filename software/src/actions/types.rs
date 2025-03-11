// Types of actions and their associations

use std::collections::HashMap;

use anyhow::Result;
use dyn_clone::{clone_trait_object, DynClone};
use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

use crate::{
    actions::{discord::*, input::*, meta::*, obs::*, soundboard::*, system::*},
    config::JukeBoxConfig,
    gui::DeviceType,
    input::InputKey,
};

#[async_trait::async_trait]
#[typetag::serde(tag = "type")]
pub trait Action: Sync + Send + DynClone {
    async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: &mut JukeBoxConfig,
    ) -> Result<()>;

    async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: &mut JukeBoxConfig,
    ) -> Result<()>;

    fn get_type(&self) -> ActionType;

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: &String,
        input_key: InputKey,
        config: &mut JukeBoxConfig,
    );

    fn help(&self) -> String;
}
clone_trait_object!(Action);

// TODO: eventually move away from this and just use strings
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum ActionType {
    // Meta
    MetaNoAction,
    MetaSwitchProfile,
    MetaCopyFromProfile,

    // Input
    InputKeyboard,
    InputMouse,
    // InputGamepad,

    // System
    SystemLaunchApplication,
    SystemOpenWebsite,
    SystemAudioInputControl,
    SystemAudioOutputControl,

    // Soundboard
    SoundboardPlaySound,

    // Discord
    DiscordToggleMute,
    DiscordToggleDeafen,
    DiscordPushToTalk,
    DiscordPushToMute,

    // OBS
    ObsStream,
    ObsRecord,
    ObsPauseRecord,
    ObsReplayBuffer,
    ObsSaveReplay,
    ObsSaveScreenshot,
    ObsSource,
    ObsMute,
    ObsSceneSwitch,
    ObsSceneCollectionSwitch,
    ObsPreviewScene,
    ObsFilter,
    ObsTransition,
    ObsChapterMarker,
}

pub struct ActionMap {
    ui_list: Vec<(String, Vec<(ActionType, String)>)>,
    enum_map: HashMap<ActionType, Box<dyn Action>>,
}
impl ActionMap {
    pub fn new() -> Self {
        let l = vec![
            meta_action_list(),
            input_action_list(),
            system_action_list(),
            soundboard_action_list(),
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

    pub fn ui_list(&self) -> Vec<(String, Vec<(ActionType, String)>)> {
        self.ui_list.clone()
    }

    pub fn enum_new(&self, t: ActionType) -> Box<dyn Action> {
        self.enum_map.get(&t).unwrap().clone()
    }

    pub fn default_action_config(&self, d: DeviceType) -> HashMap<InputKey, Box<dyn Action>> {
        let keys = match d {
            DeviceType::Unknown => &[][..],
            DeviceType::KeyPad => &[
                InputKey::KeySwitch1,
                InputKey::KeySwitch1,
                InputKey::KeySwitch2,
                InputKey::KeySwitch3,
                InputKey::KeySwitch4,
                InputKey::KeySwitch5,
                InputKey::KeySwitch6,
                InputKey::KeySwitch7,
                InputKey::KeySwitch8,
                InputKey::KeySwitch9,
                InputKey::KeySwitch10,
                InputKey::KeySwitch11,
                InputKey::KeySwitch12,
            ][..],
            DeviceType::KnobPad => &[
                InputKey::KnobLeftSwitch,
                InputKey::KnobLeftClockwise,
                InputKey::KnobLeftCounterClockwise,
                InputKey::KnobRightSwitch,
                InputKey::KnobRightClockwise,
                InputKey::KnobRightCounterClockwise,
            ][..],
            DeviceType::PedalPad => &[
                InputKey::PedalLeft,
                InputKey::PedalMiddle,
                InputKey::PedalRight,
            ][..],
        };

        let mut c = HashMap::new();
        for k in keys {
            c.insert(
                *k,
                self.enum_map
                    .get(&ActionType::MetaNoAction)
                    .unwrap()
                    .clone(),
            );
        }

        c
    }
}
