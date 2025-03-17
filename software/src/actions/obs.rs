use std::sync::{Arc, OnceLock};

use anyhow::{bail, Result};
use eframe::egui::Ui;
use egui_phosphor::regular as phos;
use obws::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{config::JukeBoxConfig, input::InputKey};

use super::{
    meta::MetaNoAction,
    types::{Action, ActionType as AT},
};

static OBS_CLIENT: OnceLock<Mutex<Client>> = OnceLock::new();

#[rustfmt::skip]
pub fn obs_action_list() -> (String, Vec<(AT, Box<dyn Action>, String)>) {
    (
        t!("action.obs.title", icon = phos::VINYL_RECORD).to_string(),
        vec![
            (AT::ObsStream,                Box::new(ObsStream::default()),        t!("action.obs.toggle_stream.title").to_string()),
            (AT::ObsRecord,                Box::new(ObsRecord::default()),        t!("action.obs.toggle_record.title").to_string()),
            (AT::ObsPauseRecord,           Box::new(ObsPauseRecord::default()),   t!("action.obs.pause_record.title").to_string()),
            (AT::ObsReplayBuffer,          Box::new(MetaNoAction::default()),     t!("action.obs.toggle_replay_buffer.title").to_string()),
            (AT::ObsSaveReplay,            Box::new(MetaNoAction::default()),     t!("action.obs.save_replay_buffer.title").to_string()),
            (AT::ObsSaveScreenshot,        Box::new(MetaNoAction::default()),     t!("action.obs.save_screenshot.title").to_string()),
            (AT::ObsSource,                Box::new(MetaNoAction::default()),     t!("action.obs.toggle_source.title").to_string()),
            (AT::ObsMute,                  Box::new(MetaNoAction::default()),     t!("action.obs.toggle_mute_audio_source.title").to_string()),
            (AT::ObsSceneSwitch,           Box::new(MetaNoAction::default()),     t!("action.obs.switch_scene.title").to_string()),
            (AT::ObsSceneCollectionSwitch, Box::new(MetaNoAction::default()),     t!("action.obs.switch_scene_collection.title").to_string()),
            (AT::ObsPreviewScene,          Box::new(MetaNoAction::default()),     t!("action.obs.switch_preview_scene.title").to_string()),
            (AT::ObsFilter,                Box::new(MetaNoAction::default()),     t!("action.obs.toggle_filter.title").to_string()),
            (AT::ObsTransition,            Box::new(MetaNoAction::default()),     t!("action.obs.switch_transition.title").to_string()),
            (AT::ObsChapterMarker,         Box::new(ObsChapterMarker::default()), t!("action.obs.add_chapter_marker.title").to_string()),
        ],
    )
}

async fn create_client(config: Arc<Mutex<JukeBoxConfig>>) -> Result<()> {
    let config = config.blocking_lock().clone();
    if let Some(o) = config.obs_access {
        // TODO: connect with config for tls and a smaller connect_timeout
        let client = Client::connect(o.host, o.port, o.password).await?;
        match OBS_CLIENT.set(Mutex::new(client)) {
            Ok(_) => {}
            Err(_) => {
                bail!("already set obs credentials")
            }
        }
        // .expect("failed to set OBS_CLIENT");
        Ok(())
    } else {
        bail!("failed to find obs credentials in config to create client");
    }
}

fn account_warning(_ui: &mut Ui, _config: Arc<Mutex<JukeBoxConfig>>) {
    // "Connect to OBS-Websocket to use this action!"
    // "Open OBS, go to Tools, then WebSocket Server Settings."
    // "Enable the WebSocket server, and copy the password into here."
    todo!()
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsStream {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsStream {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if OBS_CLIENT.get().is_none() {
            create_client(c).await?;
        }

        let client = OBS_CLIENT.get().unwrap().lock().await;
        client.streaming().toggle().await?;

        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> AT {
        AT::ObsStream
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, config);
    }

    fn help(&self) -> String {
        t!("action.obs.toggle_stream.help").to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsRecord {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsRecord {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if OBS_CLIENT.get().is_none() {
            create_client(c).await?;
        }

        let client = OBS_CLIENT.get().unwrap().lock().await;
        client.recording().toggle().await?;

        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> AT {
        AT::ObsRecord
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, config);
    }

    fn help(&self) -> String {
        t!("action.obs.toggle_record.help").to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsPauseRecord {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsPauseRecord {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if OBS_CLIENT.get().is_none() {
            create_client(c).await?;
        }

        let client = OBS_CLIENT.get().unwrap().lock().await;
        client.recording().toggle_pause().await?;

        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> AT {
        AT::ObsPauseRecord
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, config);
    }

    fn help(&self) -> String {
        t!("action.obs.pause_record.help").to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsChapterMarker {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsChapterMarker {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if OBS_CLIENT.get().is_none() {
            create_client(c).await?;
        }

        let client = OBS_CLIENT.get().unwrap().lock().await;
        client.recording().create_chapter(None).await?;

        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> AT {
        AT::ObsChapterMarker
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, config);
    }

    fn help(&self) -> String {
        t!("action.obs.add_chapter_marker.help").to_string()
    }
}
