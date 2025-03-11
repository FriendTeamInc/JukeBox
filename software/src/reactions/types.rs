// Types of reactions and their associations

use std::collections::HashMap;

use anyhow::Result;
use dyn_clone::{clone_trait_object, DynClone};
use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

use crate::{
    config::JukeBoxConfig,
    gui::DeviceType,
    input::InputKey,
    reactions::{discord::*, input::*, meta::*, obs::*, soundboard::*, system::*},
};

#[async_trait::async_trait]
#[typetag::serde(tag = "type")]
pub trait Reaction: Sync + Send + DynClone {
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

    fn get_type(&self) -> ReactionType;

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: &String,
        input_key: InputKey,
        config: &mut JukeBoxConfig,
    );

    fn help(&self) -> String;
}
clone_trait_object!(Reaction);

// TODO: eventually move away from this and just use strings
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum ReactionType {
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
    DiscordToggleCamera,

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

pub struct ReactionMap {
    ui_list: Vec<(String, Vec<(ReactionType, String)>)>,
    enum_map: HashMap<ReactionType, Box<dyn Reaction>>,
}
impl ReactionMap {
    pub fn new() -> Self {
        let ui_list = vec![
            meta_reaction_list(),
            input_reaction_list(),
            system_reaction_list(),
            soundboard_reaction_list(),
            discord_reaction_list(),
            obs_reaction_list(),
        ];
        let enum_map = HashMap::new()
            .into_iter()
            .chain(meta_enum_map())
            .chain(input_enum_map())
            .chain(system_enum_map())
            .chain(soundboard_enum_map())
            .chain(discord_enum_map())
            .chain(obs_enum_map())
            .collect();

        Self { ui_list, enum_map }
    }

    pub fn ui_list(&self) -> Vec<(String, Vec<(ReactionType, String)>)> {
        self.ui_list.clone()
    }

    pub fn enum_new(&self, t: ReactionType) -> Box<dyn Reaction> {
        self.enum_map.get(&t).unwrap().clone()
    }

    pub fn default_reaction_config(&self, d: DeviceType) -> HashMap<InputKey, Box<dyn Reaction>> {
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
                    .get(&ReactionType::MetaNoAction)
                    .unwrap()
                    .clone(),
            );
        }

        c
    }
}

// pub fn reaction_enum_to_new(t: ReactionType) -> Box<dyn Reaction> {
//     use ReactionType as r;
//     match t {
//         // r::MetaNoAction => Box::new(MetaNoAction::default()),
//         // r::MetaSwitchProfile => Box::new(MetaSwitchProfile::default()),
//         // r::MetaCopyFromProfile => Box::new(MetaCopyFromProfile::default()),
//         // r::InputKeyboard => Box::new(InputKeyboard::default()),
//         // r::InputMouse => Box::new(InputMouse::default()),
//         // r::SystemLaunchApplication => Box::new(SystemLaunchApplication::default()),
//         // r::SystemOpenWebsite => Box::new(SystemOpenWebsite::default()),
//         // r::SystemAudioInputControl => Box::new(SystemAudioInputControl::default()),
//         // r::SystemAudioOutputControl => Box::new(SystemAudioOutputControl::default()),
//         // r::SoundboardPlaySound => Box::new(SoundboardPlaySound::default()),
//         // r::DiscordToggleMute => Box::new(DiscordToggleMute::default()),
//         // r::DiscordToggleDeafen => Box::new(DiscordToggleDeafen::default()),
//         // r::DiscordPushToTalk => Box::new(DiscordPushToTalk::default()),
//         // r::DiscordPushToMute => Box::new(DiscordPushToMute::default()),
//         // r::DiscordToggleCamera => Box::new(DiscordToggleCamera::default()),
//         // r::ObsStream => Box::new(ObsStream::default()),
//         // r::ObsRecord => Box::new(ObsRecord::default()),
//         // r::ObsPauseRecord => Box::new(ObsPauseRecord::default()),
//         // r::ObsReplayBuffer => Box::new(ObsReplayBuffer::default()),
//         // r::ObsSaveReplay => Box::new(ObsSaveReplay::default()),
//         // r::ObsSaveScreenshot => Box::new(ObsSaveScreenshot::default()),
//         // r::ObsSource => Box::new(ObsSource::default()),
//         // r::ObsMute => Box::new(ObsMute::default()),
//         // r::ObsSceneSwitch => Box::new(ObsSceneSwitch::default()),
//         // r::ObsSceneCollectionSwitch => Box::new(ObsSceneCollectionSwitch::default()),
//         // r::ObsPreviewScene => Box::new(ObsPreviewScene::default()),
//         // r::ObsFilter => Box::new(ObsFilter::default()),
//         // r::ObsTransition => Box::new(ObsTransition::default()),
//         // r::ObsChapterMarker => Box::new(ObsChapterMarker::default()),
//         _ => todo!(),
//     }
// }

// pub fn reaction_ui_list() -> Vec<(String, Vec<(ReactionType, String)>)> {
//     vec![
//         meta_reaction_list(),
//         input_reaction_list(),
//         system_reaction_list(),
//         soundboard_reaction_list(),
//         discord_reaction_list(),
//         obs_reaction_list(),
//     ]
// }
