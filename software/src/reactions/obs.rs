use std::collections::HashMap;

use egui_phosphor::regular as phos;

use super::types::{Reaction, ReactionType as RT};

#[rustfmt::skip]
pub fn obs_reaction_list() -> (String, Vec<(RT, String)>) {
    (
        format!("{} OBS", phos::VINYL_RECORD),
        vec![
            (RT::ObsStream, "Toggle Stream".to_string()),
            (RT::ObsRecord, "Toggle Record".to_string()),
            (RT::ObsPauseRecord, "Pause Recording".to_string()),
            (RT::ObsReplayBuffer, "Toggle Replay Buffer".to_string()),
            (RT::ObsSaveReplay, "Save Replay".to_string()),
            (RT::ObsSaveScreenshot, "Save Screenshot".to_string()),
            (RT::ObsSource, "Toggle Source".to_string()),
            (RT::ObsMute, "Toggle Mute Audio Source".to_string()),
            (RT::ObsSceneSwitch, "Switch to Scene".to_string()),
            (RT::ObsSceneCollectionSwitch, "Switch to Scene Collection".to_string()),
            (RT::ObsPreviewScene, "Switch to Preview Scene".to_string()),
            (RT::ObsFilter, "Toggle Filter".to_string()),
            (RT::ObsTransition, "Switch to Transition".to_string()),
            (RT::ObsChapterMarker, "Add Chapter Marker".to_string()),
        ],
    )
}

#[rustfmt::skip]
pub fn obs_enum_map() -> HashMap<RT, Box<dyn Reaction>> {
    let mut h: HashMap<RT, Box<dyn Reaction>> = HashMap::new();

    // h.insert(RT::ObsStream, Box::new(ObsStream::default()));
    // h.insert(RT::ObsRecord, Box::new(ObsRecord::default()));
    // h.insert(RT::ObsPauseRecord, Box::new(ObsPauseRecord::default()));
    // h.insert(RT::ObsReplayBuffer, Box::new(ObsReplayBuffer::default()));
    // h.insert(RT::ObsSaveReplay, Box::new(ObsSaveReplay::default()));
    // h.insert(RT::ObsSaveScreenshot, Box::new(ObsSaveScreenshot::default()));
    // h.insert(RT::ObsSource, Box::new(ObsSource::default()));
    // h.insert(RT::ObsMute, Box::new(ObsMute::default()));
    // h.insert(RT::ObsSceneSwitch, Box::new(ObsSceneSwitch::default()));
    // h.insert(RT::ObsSceneCollectionSwitch, Box::new(ObsSceneCollectionSwitch::default()));
    // h.insert(RT::ObsPreviewScene, Box::new(ObsPreviewScene::default()));
    // h.insert(RT::ObsFilter, Box::new(ObsFilter::default()));
    // h.insert(RT::ObsTransition, Box::new(ObsTransition::default()));
    // h.insert(RT::ObsChapterMarker, Box::new(ObsChapterMarker::default()));

    h
}
