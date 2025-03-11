use std::collections::HashMap;

use egui_phosphor::regular as phos;

use super::types::{Action, ActionType as AT};

#[rustfmt::skip]
pub fn obs_action_list() -> (String, Vec<(AT, String)>) {
    (
        format!("{} OBS", phos::VINYL_RECORD),
        vec![
            (AT::ObsStream, "Toggle Stream".to_string()),
            (AT::ObsRecord, "Toggle Record".to_string()),
            (AT::ObsPauseRecord, "Pause Recording".to_string()),
            (AT::ObsReplayBuffer, "Toggle Replay Buffer".to_string()),
            (AT::ObsSaveReplay, "Save Replay".to_string()),
            (AT::ObsSaveScreenshot, "Save Screenshot".to_string()),
            (AT::ObsSource, "Toggle Source".to_string()),
            (AT::ObsMute, "Toggle Mute Audio Source".to_string()),
            (AT::ObsSceneSwitch, "Switch to Scene".to_string()),
            (AT::ObsSceneCollectionSwitch, "Switch to Scene Collection".to_string()),
            (AT::ObsPreviewScene, "Switch to Preview Scene".to_string()),
            (AT::ObsFilter, "Toggle Filter".to_string()),
            (AT::ObsTransition, "Switch to Transition".to_string()),
            (AT::ObsChapterMarker, "Add Chapter Marker".to_string()),
        ],
    )
}

#[rustfmt::skip]
pub fn obs_enum_map() -> HashMap<AT, Box<dyn Action>> {
    let mut h: HashMap<AT, Box<dyn Action>> = HashMap::new();

    // h.inseat(AT::ObsStream, Box::new(ObsStream::default()));
    // h.inseat(AT::ObsRecord, Box::new(ObsRecord::default()));
    // h.inseat(AT::ObsPauseRecord, Box::new(ObsPauseRecord::default()));
    // h.inseat(AT::ObsReplayBuffer, Box::new(ObsReplayBuffer::default()));
    // h.inseat(AT::ObsSaveReplay, Box::new(ObsSaveReplay::default()));
    // h.inseat(AT::ObsSaveScreenshot, Box::new(ObsSaveScreenshot::default()));
    // h.inseat(AT::ObsSource, Box::new(ObsSource::default()));
    // h.inseat(AT::ObsMute, Box::new(ObsMute::default()));
    // h.inseat(AT::ObsSceneSwitch, Box::new(ObsSceneSwitch::default()));
    // h.inseat(AT::ObsSceneCollectionSwitch, Box::new(ObsSceneCollectionSwitch::default()));
    // h.inseat(AT::ObsPreviewScene, Box::new(ObsPreviewScene::default()));
    // h.inseat(AT::ObsFilter, Box::new(ObsFilter::default()));
    // h.inseat(AT::ObsTransition, Box::new(ObsTransition::default()));
    // h.inseat(AT::ObsChapterMarker, Box::new(ObsChapterMarker::default()));

    h
}
