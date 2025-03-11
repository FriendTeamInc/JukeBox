use egui_phosphor::regular as phos;

use super::{
    meta::MetaNoAction,
    types::{Action, ActionType as AT},
};

#[rustfmt::skip]
pub fn obs_action_list() -> (String, Vec<(AT, Box<dyn Action>, String)>) {
    (
        t!("action.obs.title", icon = phos::VINYL_RECORD).to_string(),
        vec![
            (AT::ObsStream,                Box::new(MetaNoAction::default()), t!("action.obs.toggle_stream.title").to_string()),
            (AT::ObsRecord,                Box::new(MetaNoAction::default()), t!("action.obs.toggle_record.title").to_string()),
            (AT::ObsPauseRecord,           Box::new(MetaNoAction::default()), t!("action.obs.pause_record.title").to_string()),
            (AT::ObsReplayBuffer,          Box::new(MetaNoAction::default()), t!("action.obs.toggle_replay_buffer.title").to_string()),
            (AT::ObsSaveReplay,            Box::new(MetaNoAction::default()), t!("action.obs.save_replay_buffer.title").to_string()),
            (AT::ObsSaveScreenshot,        Box::new(MetaNoAction::default()), t!("action.obs.save_screenshot.title").to_string()),
            (AT::ObsSource,                Box::new(MetaNoAction::default()), t!("action.obs.toggle_source.title").to_string()),
            (AT::ObsMute,                  Box::new(MetaNoAction::default()), t!("action.obs.toggle_mute_audio_source.title").to_string()),
            (AT::ObsSceneSwitch,           Box::new(MetaNoAction::default()), t!("action.obs.switch_scene.title").to_string()),
            (AT::ObsSceneCollectionSwitch, Box::new(MetaNoAction::default()), t!("action.obs.switch_scene_collection.title").to_string()),
            (AT::ObsPreviewScene,          Box::new(MetaNoAction::default()), t!("action.obs.switch_preview_scene.title").to_string()),
            (AT::ObsFilter,                Box::new(MetaNoAction::default()), t!("action.obs.toggle_filter.title").to_string()),
            (AT::ObsTransition,            Box::new(MetaNoAction::default()), t!("action.obs.switch_transition.title").to_string()),
            (AT::ObsChapterMarker,         Box::new(MetaNoAction::default()), t!("action.obs.add_chapter_marker.title").to_string()),

            // (AT::ObsStream,                Box::new(ObsStream::default()),                t!("action.obs.toggle_stream.title").to_string()),
            // (AT::ObsRecord,                Box::new(ObsRecord::default()),                t!("action.obs.toggle_record.title").to_string()),
            // (AT::ObsPauseRecord,           Box::new(ObsPauseRecord::default()),           t!("action.obs.pause_record.title").to_string()),
            // (AT::ObsReplayBuffer,          Box::new(ObsReplayBuffer::default()),          t!("action.obs.toggle_replay_buffer.title").to_string()),
            // (AT::ObsSaveReplay,            Box::new(ObsSaveReplay::default()),            t!("action.obs.save_replay_buffer.title").to_string()),
            // (AT::ObsSaveScreenshot,        Box::new(ObsSaveScreenshot::default()),        t!("action.obs.save_screenshot.title").to_string()),
            // (AT::ObsSource,                Box::new(ObsSource::default()),                t!("action.obs.toggle_source.title").to_string()),
            // (AT::ObsMute,                  Box::new(ObsMute::default()),                  t!("action.obs.toggle_mute_audio_source.title").to_string()),
            // (AT::ObsSceneSwitch,           Box::new(ObsSceneSwitch::default()),           t!("action.obs.switch_scene.title").to_string()),
            // (AT::ObsSceneCollectionSwitch, Box::new(ObsSceneCollectionSwitch::default()), t!("action.obs.switch_scene_collection.title").to_string()),
            // (AT::ObsPreviewScene,          Box::new(ObsPreviewScene::default()),          t!("action.obs.switch_preview_scene.title").to_string()),
            // (AT::ObsFilter,                Box::new(ObsFilter::default()),                t!("action.obs.toggle_filter.title").to_string()),
            // (AT::ObsTransition,            Box::new(ObsTransition::default()),            t!("action.obs.switch_transition.title").to_string()),
            // (AT::ObsChapterMarker,         Box::new(ObsChapterMarker::default()),         t!("action.obs.add_chapter_marker.title").to_string()),
        ],
    )
}
