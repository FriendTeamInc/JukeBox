// Types of reactions and their associations

use std::collections::HashMap;

use dyn_clone::{clone_trait_object, DynClone};
use eframe::egui::Ui;
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};

use crate::{
    config::JukeBoxConfig,
    gui::DeviceType,
    input::InputKey,
    reactions::{input::*, meta::*, soundboard::*, system::*},
};

#[typetag::serde(tag = "type")]
pub trait Reaction: Send + DynClone {
    // TODO: add result output for error reporting
    fn on_press(&self, device_uid: String, key: InputKey, config: &mut JukeBoxConfig);
    fn on_release(&self, device_uid: String, key: InputKey, config: &mut JukeBoxConfig);
    fn get_type(&self) -> ReactionType;
    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: String,
        key: InputKey,
        config: &mut JukeBoxConfig,
    );
    fn help(&self) -> String;
}
clone_trait_object!(Reaction);

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
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

pub fn reaction_enum_to_new(t: ReactionType) -> Box<dyn Reaction> {
    use ReactionType as r;
    match t {
        r::MetaNoAction => Box::new(MetaNoAction::default()),
        r::MetaSwitchProfile => Box::new(MetaSwitchProfile::default()),
        r::MetaCopyFromProfile => Box::new(MetaCopyFromProfile::default()),
        r::InputKeyboard => Box::new(InputKeyboard::default()),
        r::InputMouse => Box::new(InputMouse::default()),
        r::SystemLaunchApplication => Box::new(SystemLaunchApplication::default()),
        r::SystemOpenWebsite => Box::new(SystemOpenWebsite::default()),
        r::SystemAudioInputControl => Box::new(SystemAudioInputControl::default()),
        r::SystemAudioOutputControl => Box::new(SystemAudioOutputControl::default()),
        r::SoundboardPlaySound => Box::new(SoundboardPlaySound::default()),
        // r::DiscordToggleMute => Box::new(DiscordToggleMute::default()),
        // r::DiscordToggleDeafen => Box::new(DiscordToggleDeafen::default()),
        // r::DiscordPushToTalk => Box::new(DiscordPushToTalk::default()),
        // r::DiscordPushToMute => Box::new(DiscordPushToMute::default()),
        // r::DiscordToggleCamera => Box::new(DiscordToggleCamera::default()),
        // r::ObsStream => Box::new(ObsStream::default()),
        // r::ObsRecord => Box::new(ObsRecord::default()),
        // r::ObsPauseRecord => Box::new(ObsPauseRecord::default()),
        // r::ObsReplayBuffer => Box::new(ObsReplayBuffer::default()),
        // r::ObsSaveReplay => Box::new(ObsSaveReplay::default()),
        // r::ObsSaveScreenshot => Box::new(ObsSaveScreenshot::default()),
        // r::ObsSource => Box::new(ObsSource::default()),
        // r::ObsMute => Box::new(ObsMute::default()),
        // r::ObsSceneSwitch => Box::new(ObsSceneSwitch::default()),
        // r::ObsSceneCollectionSwitch => Box::new(ObsSceneCollectionSwitch::default()),
        // r::ObsPreviewScene => Box::new(ObsPreviewScene::default()),
        // r::ObsFilter => Box::new(ObsFilter::default()),
        // r::ObsTransition => Box::new(ObsTransition::default()),
        // r::ObsChapterMarker => Box::new(ObsChapterMarker::default()),
        _ => todo!(),
    }
}

pub fn default_reaction_config(d: DeviceType) -> HashMap<InputKey, Box<dyn Reaction>> {
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
        c.insert(*k, reaction_enum_to_new(ReactionType::MetaNoAction));
    }

    c
}

pub fn reaction_ui_list() -> Vec<(String, Vec<(ReactionType, String)>)> {
    vec![
        (
            format!("{} Meta", phos::GEAR),
            vec![
                (ReactionType::MetaNoAction, "No Action".to_string()),
                (
                    ReactionType::MetaSwitchProfile,
                    "Switch Profile".to_string(),
                ),
                (
                    ReactionType::MetaCopyFromProfile,
                    "Copy From Profile".to_string(),
                ),
            ],
        ),
        (
            format!("{} Input", phos::CURSOR_CLICK),
            vec![
                (ReactionType::InputKeyboard, "Keyboard Event".to_string()),
                (ReactionType::InputMouse, "Mouse Event".to_string()),
                // (ReactionType::InputGamepad, "Gamepad Event".to_string()),
            ],
        ),
        (
            format!("{} System", phos::DESKTOP_TOWER),
            vec![
                (
                    ReactionType::SystemLaunchApplication,
                    "Launch Application".to_string(),
                ),
                (ReactionType::SystemOpenWebsite, "Open Website".to_string()),
                (
                    ReactionType::SystemAudioInputControl,
                    "Audio Input Control".to_string(),
                ),
                (
                    ReactionType::SystemAudioOutputControl,
                    "Audio Output Control".to_string(),
                ),
            ],
        ),
        (
            format!("{} Soundboard", phos::MUSIC_NOTES),
            vec![(ReactionType::SoundboardPlaySound, "Play Sound".to_string())],
        ),
        (
            format!("{} Discord", phos::DISCORD_LOGO),
            vec![
                (ReactionType::DiscordToggleMute, "Toggle Mute".to_string()),
                (
                    ReactionType::DiscordToggleDeafen,
                    "Toggle Deafen".to_string(),
                ),
                (ReactionType::DiscordPushToTalk, "Push to Talk".to_string()),
                (ReactionType::DiscordPushToMute, "Push to Mute".to_string()),
                (
                    ReactionType::DiscordToggleCamera,
                    "Toggle Camera".to_string(),
                ),
            ],
        ),
        (
            format!("{} OBS", phos::VINYL_RECORD),
            vec![
                (ReactionType::ObsStream, "Toggle Stream".to_string()),
                (ReactionType::ObsRecord, "Toggle Record".to_string()),
                (ReactionType::ObsPauseRecord, "Pause Recording".to_string()),
                (
                    ReactionType::ObsReplayBuffer,
                    "Toggle Replay Buffer".to_string(),
                ),
                (ReactionType::ObsSaveReplay, "Save Replay".to_string()),
                (
                    ReactionType::ObsSaveScreenshot,
                    "Save Screenshot".to_string(),
                ),
                (ReactionType::ObsSource, "Toggle Source".to_string()),
                (
                    ReactionType::ObsMute,
                    "Toggle Mute Audio Source".to_string(),
                ),
                (ReactionType::ObsSceneSwitch, "Switch to Scene".to_string()),
                (
                    ReactionType::ObsSceneCollectionSwitch,
                    "Switch to Scene Collection".to_string(),
                ),
                (
                    ReactionType::ObsPreviewScene,
                    "Switch to Preview Scene".to_string(),
                ),
                (ReactionType::ObsFilter, "Toggle Filter".to_string()),
                (
                    ReactionType::ObsTransition,
                    "Switch to Transition".to_string(),
                ),
                (
                    ReactionType::ObsChapterMarker,
                    "Add Chapter Marker".to_string(),
                ),
            ],
        ),
    ]
}
