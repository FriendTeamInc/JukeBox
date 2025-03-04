// Types of reactions and their associations

use dyn_clone::{clone_trait_object, DynClone};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};

use crate::input::InputKey;

use super::meta::ReactionMetaTest;

#[typetag::serde(tag = "type")]
pub trait Reaction: Send + DynClone {
    // TODO: add result output for error reporting
    fn on_press(&self, key: InputKey);
    fn on_release(&self, key: InputKey);
    fn get_type(&self) -> ReactionType;
}
clone_trait_object!(Reaction);

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum ReactionType {
    // Self
    Unknown,

    // Meta
    MetaTest,
    MetaSwitchProfile,
    MetaCopyFromProfile,

    // Input
    InputPressKey,
    InputClickMouse,
    InputMoveMouse,
    InputScrollMouse,

    // System
    SystemLaunch,
    SystemWebsite,
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
    Box::new(match t {
        ReactionType::MetaTest => ReactionMetaTest::default(),
        _ => todo!(),
    })
}

pub fn reaction_ui_list() -> Vec<(String, Vec<(ReactionType, String)>)> {
    vec![
        (
            format!("{} Meta", phos::GEAR),
            vec![
                (ReactionType::MetaTest, "Test Key".to_string()),
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
                (ReactionType::InputPressKey, "Press Key".to_string()),
                (ReactionType::InputClickMouse, "Click Mouse".to_string()),
                (ReactionType::InputMoveMouse, "Move Mouse".to_string()),
                (ReactionType::InputScrollMouse, "Scroll Mouse".to_string()),
            ],
        ),
        (
            format!("{} System", phos::DESKTOP_TOWER),
            vec![
                (ReactionType::SystemLaunch, "Launch Application".to_string()),
                (ReactionType::SystemWebsite, "Open Website".to_string()),
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
