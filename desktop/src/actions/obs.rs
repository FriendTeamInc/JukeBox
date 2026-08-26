use std::{sync::OnceLock, time::Duration};

use eframe::egui::{ComboBox, ImageSource, RichText, TextEdit, Ui, include_image};
use egui_phosphor::regular as phos;
use obws::{
    Client,
    client::{ConnectConfig, DEFAULT_BROADCAST_CAPACITY},
    requests::{inputs::InputId, scene_items::SetEnabled, scenes::SceneId},
    responses::{inputs::Input, scene_items::SceneItem, scenes::Scene},
};
use serde::{Deserialize, Serialize};
use tokio::{
    runtime::Handle,
    sync::{Mutex, MutexGuard},
};
use uuid::Uuid;

use crate::{
    actions::types::{ActionModuleConfig, ActionOk, ActionResult, ActionTrait},
    input::InputKey,
    single_fire,
};

use super::types::{Action, ActionError};

pub const AMID_OBS: &str = "JB.OBS";
pub const AID_OBS_STREAM: &str = "JB.OBS.Stream";
pub const AID_OBS_RECORD: &str = "JB.OBS.Record";
pub const AID_OBS_RECORD_PAUSE: &str = "JB.OBS.RecordPause";
pub const AID_OBS_REPLAY_BUFFER: &str = "JB.OBS.ReplayBuffer";
pub const AID_OBS_REPLAY_BUFFER_SAVE: &str = "JB.OBS.ReplayBufferSave";
pub const AID_OBS_TOGGLE_SOURCE: &str = "JB.OBS.ToggleSource";
pub const AID_OBS_TOGGLE_MUTE: &str = "JB.OBS.ToggleMute";
pub const AID_OBS_SCENE_SWITCH: &str = "JB.OBS.SceneSwitch";
pub const AID_OBS_PREVIEW_SWITCH: &str = "JB.OBS.PreviewSwitch";
pub const AID_OBS_PREVIEW_PUSH: &str = "JB.OBS.PreviewPush";
pub const AID_OBS_COLLECTION_SWITCH: &str = "JB.OBS.CollectionSwitch";
pub const AID_OBS_CHAPTER_MARKER: &str = "JB.OBS.ChapterMarker";

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
static OBS_CLIENT: OnceLock<Mutex<Option<Client>>> = OnceLock::new();

static OBS_SCENES: OnceLock<Mutex<Option<Vec<Scene>>>> = OnceLock::new();
static OBS_SOURCES: OnceLock<Mutex<Option<Vec<SceneItem>>>> = OnceLock::new();
static OBS_INPUTS: OnceLock<Mutex<Option<Vec<Input>>>> = OnceLock::new();
static OBS_SCENE_COLLECTIONS: OnceLock<Mutex<Option<Vec<String>>>> = OnceLock::new();

pub fn init_actions_obs(config: ActionModuleConfig) -> (String, Vec<Action>) {
    let c = config.blocking_lock().clone();
    let (host, port, password) = (
        c.get("host").cloned().unwrap_or("localhost".into()),
        c.get("port").cloned().unwrap_or("4455".into()),
        c.get("password").cloned().unwrap_or("".into()),
    );

    OBS_HOST_ADDRESS.get_or_init(|| Mutex::new(host));
    OBS_HOST_PORT.get_or_init(|| Mutex::new(port));
    OBS_PASSWORD.get_or_init(|| Mutex::new(password));

    OBS_SCENES.get_or_init(|| Mutex::new(None));
    OBS_SOURCES.get_or_init(|| Mutex::new(None));
    OBS_INPUTS.get_or_init(|| Mutex::new(None));
    OBS_SCENE_COLLECTIONS.get_or_init(|| Mutex::new(None));

    // init obs websocket (if we have a saved config for it)
    tokio::runtime::Handle::current().spawn(async move { create_client(config).await });

    (
        t!("action.obs.title", icon = phos::VINYL_RECORD).into(),
        vec![
            Box::new(ObsStream::default()),
            Box::new(ObsRecord::default()),
            Box::new(ObsPauseRecord::default()),
            Box::new(ObsReplayBuffer::default()),
            Box::new(ObsSaveReplay::default()),
            Box::new(ObsSource::default()),
            Box::new(ObsMute::default()),
            Box::new(ObsSceneSwitch::default()),
            Box::new(ObsPreviewSceneSwitch::default()),
            Box::new(ObsPreviewScenePush::default()),
            Box::new(ObsSceneCollectionSwitch::default()),
            // Box::new(ObsFilter::default()),
            // Box::new(ObsTransition::default()),
            // // TODO: Source Screenshot?
            Box::new(ObsChapterMarker::default()),
        ],
    )
}

async fn create_client<'a>(
    config: ActionModuleConfig,
) -> Result<MutexGuard<'a, Option<Client>>, ()> {
    let client_config = {
        let c = config.lock().await.clone();

        let (host, port, password) =
            if c.contains_key("host") && c.contains_key("port") && c.contains_key("password") {
                let host = c.get("host").unwrap().clone();
                let port = c
                    .get("port")
                    .unwrap()
                    .clone()
                    .parse()
                    .expect("cannot parse port");
                let password = c.get("password").cloned();

                (host, port, password)
            } else {
                let pw = OBS_PASSWORD.get().unwrap().lock().await.clone();
                let password = if pw.len() == 0 { None } else { Some(pw) };
                let host = OBS_HOST_ADDRESS.get().unwrap().lock().await.clone();
                let port = OBS_HOST_PORT
                    .get()
                    .unwrap()
                    .lock()
                    .await
                    .clone()
                    .parse()
                    .expect("cannot parse port");

                (host, port, password)
            };

        ConnectConfig {
            host: host,
            port: port,
            dangerous: None,
            password: password,
            event_subscriptions: None, // TODO: subscribe for kicked/disconnected events?
            // tls: false,
            broadcast_capacity: DEFAULT_BROADCAST_CAPACITY,
            connect_timeout: Duration::from_millis(250),
            // NOTE: we're using a pretty low connection timeout time here because of UI reasons.
            // In the future, we should increase this for high latency environments.
            // (At the cost of frames)
        }
    };

    let client = Client::connect_with_config(client_config)
        .await
        .map_err(|_| ())?;

    if OBS_CLIENT.get().is_none() {
        let _ = OBS_CLIENT.set(Mutex::new(Some(client)));
    } else {
        let mut c = OBS_CLIENT.get().unwrap().lock().await;
        *c = Some(client);
    }

    Ok(OBS_CLIENT.get().unwrap().lock().await)
}

fn account_warning(ui: &mut Ui, config: ActionModuleConfig) -> Option<()> {
    if OBS_CLIENT.get().is_none() || OBS_CLIENT.get().unwrap().blocking_lock().is_none() {
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(t!("action.obs.setup.help_connect")).size(11.0));
            ui.label(RichText::new(t!("action.obs.setup.help_instructions")).size(9.0));
        });
        ui.label("");
        if ui.button(t!("action.obs.setup.button")).clicked() {
            let res = Handle::current().block_on(async { create_client(config.clone()).await });
            match res {
                Ok(_) => {
                    let mut c = config.blocking_lock();

                    c.insert(
                        "host".into(),
                        OBS_HOST_ADDRESS.get().unwrap().blocking_lock().clone(),
                    );
                    c.insert(
                        "port".into(),
                        OBS_HOST_PORT.get().unwrap().blocking_lock().clone(),
                    );
                    c.insert(
                        "password".into(),
                        OBS_PASSWORD.get().unwrap().blocking_lock().clone(),
                    );

                    log::error!("connected to obs")
                }
                Err(e) => log::error!("failed to connect to obs: {:?}", e),
            }

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
                    .hint_text("password")
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

async fn check_client<'a>(
    device_uid: &String,
    input_key: &InputKey,
    config: ActionModuleConfig,
) -> Result<MutexGuard<'a, Option<Client>>, ActionError> {
    if OBS_CLIENT.get().is_none() || OBS_CLIENT.get().unwrap().lock().await.is_none() {
        create_client(config)
            .await
            .map_err(|_| ActionError::new(device_uid, *input_key, t!("action.obs.err.client")))
    } else {
        Ok(OBS_CLIENT.get().unwrap().lock().await)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsStream {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsStream {
    fn get_type(&self) -> &'static str {
        AID_OBS_STREAM
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.toggle_stream.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.toggle_stream.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let res = client
            .as_ref()
            .unwrap()
            .streaming()
            .toggle()
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(device_uid, *input_key, t!("action.obs.toggle_stream.err"))
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config);
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_STREAM]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsRecord {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsRecord {
    fn get_type(&self) -> &'static str {
        AID_OBS_RECORD
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.toggle_record.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.toggle_record.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let res = client
            .as_ref()
            .unwrap()
            .recording()
            .toggle()
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(device_uid, *input_key, t!("action.obs.toggle_record.err"))
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config);
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_RECORD]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsPauseRecord {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsPauseRecord {
    fn get_type(&self) -> &'static str {
        AID_OBS_RECORD_PAUSE
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.pause_record.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.pause_record.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let res = client
            .as_ref()
            .unwrap()
            .recording()
            .toggle_pause()
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(device_uid, *input_key, t!("action.obs.pause_record.err"))
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config);
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_PAUSE_RECORD]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsReplayBuffer {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsReplayBuffer {
    fn get_type(&self) -> &'static str {
        AID_OBS_REPLAY_BUFFER
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.toggle_replay_buffer.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.toggle_replay_buffer.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let res = client
            .as_ref()
            .unwrap()
            .replay_buffer()
            .toggle()
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    *input_key,
                    t!("action.obs.toggle_replay_buffer.err"),
                )
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config);
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_REPLAY_BUFFER]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsSaveReplay {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsSaveReplay {
    fn get_type(&self) -> &'static str {
        AID_OBS_REPLAY_BUFFER_SAVE
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.save_replay_buffer.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.save_replay_buffer.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let res = client
            .as_ref()
            .unwrap()
            .replay_buffer()
            .save()
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    *input_key,
                    t!("action.obs.save_replay_buffer.err"),
                )
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config);
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_SAVE_REPLAY]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsSource {
    scene: Option<(Uuid, String)>,
    source: Option<(i64, String)>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsSource {
    fn get_type(&self) -> &'static str {
        AID_OBS_TOGGLE_SOURCE
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.toggle_source.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.toggle_source.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let Some(scene) = &self.scene else {
            return Err(ActionError::new(
                device_uid,
                *input_key,
                "action.obs.toggle_source.err.scene_not_configured",
            ));
        };

        let Some(source) = &self.source else {
            return Err(ActionError::new(
                device_uid,
                *input_key,
                "action.obs.toggle_source.err.source_not_configured",
            ));
        };

        let scene_id = SceneId::Uuid(scene.0);

        let res = client
            .as_ref()
            .unwrap()
            .scene_items()
            .enabled(scene_id, source.0)
            .await
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    *input_key,
                    t!(
                        "action.obs.toggle_source.err.get_enabled",
                        scene = scene.1,
                        source = source.1
                    ),
                )
            });

        let enabled = match res {
            Ok(e) => e,
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                return Err(e);
            }
        };

        let res = client
            .as_ref()
            .unwrap()
            .scene_items()
            .set_enabled(SetEnabled {
                scene: scene_id,
                item_id: source.0,
                enabled: !enabled,
            })
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    *input_key,
                    t!(
                        "action.obs.toggle_source.err.set_enabled",
                        scene = scene.1,
                        source = source.1
                    ),
                )
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        if account_warning(ui, module_config).is_none() {
            return;
        }

        ui.label("");

        ui.label(t!("action.obs.options.select_scene"));
        let ir = ComboBox::from_id_salt("ObsSceneSelect")
            .width(200.0)
            .selected_text(self.scene.clone().map(|s| s.1).unwrap_or("".into()))
            .show_ui(ui, |ui| {
                let scenes = OBS_SCENES.get().unwrap().blocking_lock();
                if let Some(scenes) = &*scenes {
                    for scene in scenes {
                        let selected = if let Some(selected_scene) = &self.scene {
                            selected_scene.0 == scene.id.uuid
                        } else {
                            false
                        };
                        let l = ui.selectable_label(selected, scene.id.name.clone());
                        if l.clicked() {
                            self.scene = Some((scene.id.uuid, scene.id.name.clone()));
                            self.source = None;
                        }
                    }
                } else {
                    ui.label(t!("action.obs.options.loading"));
                }
            });

        single_fire!(ComboBox::is_open(ui.ctx(), ir.response.id), {
            *OBS_SCENES.get().unwrap().blocking_lock() = None;
            tokio::spawn(async {
                let client = OBS_CLIENT.get().unwrap().lock().await;
                if let Ok(scene_list) = client.as_ref().unwrap().scenes().list().await {
                    *OBS_SCENES.get().unwrap().lock().await = Some(scene_list.scenes);
                }
            });
        });

        ui.label(t!("action.obs.options.select_source"));
        let ir = ui
            .add_enabled_ui(self.scene.is_some(), |ui| {
                ComboBox::from_id_salt("ObsSourceSelect")
                    .width(200.0)
                    .selected_text(self.source.clone().map(|s| s.1).unwrap_or("".into()))
                    .show_ui(ui, |ui| {
                        let sources = OBS_SOURCES.get().unwrap().blocking_lock();
                        if let Some(sources) = &*sources {
                            for source in sources {
                                let selected = if let Some(selected_source) = &self.source {
                                    selected_source.0 == source.id
                                } else {
                                    false
                                };
                                let l = ui.selectable_label(selected, source.source_name.clone());
                                if l.clicked() {
                                    self.source = Some((source.id, source.source_name.clone()));
                                }
                            }
                        } else {
                            ui.label(t!("action.obs.options.loading"));
                        }
                    })
            })
            .inner;

        single_fire!(ComboBox::is_open(ui.ctx(), ir.response.id), {
            *OBS_SOURCES.get().unwrap().blocking_lock() = None;
            let scene_id = SceneId::Uuid(self.scene.clone().unwrap().0);
            tokio::spawn(async move {
                let client = OBS_CLIENT.get().unwrap().lock().await;
                if let Ok(items) = client.as_ref().unwrap().scene_items().list(scene_id).await {
                    *OBS_SOURCES.get().unwrap().lock().await = Some(items);
                }
            });
        });
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_SOURCE]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsMute {
    input: Option<(Uuid, String)>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsMute {
    fn get_type(&self) -> &'static str {
        AID_OBS_TOGGLE_MUTE
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.toggle_mute.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.toggle_mute.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let Some(input) = &self.input else {
            return Err(ActionError::new(
                device_uid,
                *input_key,
                t!("action.obs.toggle_mute.err.input_not_configured",),
            ));
        };

        let res = client
            .as_ref()
            .unwrap()
            .inputs()
            .toggle_mute(InputId::Uuid(input.0))
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    *input_key,
                    t!("action.obs.toggle_mute.err.failure", input = input.1,),
                )
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        if account_warning(ui, module_config).is_none() {
            return;
        }

        ui.label("");

        ui.label(t!("action.obs.options.select_scene"));
        let ir = ComboBox::from_id_salt("ObsInputSelect")
            .width(200.0)
            .selected_text(self.input.clone().map(|s| s.1).unwrap_or("".into()))
            .show_ui(ui, |ui| {
                let inputs = OBS_INPUTS.get().unwrap().blocking_lock();
                if let Some(inputs) = &*inputs {
                    for input in inputs {
                        let selected = if let Some(selected_input) = &self.input {
                            selected_input.0 == input.id.uuid
                        } else {
                            false
                        };
                        let l = ui.selectable_label(selected, input.id.name.clone());
                        if l.clicked() {
                            self.input = Some((input.id.uuid, input.id.name.clone()));
                        }
                    }
                } else {
                    ui.label(t!("action.obs.options.loading"));
                }
            });

        single_fire!(ComboBox::is_open(ui.ctx(), ir.response.id), {
            *OBS_INPUTS.get().unwrap().blocking_lock() = None;
            tokio::spawn(async {
                let client = OBS_CLIENT.get().unwrap().lock().await;
                if let Ok(input_list) = client.as_ref().unwrap().inputs().list(None).await {
                    // TODO: filter out non-audio sources
                    *OBS_INPUTS.get().unwrap().lock().await = Some(input_list);
                }
            });
        });
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_MUTE]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsSceneSwitch {
    scene: Option<(Uuid, String)>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsSceneSwitch {
    fn get_type(&self) -> &'static str {
        AID_OBS_SCENE_SWITCH
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.switch_scene.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.switch_scene.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let Some(scene) = &self.scene else {
            return Err(ActionError::new(
                device_uid,
                *input_key,
                t!("action.obs.switch_scene.err.scene_not_configured"),
            ));
        };

        let res = client
            .as_ref()
            .unwrap()
            .scenes()
            .set_current_program_scene(SceneId::Uuid(scene.0))
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    *input_key,
                    t!("action.obs.switch_scene.err.failure", scene = scene.1,),
                )
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        if account_warning(ui, module_config).is_none() {
            return;
        }

        ui.label("");

        ui.label(t!("action.obs.options.select_scene"));
        let ir = ComboBox::from_id_salt("ObsSceneSelect")
            .width(200.0)
            .selected_text(self.scene.clone().map(|s| s.1).unwrap_or("".into()))
            .show_ui(ui, |ui| {
                let scenes = OBS_SCENES.get().unwrap().blocking_lock();
                if let Some(scenes) = &*scenes {
                    for scene in scenes {
                        let selected = if let Some(selected_scene) = &self.scene {
                            selected_scene.0 == scene.id.uuid
                        } else {
                            false
                        };
                        let l = ui.selectable_label(selected, scene.id.name.clone());
                        if l.clicked() {
                            self.scene = Some((scene.id.uuid, scene.id.name.clone()));
                        }
                    }
                } else {
                    ui.label(t!("action.obs.options.loading"));
                }
            });

        single_fire!(ComboBox::is_open(ui.ctx(), ir.response.id), {
            *OBS_SCENES.get().unwrap().blocking_lock() = None;
            tokio::spawn(async {
                let client = OBS_CLIENT.get().unwrap().lock().await;
                if let Ok(scene_list) = client.as_ref().unwrap().scenes().list().await {
                    *OBS_SCENES.get().unwrap().lock().await = Some(scene_list.scenes);
                }
            });
        });
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_SWITCH_SCENE]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsPreviewSceneSwitch {
    scene: Option<(Uuid, String)>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsPreviewSceneSwitch {
    fn get_type(&self) -> &'static str {
        AID_OBS_PREVIEW_SWITCH
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.switch_preview_scene.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.switch_preview_scene.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let Some(scene) = &self.scene else {
            return Err(ActionError::new(
                device_uid,
                *input_key,
                t!("action.obs.switch_preview_scene.err.scene_not_configured"),
            ));
        };

        let res = client
            .as_ref()
            .unwrap()
            .scenes()
            .set_current_preview_scene(SceneId::Uuid(scene.0))
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    *input_key,
                    t!(
                        "action.obs.switch_preview_scene.err.failure",
                        scene = scene.1,
                    ),
                )
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        if account_warning(ui, module_config).is_none() {
            return;
        }

        ui.label("");

        ui.label(t!("action.obs.options.select_scene"));
        let ir = ComboBox::from_id_salt("ObsSceneSelect")
            .width(200.0)
            .selected_text(self.scene.clone().map(|s| s.1).unwrap_or("".into()))
            .show_ui(ui, |ui| {
                let scenes = OBS_SCENES.get().unwrap().blocking_lock();
                if let Some(scenes) = &*scenes {
                    for scene in scenes {
                        let selected = if let Some(selected_scene) = &self.scene {
                            selected_scene.0 == scene.id.uuid
                        } else {
                            false
                        };
                        let l = ui.selectable_label(selected, scene.id.name.clone());
                        if l.clicked() {
                            self.scene = Some((scene.id.uuid, scene.id.name.clone()));
                        }
                    }
                } else {
                    ui.label(t!("action.obs.options.loading"));
                }
            });

        single_fire!(ComboBox::is_open(ui.ctx(), ir.response.id), {
            *OBS_SCENES.get().unwrap().blocking_lock() = None;
            tokio::spawn(async {
                let client = OBS_CLIENT.get().unwrap().lock().await;
                if let Ok(scene_list) = client.as_ref().unwrap().scenes().list().await {
                    *OBS_SCENES.get().unwrap().lock().await = Some(scene_list.scenes);
                }
            });
        });
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_SWITCH_PREVIEW]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsPreviewScenePush {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsPreviewScenePush {
    fn get_type(&self) -> &'static str {
        AID_OBS_PREVIEW_PUSH
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.push_preview_scene.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.push_preview_scene.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let res = client
            .as_ref()
            .unwrap()
            .transitions()
            .trigger()
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    *input_key,
                    t!("action.obs.push_preview_scene.err"),
                )
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config);
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_PUSH_PREVIEW]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsSceneCollectionSwitch {
    scene_collection: Option<String>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsSceneCollectionSwitch {
    fn get_type(&self) -> &'static str {
        AID_OBS_COLLECTION_SWITCH
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.switch_scene_collection.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.switch_scene_collection.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let Some(scene_collection) = &self.scene_collection else {
            return Err(ActionError::new(
                device_uid,
                *input_key,
                t!("action.obs.switch_scene_collection.err.collection_not_configured"),
            ));
        };

        let res = client
            .as_ref()
            .unwrap()
            .scene_collections()
            .set_current(scene_collection)
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    *input_key,
                    t!(
                        "action.obs.switch_scene_collection.err.failure",
                        collection = scene_collection,
                    ),
                )
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        if account_warning(ui, module_config).is_none() {
            return;
        }

        ui.label("");

        ui.label(t!("action.obs.options.select_scene"));
        let ir = ComboBox::from_id_salt("ObsSceneSelect")
            .width(200.0)
            .selected_text(self.scene_collection.clone().unwrap_or("".into()))
            .show_ui(ui, |ui| {
                let collections = OBS_SCENE_COLLECTIONS.get().unwrap().blocking_lock();
                if let Some(collections) = &*collections {
                    for collection in collections {
                        let selected = if let Some(selected_collection) = &self.scene_collection {
                            *selected_collection == *collection
                        } else {
                            false
                        };
                        let l = ui.selectable_label(selected, collection.clone());
                        if l.clicked() {
                            self.scene_collection = Some(collection.clone());
                        }
                    }
                } else {
                    ui.label(t!("action.obs.options.loading"));
                }
            });

        single_fire!(ComboBox::is_open(ui.ctx(), ir.response.id), {
            *OBS_SCENE_COLLECTIONS.get().unwrap().blocking_lock() = None;
            tokio::spawn(async {
                let client = OBS_CLIENT.get().unwrap().lock().await;
                if let Ok(collection_list) =
                    client.as_ref().unwrap().scene_collections().list().await
                {
                    *OBS_SCENE_COLLECTIONS.get().unwrap().lock().await =
                        Some(collection_list.collections);
                }
            });
        });
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_SWITCH_COLLECTION]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ObsChapterMarker {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for ObsChapterMarker {
    fn get_type(&self) -> &'static str {
        AID_OBS_CHAPTER_MARKER
    }
    fn get_module(&self) -> &'static str {
        AMID_OBS
    }
    fn get_title(&self) -> &'static str {
        "action.obs.add_chapter_marker.title"
    }
    fn get_description(&self) -> &'static str {
        "action.obs.add_chapter_marker.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        let mut client = check_client(device_uid, input_key, module_config).await?;

        let res = client
            .as_ref()
            .unwrap()
            .recording()
            .create_chapter(None)
            .await
            .map(|_| ActionOk::new())
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    *input_key,
                    t!("action.obs.add_chapter_marker.err.failure"),
                )
            });

        match res {
            Ok(o) => Ok(o),
            Err(e) => {
                client.as_mut().unwrap().disconnect().await;
                *client = None;
                Err(e)
            }
        }
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config);
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_CHAPTER_MARKER]
    }
}
