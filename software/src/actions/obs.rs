use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use anyhow::Result;
use eframe::egui::{RichText, TextEdit, Ui};
use egui_phosphor::regular as phos;
use obws::{
    client::{ConnectConfig, DEFAULT_BROADCAST_CAPACITY},
    Client,
};
use serde::{Deserialize, Serialize};
use tokio::{runtime::Handle, sync::Mutex};

use crate::{
    config::{JukeBoxConfig, ObsAccess},
    input::InputKey,
};

use super::{
    meta::MetaNoAction,
    types::{Action, ActionType as AT},
};

static OBS_HOST_ADDRESS: OnceLock<Mutex<String>> = OnceLock::new();
static OBS_HOST_PORT: OnceLock<Mutex<String>> = OnceLock::new();
static OBS_PASSWORD: OnceLock<Mutex<String>> = OnceLock::new();
static OBS_CLIENT: OnceLock<Mutex<Client>> = OnceLock::new();

#[rustfmt::skip]
pub fn obs_action_list() -> (String, Vec<(AT, Box<dyn Action>, String)>) {
    (
        t!("action.obs.title", icon = phos::VINYL_RECORD).to_string(),
        vec![
            (AT::ObsStream,                Box::new(ObsStream::default()),        t!("action.obs.toggle_stream.title").to_string()),
            (AT::ObsRecord,                Box::new(ObsRecord::default()),        t!("action.obs.toggle_record.title").to_string()),
            (AT::ObsPauseRecord,           Box::new(ObsPauseRecord::default()),   t!("action.obs.pause_record.title").to_string()),
            (AT::ObsReplayBuffer,          Box::new(ObsReplayBuffer::default()),  t!("action.obs.toggle_replay_buffer.title").to_string()),
            (AT::ObsSaveReplay,            Box::new(ObsSaveReplay::default()),    t!("action.obs.save_replay_buffer.title").to_string()),
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
    let client_config = {
        let c = config.lock().await.clone();

        let config = if let Some(o) = c.obs_access {
            o
        } else {
            let pw = OBS_PASSWORD.get().unwrap().lock().await.clone();
            let password = if pw.len() == 0 { None } else { Some(pw) };
            ObsAccess {
                host: OBS_HOST_ADDRESS.get().unwrap().lock().await.clone(),
                port: OBS_HOST_PORT.get().unwrap().lock().await.clone().parse()?,
                password,
            }
        };

        ConnectConfig {
            host: config.host,
            port: config.port,
            dangerous: None,
            password: config.password,
            event_subscriptions: None, // TODO: subscribe for kicked events?
            // tls: false,
            broadcast_capacity: DEFAULT_BROADCAST_CAPACITY,
            connect_timeout: Duration::from_millis(250),
            // NOTE: we're using a pretty low connection timeout time here because of UI reasons.
            // In the future, we should increase this for high latency environments.
            // (At the cost of frames)
        }
    };

    let obs_access = ObsAccess {
        host: client_config.host.clone(),
        port: client_config.port,
        password: client_config.password.clone(),
    };

    let client = Client::connect_with_config(client_config).await?;

    {
        let mut config = config.lock().await;
        config.obs_access = Some(obs_access);
        config.save();
    }

    let _ = OBS_CLIENT.set(Mutex::new(client));

    Ok(())
}

fn account_warning(ui: &mut Ui, config: Arc<Mutex<JukeBoxConfig>>) -> Option<()> {
    if OBS_HOST_ADDRESS.get().is_none()
        && OBS_HOST_PORT.get().is_none()
        && OBS_PASSWORD.get().is_none()
    {
        let c = config.blocking_lock().clone();
        if let Some(c) = c.obs_access {
            OBS_HOST_ADDRESS.get_or_init(|| Mutex::new(c.host));
            OBS_HOST_PORT.get_or_init(|| Mutex::new(c.port.to_string()));
            OBS_PASSWORD.get_or_init(|| Mutex::new(c.password.unwrap_or("".to_string())));
        } else {
            OBS_HOST_ADDRESS.get_or_init(|| Mutex::new("localhost".to_string()));
            OBS_HOST_PORT.get_or_init(|| Mutex::new("4455".to_string()));
            OBS_PASSWORD.get_or_init(|| Mutex::new("".to_string()));
        }
    }

    let o = config.blocking_lock().obs_access.clone();
    if OBS_CLIENT.get().is_none() && o.is_some() {
        let c = config.clone();
        let res = Handle::current().block_on(async { create_client(c).await });
        if let Err(_) = res {
            let config = config.clone();
            let mut c = config.blocking_lock();
            c.obs_access = None;
            c.save();
        }
    }

    if OBS_CLIENT.get().is_none() {
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t!("action.obs.setup.help_connect")).size(11.0));
            ui.label(RichText::new(t!("action.obs.setup.help_instructions")).size(9.0));
        });
        ui.label("");
        if ui.button(t!("action.obs.setup.button")).clicked() {
            let res = Handle::current().block_on(async { create_client(config).await });
            log::info!("connect to obs: {:?}", res);
            // TODO: error handle
        }
        ui.label("");
        {
            ui.label(t!("action.obs.setup.host_address"));
            let mut obs_host_address = OBS_HOST_ADDRESS.get().unwrap().blocking_lock();
            ui.add(TextEdit::singleline(&mut *obs_host_address).hint_text("localhost"));
        }
        {
            ui.label(t!("action.obs.setup.host_port"));
            let mut obs_host_port = OBS_HOST_PORT.get().unwrap().blocking_lock();
            let old_port = obs_host_port.clone();
            ui.add(TextEdit::singleline(&mut *obs_host_port).hint_text("4455"));
            if let Err(_) = obs_host_port.parse::<u16>() {
                *obs_host_port = old_port;
            }
        }
        {
            ui.label(t!("action.obs.setup.password"));
            let mut obs_password = OBS_PASSWORD.get().unwrap().blocking_lock();
            ui.add(
                TextEdit::singleline(&mut *obs_password)
                    .hint_text("Password")
                    .password(true),
            );
        }

        None
    } else {
        ui.vertical_centered(|ui| {
            ui.label(t!("action.obs.setup.success"));
        });

        Some(())
    }
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
pub struct ObsReplayBuffer {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsReplayBuffer {
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
        client.replay_buffer().toggle().await?;

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
        AT::ObsReplayBuffer
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
        t!("action.obs.toggle_replay_buffer.help").to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsSaveReplay {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsSaveReplay {
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
        client.replay_buffer().save().await?;

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
        AT::ObsSaveReplay
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
        t!("action.obs.save_replay_buffer.help").to_string()
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
