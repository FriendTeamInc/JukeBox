use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use discord_rich_presence::{voice_settings::VoiceSettings, DiscordIpc, DiscordIpcClient};
use eframe::egui::{include_image, vec2, Button, ImageSource, Ui};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::{
    config::{DiscordOauthAccess, JukeBoxConfig},
    get_reqwest_client,
    input::InputKey,
};

use super::types::{Action, ActionError};

pub const AID_DISCORD_TOGGLE_MUTE: &str = "DiscordToggleMute";
pub const AID_DISCORD_TOGGLE_DEAFEN: &str = "DiscordToggleDeafen";
pub const AID_DISCORD_PUSH_TO_TALK: &str = "DiscordPushToTalk";
pub const AID_DISCORD_PUSH_TO_MUTE: &str = "DiscordPushToMute";
pub const AID_DISCORD_PUSH_TO_DEAFEN: &str = "DiscordPushToDeafen";

const ICON_MUTE: ImageSource =
    include_image!("../../../assets/action-icons/discord-microphone-2.bmp");
const ICON_DEAFEN: ImageSource =
    include_image!("../../../assets/action-icons/discord-headphones-2.bmp");
const ICON_PUSH_TO_TALK: ImageSource =
    include_image!("../../../assets/action-icons/discord-talking-1.bmp");
const ICON_PUSH_TO_MUTE: ImageSource =
    include_image!("../../../assets/action-icons/discord-microphone-1.bmp");
const ICON_PUSH_TO_DEAFEN: ImageSource =
    include_image!("../../../assets/action-icons/discord-headphones-1.bmp");

const DISCORD_CLIENT_ID: &str = env!("DISCORD_CLIENT_ID");
const DISCORD_CLIENT_SECRET: &str = env!("DISCORD_CLIENT_SECRET");
static DISCORD_CLIENT: OnceLock<Mutex<DiscordIpcClient>> = OnceLock::new();
static DISCORD_MUTED: OnceLock<Mutex<bool>> = OnceLock::new();
static DISCORD_DEAFENED: OnceLock<Mutex<bool>> = OnceLock::new();

#[rustfmt::skip]
pub fn discord_action_list() -> (String, Vec<(String, Box<dyn Action>, String)>) {
    (
        t!("action.discord.title", icon = phos::DISCORD_LOGO).into(),
        vec![
            (AID_DISCORD_TOGGLE_MUTE.into(),    Box::new(DiscordToggleMute::default()),   t!("action.discord.toggle_mute.title").into()),
            (AID_DISCORD_TOGGLE_DEAFEN.into(),  Box::new(DiscordToggleDeafen::default()), t!("action.discord.toggle_deafen.title").into()),
            (AID_DISCORD_PUSH_TO_TALK.into(),   Box::new(DiscordPushToTalk::default()),   t!("action.discord.push_to_talk.title").into()),
            (AID_DISCORD_PUSH_TO_MUTE.into(),   Box::new(DiscordPushToMute::default()),   t!("action.discord.push_to_mute.title").into()),
            (AID_DISCORD_PUSH_TO_DEAFEN.into(), Box::new(DiscordPushToDeafen::default()), t!("action.discord.push_to_deafen.title").into()),
        ],
    )
}

async fn discord_access_token_request(
    code: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<DiscordOauthAccess, ()> {
    let params = HashMap::from([
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", "http://localhost:61961"),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ]);

    let r = get_reqwest_client()
        .post("https://discord.com/api/oauth2/token")
        .form(&params)
        .send()
        .await
        .map_err(|_| ())?;

    r.json().await.map_err(|_| ())
}

async fn discord_refresh_access_token(
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<DiscordOauthAccess, ()> {
    let params = HashMap::from([
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ]);

    let r = get_reqwest_client()
        .post("https://discord.com/api/oauth2/token")
        .form(&params)
        .send()
        .await
        .map_err(|_| ())?;

    r.json().await.map_err(|_| ())
}

async fn create_client(
    device_uid: &String,
    input_key: InputKey,
    config: Arc<Mutex<JukeBoxConfig>>,
) -> Result<(), ActionError> {
    let mut client = DiscordIpcClient::new(DISCORD_CLIENT_ID);
    client
        .connect()
        .map_err(|_| ActionError::new(device_uid, input_key, t!("action.discord.err.connect")))?;

    let mut config = config.lock().await;

    if config.discord_oauth_access.is_none() {
        let code = client
            .authorize(&["rpc", "rpc.voice.read", "rpc.voice.write"])
            .map_err(|_| {
                ActionError::new(device_uid, input_key, t!("action.discord.err.authorize"))
            })?;

        let oauth = discord_access_token_request(&code, DISCORD_CLIENT_ID, DISCORD_CLIENT_SECRET)
            .await
            .map_err(|_| {
                ActionError::new(
                    device_uid,
                    input_key,
                    t!("action.discord.err.oauth_request"),
                )
            })?;

        config.discord_oauth_access = Some(oauth);
        config.save();
    } else {
        // TODO: refresh with refresh token
        let oauth = discord_refresh_access_token(
            &config.discord_oauth_access.as_ref().unwrap().refresh_token,
            DISCORD_CLIENT_ID,
            DISCORD_CLIENT_SECRET,
        )
        .await
        .map_err(|_| {
            ActionError::new(
                device_uid,
                input_key,
                t!("action.discord.err.oauth_refresh"),
            )
        })?;

        config.discord_oauth_access = Some(oauth);
        config.save();
    }

    client
        .authenticate(&config.discord_oauth_access.clone().unwrap().access_token)
        .map_err(|_| {
            ActionError::new(device_uid, input_key, t!("action.discord.err.authenticate"))
        })?;

    DISCORD_CLIENT
        .set(Mutex::new(client))
        .expect("failed to set DISCORD_CLIENT");

    Ok(())
}

fn account_warning(
    ui: &mut Ui,
    device_uid: &String,
    input_key: InputKey,
    config: Arc<Mutex<JukeBoxConfig>>,
) {
    if DISCORD_CLIENT.get().is_none() {
        let has_oauth = config.blocking_lock().discord_oauth_access.is_some();
        if has_oauth {
            let _ = tokio::runtime::Handle::current()
                .block_on(async move { create_client(device_uid, input_key, config).await });
        } else {
            ui.vertical_centered(|ui| ui.label(t!("action.discord.warning.help")));
            ui.label("");
            if ui
                .add_sized(
                    vec2(228.0, 110.0),
                    Button::new(t!("action.discord.warning.button")),
                )
                .clicked()
            {
                let _ = tokio::runtime::Handle::current()
                    .block_on(async move { create_client(device_uid, input_key, config).await });
            }
        }
    } else {
        ui.vertical_centered(|ui| ui.label(t!("action.discord.warning.success")));
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct DiscordToggleMute {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for DiscordToggleMute {
    async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(device_uid, input_key, c).await?;
        }
        let device_uid: String = device_uid.into();
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();
            let mut muted = DISCORD_MUTED
                .get_or_init(|| Mutex::new(false))
                .blocking_lock();

            *muted = !*muted;

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::VoiceActivity))
                        .mute(*muted),
                )
                .map(|_| Ok(()))
                .map_err(|_| {
                    ActionError::new(device_uid, input_key, t!("action.discord.toggle_mute.err"))
                })
        })
        .await
        .unwrap()?
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn get_type(&self) -> String {
        AID_DISCORD_TOGGLE_MUTE.into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, device_uid, input_key, config)
    }

    fn help(&self) -> String {
        t!("action.discord.toggle_mute.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_MUTE
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct DiscordToggleDeafen {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for DiscordToggleDeafen {
    async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(device_uid, input_key, c).await?;
        }
        let device_uid: String = device_uid.into();
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();
            let mut deafened = DISCORD_DEAFENED
                .get_or_init(|| Mutex::new(false))
                .blocking_lock();

            *deafened = !*deafened;

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::VoiceActivity))
                        .mute(*deafened)
                        .deaf(*deafened),
                )
                .map(|_| Ok(()))
                .map_err(|_| {
                    ActionError::new(
                        device_uid,
                        input_key,
                        t!("action.discord.toggle_deafen.err"),
                    )
                })
        })
        .await
        .unwrap()?
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn get_type(&self) -> String {
        AID_DISCORD_TOGGLE_DEAFEN.into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, device_uid, input_key, config)
    }

    fn help(&self) -> String {
        t!("action.discord.toggle_deafen.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_DEAFEN
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct DiscordPushToTalk {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for DiscordPushToTalk {
    async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(device_uid, input_key, c).await?;
        }
        let device_uid: String = device_uid.into();
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::PushToTalk))
                        .mute(false),
                )
                .map(|_| Ok(()))
                .map_err(|_| {
                    ActionError::new(
                        device_uid,
                        input_key,
                        t!("action.discord.push_to_talk.err_press"),
                    )
                })
        })
        .await
        .unwrap()?
    }

    async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(device_uid, input_key, c).await?;
        }
        let device_uid: String = device_uid.into();
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::PushToTalk))
                        .mute(true),
                )
                .map(|_| Ok(()))
                .map_err(|_| {
                    ActionError::new(
                        device_uid,
                        input_key,
                        t!("action.discord.push_to_talk.err_release"),
                    )
                })
        })
        .await
        .unwrap()?
    }

    fn get_type(&self) -> String {
        AID_DISCORD_PUSH_TO_TALK.into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, device_uid, input_key, config)
    }

    fn help(&self) -> String {
        t!("action.discord.push_to_talk.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_PUSH_TO_TALK
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct DiscordPushToMute {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for DiscordPushToMute {
    async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(device_uid, input_key, c).await?;
        }
        let device_uid: String = device_uid.into();
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::PushToTalk))
                        .mute(true),
                )
                .map(|_| Ok(()))
                .map_err(|_| {
                    ActionError::new(
                        device_uid,
                        input_key,
                        t!("action.discord.push_to_mute.err_press"),
                    )
                })
        })
        .await
        .unwrap()?
    }

    async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(device_uid, input_key, c).await?;
        }
        let device_uid: String = device_uid.into();
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::PushToTalk))
                        .mute(false),
                )
                .map(|_| Ok(()))
                .map_err(|_| {
                    ActionError::new(
                        device_uid,
                        input_key,
                        t!("action.discord.push_to_mute.err_release"),
                    )
                })
        })
        .await
        .unwrap()?
    }

    fn get_type(&self) -> String {
        AID_DISCORD_PUSH_TO_MUTE.into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, device_uid, input_key, config)
    }

    fn help(&self) -> String {
        t!("action.discord.push_to_mute.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_PUSH_TO_MUTE
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct DiscordPushToDeafen {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for DiscordPushToDeafen {
    async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(device_uid, input_key, c).await?;
        }
        let device_uid: String = device_uid.into();
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::PushToTalk))
                        .mute(true)
                        .deaf(true),
                )
                .map(|_| Ok(()))
                .map_err(|_| {
                    ActionError::new(
                        device_uid,
                        input_key,
                        t!("action.discord.push_to_deafen.err_press"),
                    )
                })
        })
        .await
        .unwrap()?
    }

    async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(device_uid, input_key, c).await?;
        }
        let device_uid: String = device_uid.into();
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::PushToTalk))
                        .mute(false)
                        .deaf(false),
                )
                .map(|_| Ok(()))
                .map_err(|_| {
                    ActionError::new(
                        device_uid,
                        input_key,
                        t!("action.discord.push_to_deafen.err_release"),
                    )
                })
        })
        .await
        .unwrap()?
    }

    fn get_type(&self) -> String {
        AID_DISCORD_PUSH_TO_DEAFEN.into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, device_uid, input_key, config)
    }

    fn help(&self) -> String {
        t!("action.discord.push_to_deafen.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_PUSH_TO_DEAFEN
    }
}
