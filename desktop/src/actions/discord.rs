use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use anyhow::{bail, Result};
use discord_rich_presence::{voice_settings::VoiceSettings, DiscordIpc, DiscordIpcClient};
use eframe::egui::{include_image, vec2, Button, ImageSource, Ui};
use egui_phosphor::regular as phos;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::{
    config::{DiscordOauthAccess, JukeBoxConfig},
    input::InputKey,
};

use super::types::Action;

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
static REQWEST_CLIENT: OnceLock<Mutex<Client>> = OnceLock::new();

#[rustfmt::skip]
pub fn discord_action_list() -> (String, Vec<(String, Box<dyn Action>, String)>) {
    (
        t!("action.discord.title", icon = phos::DISCORD_LOGO).into(),
        vec![
            ("DiscordToggleMute".into(),   Box::new(DiscordToggleMute::default()),   t!("action.discord.toggle_mute.title").into()),
            ("DiscordToggleDeafen".into(), Box::new(DiscordToggleDeafen::default()), t!("action.discord.toggle_deafen.title").into()),
            ("DiscordPushToTalk".into(),   Box::new(DiscordPushToTalk::default()),   t!("action.discord.push_to_talk.title").into()),
            ("DiscordPushToMute".into(),   Box::new(DiscordPushToMute::default()),   t!("action.discord.push_to_mute.title").into()),
            ("DiscordPushToDeafen".into(), Box::new(DiscordPushToDeafen::default()), t!("action.discord.push_to_deafen.title").into()),
        ],
    )
}

async fn discord_access_token_request(
    code: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<DiscordOauthAccess> {
    let params = HashMap::from([
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", "http://localhost:61961"),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ]);

    let r = REQWEST_CLIENT
        .get_or_init(|| Mutex::new(Client::new()))
        .lock()
        .await
        .post("https://discord.com/api/oauth2/token")
        .form(&params)
        .send()
        .await?;

    if let Ok(j) = r.json().await {
        Ok(j)
    } else {
        bail!("failed to gain oauth access");
    }
}

async fn discord_refresh_access_token(
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<DiscordOauthAccess> {
    let params = HashMap::from([
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ]);

    let r = REQWEST_CLIENT
        .get_or_init(|| Mutex::new(Client::new()))
        .lock()
        .await
        .post("https://discord.com/api/oauth2/token")
        .form(&params)
        .send()
        .await?;

    if let Ok(j) = r.json().await {
        Ok(j)
    } else {
        bail!("failed to refresh oauth access");
    }
}

// TODO: better error handling
async fn create_client(config: Arc<Mutex<JukeBoxConfig>>) -> Result<()> {
    let mut client = DiscordIpcClient::new(DISCORD_CLIENT_ID);
    client.connect().expect("cannot connect to discord");

    let mut config = config.lock().await;

    if config.discord_oauth_access.is_none() {
        let code = client
            .authorize(&["rpc", "rpc.voice.read", "rpc.voice.write"])
            .expect("failed to authorize wtih discord");
        let oauth = discord_access_token_request(&code, DISCORD_CLIENT_ID, DISCORD_CLIENT_SECRET)
            .await
            .expect("failed to get oauth access token");

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
        .expect("failed to refresh oauth access token");

        config.discord_oauth_access = Some(oauth);
        config.save();
    }

    client
        .authenticate(&config.discord_oauth_access.clone().unwrap().access_token)
        .expect("failed to authenticate with discord");

    DISCORD_CLIENT
        .set(Mutex::new(client))
        .expect("failed to set DISCORD_CLIENT");

    Ok(())
}

fn account_warning(ui: &mut Ui, config: Arc<Mutex<JukeBoxConfig>>) {
    if DISCORD_CLIENT.get().is_none() {
        let has_oauth = config.blocking_lock().discord_oauth_access.is_some();
        if has_oauth {
            let _ = tokio::runtime::Handle::current()
                .block_on(async move { create_client(config).await });
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
                    .block_on(async move { create_client(config).await });
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
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(c).await?;
        }
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
                .map_err(anyhow::Error::from)
        })
        .await
        .unwrap()?
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
        "DiscordToggleMute".into()
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
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(c).await?;
        }
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
                .map_err(anyhow::Error::from)
        })
        .await
        .unwrap()?
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
        "DiscordToggleDeafen".into()
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
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(c).await?;
        }
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::PushToTalk))
                        .mute(false),
                )
                .map(|_| Ok(()))
                .map_err(anyhow::Error::from)
        })
        .await
        .unwrap()?
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(c).await?;
        }
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::PushToTalk))
                        .mute(true),
                )
                .map(|_| Ok(()))
                .map_err(anyhow::Error::from)
        })
        .await
        .unwrap()?
    }

    fn get_type(&self) -> String {
        "DiscordPushToTalk".into()
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
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(c).await?;
        }
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::PushToTalk))
                        .mute(true),
                )
                .map(|_| Ok(()))
                .map_err(anyhow::Error::from)
        })
        .await
        .unwrap()?
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(c).await?;
        }
        spawn_blocking(move || {
            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();

            client
                .set_voice_settings(
                    VoiceSettings::new()
                        // .mode(VoiceModeSettings::new().voice_mode(VoiceMode::PushToTalk))
                        .mute(false),
                )
                .map(|_| Ok(()))
                .map_err(anyhow::Error::from)
        })
        .await
        .unwrap()?
    }

    fn get_type(&self) -> String {
        "DiscordPushToMute".into()
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
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(c).await?;
        }
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
                .map_err(anyhow::Error::from)
        })
        .await
        .unwrap()?
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let c = config.clone();
        if DISCORD_CLIENT.get().is_none() {
            create_client(c).await?;
        }
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
                .map_err(anyhow::Error::from)
        })
        .await
        .unwrap()?
    }

    fn get_type(&self) -> String {
        "DiscordPushToDeafen".into()
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
        t!("action.discord.push_to_deafen.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_PUSH_TO_DEAFEN
    }
}
