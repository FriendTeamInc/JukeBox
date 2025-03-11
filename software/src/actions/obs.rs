use egui_phosphor::regular as phos;

use super::{
    meta::MetaNoAction,
    types::{Action, ActionType as AT},
};

#[rustfmt::skip]
pub fn obs_action_list() -> (String, Vec<(AT, Box<dyn Action>, String)>) {
    (
        format!("{} OBS", phos::VINYL_RECORD),
        vec![
            (AT::ObsStream,                Box::new(MetaNoAction::default()), "Toggle Stream".to_string()),
            (AT::ObsRecord,                Box::new(MetaNoAction::default()), "Toggle Record".to_string()),
            (AT::ObsPauseRecord,           Box::new(MetaNoAction::default()), "Pause Recording".to_string()),
            (AT::ObsReplayBuffer,          Box::new(MetaNoAction::default()), "Toggle Replay Buffer".to_string()),
            (AT::ObsSaveReplay,            Box::new(MetaNoAction::default()), "Save Replay".to_string()),
            (AT::ObsSaveScreenshot,        Box::new(MetaNoAction::default()), "Save Screenshot".to_string()),
            (AT::ObsSource,                Box::new(MetaNoAction::default()), "Toggle Source".to_string()),
            (AT::ObsMute,                  Box::new(MetaNoAction::default()), "Toggle Mute Audio Source".to_string()),
            (AT::ObsSceneSwitch,           Box::new(MetaNoAction::default()), "Switch to Scene".to_string()),
            (AT::ObsSceneCollectionSwitch, Box::new(MetaNoAction::default()), "Switch to Scene Collection".to_string()),
            (AT::ObsPreviewScene,          Box::new(MetaNoAction::default()), "Switch to Preview Scene".to_string()),
            (AT::ObsFilter,                Box::new(MetaNoAction::default()), "Toggle Filter".to_string()),
            (AT::ObsTransition,            Box::new(MetaNoAction::default()), "Switch to Transition".to_string()),
            (AT::ObsChapterMarker,         Box::new(MetaNoAction::default()), "Add Chapter Marker".to_string()),

            // (AT::ObsStream,                Box::new(ObsStream::default()),                "Toggle Stream".to_string()),
            // (AT::ObsRecord,                Box::new(ObsRecord::default()),                "Toggle Record".to_string()),
            // (AT::ObsPauseRecord,           Box::new(ObsPauseRecord::default()),           "Pause Recording".to_string()),
            // (AT::ObsReplayBuffer,          Box::new(ObsReplayBuffer::default()),          "Toggle Replay Buffer".to_string()),
            // (AT::ObsSaveReplay,            Box::new(ObsSaveReplay::default()),            "Save Replay".to_string()),
            // (AT::ObsSaveScreenshot,        Box::new(ObsSaveScreenshot::default()),        "Save Screenshot".to_string()),
            // (AT::ObsSource,                Box::new(ObsSource::default()),                "Toggle Source".to_string()),
            // (AT::ObsMute,                  Box::new(ObsMute::default()),                  "Toggle Mute Audio Source".to_string()),
            // (AT::ObsSceneSwitch,           Box::new(ObsSceneSwitch::default()),           "Switch to Scene".to_string()),
            // (AT::ObsSceneCollectionSwitch, Box::new(ObsSceneCollectionSwitch::default()), "Switch to Scene Collection".to_string()),
            // (AT::ObsPreviewScene,          Box::new(ObsPreviewScene::default()),          "Switch to Preview Scene".to_string()),
            // (AT::ObsFilter,                Box::new(ObsFilter::default()),                "Toggle Filter".to_string()),
            // (AT::ObsTransition,            Box::new(ObsTransition::default()),            "Switch to Transition".to_string()),
            // (AT::ObsChapterMarker,         Box::new(ObsChapterMarker::default()),         "Add Chapter Marker".to_string()),
        ],
    )
}
