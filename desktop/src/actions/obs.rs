use std::{
    
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
    time::Duration,
};

use anyhow::{bail, Result};
use eframe::egui::{include_image, ComboBox, ImageSource, RichText, TextEdit, Ui};
use egui_phosphor::regular as phos;
use obws::{
    client::{ConnectConfig, DEFAULT_BROADCAST_CAPACITY},
    requests::{inputs::InputId, scene_items::SetEnabled, scenes::SceneId},
    responses::{inputs::Input, scene_items::SceneItem, scenes::Scene},
    Client,
};
use serde::{Deserialize, Serialize};
use tokio::{runtime::Handle, sync::Mutex};
use uuid::Uuid;

use crate::{
    config::{JukeBoxConfig, ObsAccess},
    input::InputKey,
};

use super::types::Action;

// TODO: finish icons
const ICON_STREAM: ImageSource = include_image!("../../../assets/action-icons/obs-stream.bmp");
const ICON_RECORD: ImageSource = include_image!("../../../assets/action-icons/obs-record.bmp");
const ICON_PAUSE_RECORD: ImageSource =
    include_image!("../../../assets/action-icons/obs-recordpause.bmp");
const ICON_REPLAY_BUFFER: ImageSource =
    include_image!("../../../assets/action-icons/obs-replaybufferpause.bmp");
const ICON_SAVE_REPLAY: ImageSource =
    include_image!("../../../assets/action-icons/obs-replaybuffer.bmp");
const ICON_SOURCE: ImageSource = include_image!("../../../assets/action-icons/obs-source.bmp");
const ICON_MUTE: ImageSource = include_image!("../../../assets/action-icons/obs-mute.bmp");
const ICON_SWITCH_SCENE: ImageSource =
    include_image!("../../../assets/action-icons/obs-sceneswitch.bmp");
const ICON_SWITCH_PREVIEW: ImageSource =
    include_image!("../../../assets/action-icons/obs-previewswitch.bmp");
const ICON_PUSH_PREVIEW: ImageSource =
    include_image!("../../../assets/action-icons/obs-previewpush.bmp");
const ICON_SWITCH_COLLECTION: ImageSource =
    include_image!("../../../assets/action-icons/obs-collectionswitch.bmp");
const ICON_CHAPTER_MARKER: ImageSource =
    include_image!("../../../assets/action-icons/obs-chaptermarker.bmp");

static OBS_HOST_ADDRESS: OnceLock<Mutex<String>> = OnceLock::new();
static OBS_HOST_PORT: OnceLock<Mutex<String>> = OnceLock::new();
static OBS_PASSWORD: OnceLock<Mutex<String>> = OnceLock::new();
static OBS_CLIENT: OnceLock<Mutex<Client>> = OnceLock::new();

static OBS_GET_SCENES: OnceLock<AtomicBool> = OnceLock::new();
static OBS_SCENES: OnceLock<Mutex<Option<Vec<Scene>>>> = OnceLock::new();
static OBS_GET_SOURCES: OnceLock<AtomicBool> = OnceLock::new();
static OBS_SOURCES: OnceLock<Mutex<Option<Vec<SceneItem>>>> = OnceLock::new();
static OBS_GET_INPUTS: OnceLock<AtomicBool> = OnceLock::new();
static OBS_INPUTS: OnceLock<Mutex<Option<Vec<Input>>>> = OnceLock::new();
static OBS_GET_SCENE_COLLECTIONS: OnceLock<AtomicBool> = OnceLock::new();
static OBS_SCENE_COLLECTIONS: OnceLock<Mutex<Option<Vec<String>>>> = OnceLock::new();

#[rustfmt::skip]
pub fn obs_action_list() -> (String, Vec<(String, Box<dyn Action>, String)>) {
    OBS_GET_SCENES.get_or_init(|| true.into());
    OBS_SCENES.get_or_init(|| Mutex::new(None));
    OBS_GET_SOURCES.get_or_init(|| true.into());
    OBS_SOURCES.get_or_init(|| Mutex::new(None));
    OBS_GET_INPUTS.get_or_init(|| true.into());
    OBS_INPUTS.get_or_init(|| Mutex::new(None));
    OBS_GET_SCENE_COLLECTIONS.get_or_init(|| true.into());
    OBS_SCENE_COLLECTIONS.get_or_init(|| Mutex::new(None));

    (
        t!("action.obs.title", icon = phos::VINYL_RECORD).into(),
        vec![
            ("ObsStream".into(),                Box::new(ObsStream::default()),                t!("action.obs.toggle_stream.title").into()),
            ("ObsRecord".into(),                Box::new(ObsRecord::default()),                t!("action.obs.toggle_record.title").into()),
            ("ObsPauseRecord".into(),           Box::new(ObsPauseRecord::default()),           t!("action.obs.pause_record.title").into()),
            ("ObsReplayBuffer".into(),          Box::new(ObsReplayBuffer::default()),          t!("action.obs.toggle_replay_buffer.title").into()),
            ("ObsSaveReplay".into(),            Box::new(ObsSaveReplay::default()),            t!("action.obs.save_replay_buffer.title").into()),
            ("ObsSource".into(),                Box::new(ObsSource::default()),                t!("action.obs.toggle_source.title").into()),
            ("ObsMute".into(),                  Box::new(ObsMute::default()),                  t!("action.obs.toggle_mute.title").into()),
            ("ObsSceneSwitch".into(),           Box::new(ObsSceneSwitch::default()),           t!("action.obs.switch_scene.title").into()),
            ("ObsPreviewSceneSwitch".into(),    Box::new(ObsPreviewSceneSwitch::default()),    t!("action.obs.switch_preview_scene.title").into()),
            ("ObsPreviewScenePush".into(),      Box::new(ObsPreviewScenePush::default()),      t!("action.obs.push_preview_scene.title").into()),
            ("ObsSceneCollectionSwitch".into(), Box::new(ObsSceneCollectionSwitch::default()), t!("action.obs.switch_scene_collection.title").into()),
            // ("ObsFilter".into(),                Box::new(ObsFilter::default()),                t!("action.obs.toggle_filter.title").into()),
            // ("ObsTransition".into(),            Box::new(ObsTransition::default()),            t!("action.obs.switch_transition.title").into()),
            ("ObsChapterMarker".into(),         Box::new(ObsChapterMarker::default()),         t!("action.obs.add_chapter_marker.title").into()),
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
            OBS_PASSWORD.get_or_init(|| Mutex::new(c.password.unwrap_or("".into())));
        } else {
            OBS_HOST_ADDRESS.get_or_init(|| Mutex::new("localhost".into()));
            OBS_HOST_PORT.get_or_init(|| Mutex::new("4455".into()));
            OBS_PASSWORD.get_or_init(|| Mutex::new("".into()));
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

    fn get_type(&self) -> String {
        "ObsStream".into()
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
        t!("action.obs.toggle_stream.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_STREAM
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

    fn get_type(&self) -> String {
        "ObsRecord".into()
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
        t!("action.obs.toggle_record.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_RECORD
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

    fn get_type(&self) -> String {
        "ObsPauseRecord".into()
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
        t!("action.obs.pause_record.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_PAUSE_RECORD
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

    fn get_type(&self) -> String {
        "ObsReplayBuffer".into()
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
        t!("action.obs.toggle_replay_buffer.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_REPLAY_BUFFER
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

    fn get_type(&self) -> String {
        "ObsSaveReplay".into()
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
        t!("action.obs.save_replay_buffer.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_SAVE_REPLAY
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsSource {
    scene: Option<(Uuid, String)>,
    source: Option<(i64, String)>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsSource {
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
        if let Some(scene) = &self.scene {
            if let Some(source) = &self.source {
                let scene_id = SceneId::Uuid(scene.0);

                let enabled = client.scene_items().enabled(scene_id, source.0).await?;

                client
                    .scene_items()
                    .set_enabled(SetEnabled {
                        scene: scene_id,
                        item_id: source.0,
                        enabled: !enabled,
                    })
                    .await?;
            } else {
                bail!("source not configured!");
            }
        } else {
            bail!("scene not configured!");
        }

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

    fn get_type(&self) -> String {
        "ObsSource".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        if account_warning(ui, config).is_none() {
            return;
        }

        ui.label("");

        ui.label(t!("action.obs.options.select_scene"));
        let ir = ComboBox::from_id_salt("ObsSceneSelect")
            .width(200.0)
            .selected_text(
                self.scene
                    .clone()
                    .and_then(|s| Some(s.1))
                    .unwrap_or("".into()),
            )
            .show_ui(ui, |ui| {
                let scenes = OBS_SCENES.get().unwrap().blocking_lock().clone();
                if let Some(scenes) = scenes {
                    for scene in scenes {
                        let selected = if let Some(selected_scene) = &self.scene {
                            selected_scene.0 == scene.id.uuid
                        } else {
                            false
                        };
                        let l = ui.selectable_label(selected, scene.id.name.clone());
                        if l.clicked() {
                            self.scene = Some((scene.id.uuid, scene.id.name));
                            self.source = None;
                        }
                    }
                } else {
                    ui.label(t!("action.obs.options.loading"));
                }
            });
        if ComboBox::is_open(ui.ctx(), ir.response.id) {
            if OBS_GET_SCENES.get().unwrap().load(Ordering::Relaxed) {
                *OBS_SCENES.get().unwrap().blocking_lock() = None;
                tokio::spawn(async {
                    let client = OBS_CLIENT.get().unwrap().lock().await;
                    if let Ok(scene_list) = client.scenes().list().await {
                        *OBS_SCENES.get().unwrap().lock().await = Some(scene_list.scenes);
                    }
                });
            }
            let _ = OBS_GET_SCENES.set(false.into());
        } else {
            let _ = OBS_GET_SCENES.set(true.into());
        }

        ui.label(t!("action.obs.options.select_source"));
        let ir = ui
            .add_enabled_ui(self.scene.is_some(), |ui| {
                ComboBox::from_id_salt("ObsSourceSelect")
                    .width(200.0)
                    .selected_text(
                        self.source
                            .clone()
                            .and_then(|s| Some(s.1))
                            .unwrap_or("".into()),
                    )
                    .show_ui(ui, |ui| {
                        let sources = OBS_SOURCES.get().unwrap().blocking_lock().clone();
                        if let Some(sources) = sources {
                            for source in sources {
                                let selected = if let Some(selected_source) = &self.source {
                                    selected_source.0 == source.id
                                } else {
                                    false
                                };
                                let l = ui.selectable_label(selected, source.source_name.clone());
                                if l.clicked() {
                                    self.source = Some((source.id, source.source_name));
                                }
                            }
                        } else {
                            ui.label(t!("action.obs.options.loading"));
                        }
                    })
            })
            .inner;
        if ComboBox::is_open(ui.ctx(), ir.response.id) {
            if OBS_GET_SOURCES.get().unwrap().load(Ordering::Relaxed) {
                *OBS_SOURCES.get().unwrap().blocking_lock() = None;
                let scene_id = SceneId::Uuid(self.scene.clone().unwrap().0);
                tokio::spawn(async move {
                    let client = OBS_CLIENT.get().unwrap().lock().await;
                    if let Ok(items) = client.scene_items().list(scene_id).await {
                        *OBS_SOURCES.get().unwrap().lock().await = Some(items);
                    }
                });
            }
            let _ = OBS_GET_SOURCES.set(false.into());
        } else {
            let _ = OBS_GET_SOURCES.set(true.into());
        }
    }

    fn help(&self) -> String {
        t!("action.obs.toggle_source.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_SOURCE
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsMute {
    input: Option<(Uuid, String)>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsMute {
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
        if let Some(input) = &self.input {
            client.inputs().toggle_mute(InputId::Uuid(input.0)).await?;
        } else {
            bail!("Input not configured!");
        }

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

    fn get_type(&self) -> String {
        "ObsMute".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        if account_warning(ui, config).is_none() {
            return;
        }

        ui.label("");

        ui.label(t!("action.obs.options.select_scene"));
        let ir = ComboBox::from_id_salt("ObsInputSelect")
            .width(200.0)
            .selected_text(
                self.input
                    .clone()
                    .and_then(|s| Some(s.1))
                    .unwrap_or("".into()),
            )
            .show_ui(ui, |ui| {
                let inputs = OBS_INPUTS.get().unwrap().blocking_lock().clone();
                if let Some(inputs) = inputs {
                    for input in inputs {
                        let selected = if let Some(selected_input) = &self.input {
                            selected_input.0 == input.id.uuid
                        } else {
                            false
                        };
                        let l = ui.selectable_label(selected, input.id.name.clone());
                        if l.clicked() {
                            self.input = Some((input.id.uuid, input.id.name));
                        }
                    }
                } else {
                    ui.label(t!("action.obs.options.loading"));
                }
            });
        if ComboBox::is_open(ui.ctx(), ir.response.id) {
            if OBS_GET_INPUTS.get().unwrap().load(Ordering::Relaxed) {
                *OBS_INPUTS.get().unwrap().blocking_lock() = None;
                tokio::spawn(async {
                    let client = OBS_CLIENT.get().unwrap().lock().await;
                    if let Ok(input_list) = client.inputs().list(None).await {
                        // TODO: filter out non-audio sources
                        *OBS_INPUTS.get().unwrap().lock().await = Some(input_list);
                    }
                });
            }
            let _ = OBS_GET_INPUTS.set(false.into());
        } else {
            let _ = OBS_GET_INPUTS.set(true.into());
        }
    }

    fn help(&self) -> String {
        t!("action.obs.toggle_mute.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_MUTE
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsSceneSwitch {
    scene: Option<(Uuid, String)>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsSceneSwitch {
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
        if let Some(scene) = &self.scene {
            client
                .scenes()
                .set_current_program_scene(SceneId::Uuid(scene.0))
                .await?;
        } else {
            bail!("Scene not configured!");
        }

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

    fn get_type(&self) -> String {
        "ObsSceneSwitch".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        if account_warning(ui, config).is_none() {
            return;
        }

        ui.label("");

        ui.label(t!("action.obs.options.select_scene"));
        let ir = ComboBox::from_id_salt("ObsSceneSelect")
            .width(200.0)
            .selected_text(
                self.scene
                    .clone()
                    .and_then(|s| Some(s.1))
                    .unwrap_or("".into()),
            )
            .show_ui(ui, |ui| {
                let scenes = OBS_SCENES.get().unwrap().blocking_lock().clone();
                if let Some(scenes) = scenes {
                    for scene in scenes {
                        let selected = if let Some(selected_scene) = &self.scene {
                            selected_scene.0 == scene.id.uuid
                        } else {
                            false
                        };
                        let l = ui.selectable_label(selected, scene.id.name.clone());
                        if l.clicked() {
                            self.scene = Some((scene.id.uuid, scene.id.name));
                        }
                    }
                } else {
                    ui.label(t!("action.obs.options.loading"));
                }
            });
        if ComboBox::is_open(ui.ctx(), ir.response.id) {
            if OBS_GET_SCENES.get().unwrap().load(Ordering::Relaxed) {
                *OBS_SCENES.get().unwrap().blocking_lock() = None;
                tokio::spawn(async {
                    let client = OBS_CLIENT.get().unwrap().lock().await;
                    if let Ok(scene_list) = client.scenes().list().await {
                        *OBS_SCENES.get().unwrap().lock().await = Some(scene_list.scenes);
                    }
                });
            }
            let _ = OBS_GET_SCENES.set(false.into());
        } else {
            let _ = OBS_GET_SCENES.set(true.into());
        }
    }

    fn help(&self) -> String {
        t!("action.obs.switch_scene.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_SWITCH_SCENE
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsPreviewSceneSwitch {
    scene: Option<(Uuid, String)>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsPreviewSceneSwitch {
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
        if let Some(scene) = &self.scene {
            client
                .scenes()
                .set_current_preview_scene(SceneId::Uuid(scene.0))
                .await?;
        } else {
            bail!("Scene not configured!");
        }

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

    fn get_type(&self) -> String {
        "ObsPreviewSceneSwitch".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        if account_warning(ui, config).is_none() {
            return;
        }

        ui.label("");

        ui.label(t!("action.obs.options.select_scene"));
        let ir = ComboBox::from_id_salt("ObsSceneSelect")
            .width(200.0)
            .selected_text(
                self.scene
                    .clone()
                    .and_then(|s| Some(s.1))
                    .unwrap_or("".into()),
            )
            .show_ui(ui, |ui| {
                let scenes = OBS_SCENES.get().unwrap().blocking_lock().clone();
                if let Some(scenes) = scenes {
                    for scene in scenes {
                        let selected = if let Some(selected_scene) = &self.scene {
                            selected_scene.0 == scene.id.uuid
                        } else {
                            false
                        };
                        let l = ui.selectable_label(selected, scene.id.name.clone());
                        if l.clicked() {
                            self.scene = Some((scene.id.uuid, scene.id.name));
                        }
                    }
                } else {
                    ui.label(t!("action.obs.options.loading"));
                }
            });
        if ComboBox::is_open(ui.ctx(), ir.response.id) {
            if OBS_GET_SCENES.get().unwrap().load(Ordering::Relaxed) {
                *OBS_SCENES.get().unwrap().blocking_lock() = None;
                tokio::spawn(async {
                    let client = OBS_CLIENT.get().unwrap().lock().await;
                    if let Ok(scene_list) = client.scenes().list().await {
                        *OBS_SCENES.get().unwrap().lock().await = Some(scene_list.scenes);
                    }
                });
            }
            let _ = OBS_GET_SCENES.set(false.into());
        } else {
            let _ = OBS_GET_SCENES.set(true.into());
        }
    }

    fn help(&self) -> String {
        t!("action.obs.switch_preview_scene.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_SWITCH_PREVIEW
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsPreviewScenePush {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsPreviewScenePush {
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
        client.transitions().trigger().await?;

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

    fn get_type(&self) -> String {
        "ObsPreviewScenePush".into()
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
        t!("action.obs.push_preview_scene.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_PUSH_PREVIEW
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ObsSceneCollectionSwitch {
    scene_collection: Option<String>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for ObsSceneCollectionSwitch {
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
        if let Some(scene_collection) = &self.scene_collection {
            client
                .scene_collections()
                .set_current(scene_collection)
                .await?;
        } else {
            bail!("Scene not configured!");
        }

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

    fn get_type(&self) -> String {
        "ObsSceneCollectionSwitch".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        if account_warning(ui, config).is_none() {
            return;
        }

        ui.label("");

        ui.label(t!("action.obs.options.select_scene"));
        let ir = ComboBox::from_id_salt("ObsSceneSelect")
            .width(200.0)
            .selected_text(self.scene_collection.clone().unwrap_or("".into()))
            .show_ui(ui, |ui| {
                let collections = OBS_SCENE_COLLECTIONS.get().unwrap().blocking_lock().clone();
                if let Some(collections) = collections {
                    for collection in collections {
                        let selected = if let Some(selected_collection) = &self.scene_collection {
                            *selected_collection == collection
                        } else {
                            false
                        };
                        let l = ui.selectable_label(selected, collection.clone());
                        if l.clicked() {
                            self.scene_collection = Some(collection);
                        }
                    }
                } else {
                    ui.label(t!("action.obs.options.loading"));
                }
            });
        if ComboBox::is_open(ui.ctx(), ir.response.id) {
            if OBS_GET_SCENE_COLLECTIONS
                .get()
                .unwrap()
                .load(Ordering::Relaxed)
            {
                *OBS_SCENE_COLLECTIONS.get().unwrap().blocking_lock() = None;
                tokio::spawn(async {
                    let client = OBS_CLIENT.get().unwrap().lock().await;
                    if let Ok(collection_list) = client.scene_collections().list().await {
                        *OBS_SCENE_COLLECTIONS.get().unwrap().lock().await =
                            Some(collection_list.collections);
                    }
                });
            }
            let _ = OBS_GET_SCENE_COLLECTIONS.set(false.into());
        } else {
            let _ = OBS_GET_SCENE_COLLECTIONS.set(true.into());
        }
    }

    fn help(&self) -> String {
        t!("action.obs.switch_scene_collection.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_SWITCH_COLLECTION
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

    fn get_type(&self) -> String {
        "ObsChapterMarker".into()
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
        t!("action.obs.add_chapter_marker.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_CHAPTER_MARKER
    }
}
