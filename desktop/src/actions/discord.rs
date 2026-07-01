use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, OnceLock,
    },
};

use discord_rich_presence::{voice_settings::VoiceSettings, DiscordIpc, DiscordIpcClient};
use eframe::egui::{include_image, vec2, Button, ImageSource, Ui};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

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
    include_image!("../../../assets/action-icons/discord-microphone-1.bmp");
const ICON_MUTED: ImageSource =
    include_image!("../../../assets/action-icons/discord-microphone-2.bmp");
const ICON_DEAFEN: ImageSource =
    include_image!("../../../assets/action-icons/discord-headphones-1.bmp");
const ICON_DEAFENED: ImageSource =
    include_image!("../../../assets/action-icons/discord-headphones-2.bmp");

// TODO: make new icons for push actions
const ICON_PUSH_TO_TALK: ImageSource =
    include_image!("../../../assets/action-icons/discord-talking-1.bmp");
const ICON_PUSH_TO_MUTE: ImageSource =
    include_image!("../../../assets/action-icons/discord-microphone-1.bmp");
const ICON_PUSH_TO_DEAFEN: ImageSource =
    include_image!("../../../assets/action-icons/discord-headphones-1.bmp");

const DISCORD_CLIENT_ID: Option<&str> = option_env!("DISCORD_CLIENT_ID");
const DISCORD_CLIENT_SECRET: Option<&str> = option_env!("DISCORD_CLIENT_SECRET");
static DISCORD_CLIENT: OnceLock<Mutex<DiscordIpcClient>> = OnceLock::new();
static DISCORD_MUTED: AtomicBool = AtomicBool::new(false);
static DISCORD_DEAFENED: AtomicBool = AtomicBool::new(false);

#[rustfmt::skip]
#[allow(dead_code)]
pub fn init_actions_discord(config: Arc<Mutex<JukeBoxConfig>>) -> (String, Vec<(String, Action, String)>) {
    // init discord connection (if we have a config saved for it)
    let _ = tokio::runtime::Handle::current()
        .spawn(async move { create_client(config, true).await });

    (
        t!("action.discord.title", icon = phos::DISCORD_LOGO).into(),
        vec![
            (AID_DISCORD_TOGGLE_MUTE.into(),    Action::DiscordToggleMute(DiscordToggleMute::default()),     t!("action.discord.toggle_mute.title").into()),
            (AID_DISCORD_TOGGLE_DEAFEN.into(),  Action::DiscordToggleDeafen(DiscordToggleDeafen::default()), t!("action.discord.toggle_deafen.title").into()),
            (AID_DISCORD_PUSH_TO_TALK.into(),   Action::DiscordPushToTalk(DiscordPushToTalk::default()),     t!("action.discord.push_to_talk.title").into()),
            (AID_DISCORD_PUSH_TO_MUTE.into(),   Action::DiscordPushToMute(DiscordPushToMute::default()),     t!("action.discord.push_to_mute.title").into()),
            (AID_DISCORD_PUSH_TO_DEAFEN.into(), Action::DiscordPushToDeafen(DiscordPushToDeafen::default()), t!("action.discord.push_to_deafen.title").into()),
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

async fn auth_client(
    config: Arc<Mutex<JukeBoxConfig>>,
    client: &mut DiscordIpcClient,
    skip_if_no_auth: bool,
) -> Result<(), ActionError> {
    let mut config = config.lock().await;

    if config.discord_oauth_access.is_none() {
        if skip_if_no_auth {
            return Ok(());
        }

        let code = client
            .authorize(&["rpc", "rpc.voice.read", "rpc.voice.write"])
            .map_err(|e| {
                ActionError::msg(t!(
                    "action.discord.err.authorize",
                    error = format!("{:?}", e)
                ))
            })?;

        let oauth = discord_access_token_request(
            &code,
            DISCORD_CLIENT_ID.unwrap(),
            DISCORD_CLIENT_SECRET.unwrap(),
        )
        .await
        .map_err(|_| ActionError::msg(t!("action.discord.err.oauth_request")))?;

        config.discord_oauth_access = Some(oauth);
        config.save();
    } else {
        let oauth = discord_refresh_access_token(
            &config.discord_oauth_access.as_ref().unwrap().refresh_token,
            DISCORD_CLIENT_ID.unwrap(),
            DISCORD_CLIENT_SECRET.unwrap(),
        )
        .await
        .map_err(|_| ActionError::msg(t!("action.discord.err.oauth_refresh")))?;

        config.discord_oauth_access = Some(oauth);
        config.save();
    }

    client
        .authenticate(&config.discord_oauth_access.clone().unwrap().access_token)
        .map_err(|e| {
            ActionError::msg(t!(
                "action.discord.err.authenticate",
                error = format!("{:?}", e)
            ))
        })?;

    Ok(())
}

async fn create_client(
    config: Arc<Mutex<JukeBoxConfig>>,
    skip_if_no_auth: bool,
) -> Result<(), ActionError> {
    if DISCORD_CLIENT_ID.is_none() || DISCORD_CLIENT_SECRET.is_none() {
        log::error!("discord: missing client id and secret from compile");
        return Err(ActionError::msg(t!("action.discord.err.compile")));
    }

    let mut client = DiscordIpcClient::new(DISCORD_CLIENT_ID.unwrap());
    client.connect().map_err(|e| {
        ActionError::msg(t!("action.discord.err.connect", error = format!("{:?}", e)))
    })?;

    auth_client(config, &mut client, skip_if_no_auth).await?;

    if let Ok(v) = client.get_voice_settings() {
        let deaf = if let Some(deaf) = v.deaf {
            DISCORD_DEAFENED.store(deaf, Ordering::Relaxed);
            DISCORD_MUTED.store(deaf, Ordering::Relaxed);
            deaf
        } else {
            false
        };
        if let Some(mute) = v.mute {
            DISCORD_MUTED.store(mute || deaf, Ordering::Relaxed);
        }
    }

    DISCORD_CLIENT
        .set(Mutex::new(client))
        .expect("failed to set DISCORD_CLIENT");

    Ok(())
}

fn account_warning(ui: &mut Ui, config: Arc<Mutex<JukeBoxConfig>>) {
    if DISCORD_CLIENT.get().is_none() {
        let has_oauth = config.blocking_lock().discord_oauth_access.is_some();
        if has_oauth {
            // TODO: send any error to gui
            tokio::runtime::Handle::current()
                .spawn(async move { create_client(config, false).await });
        } else {
            ui.vertical_centered(|ui| ui.label(t!("action.discord.warning.help")));
            ui.label("");
            if ui
                .add_sized(
                    vec2(228.0, 100.0),
                    Button::new(t!("action.discord.warning.connect_button")),
                )
                .clicked()
            {
                // TODO: send any error to gui
                tokio::runtime::Handle::current()
                    .spawn(async move { create_client(config, false).await });
            }
        }
    } else {
        ui.vertical_centered(|ui| ui.label(t!("action.discord.warning.success")));
        ui.label("");
        if ui
            .add_sized(
                vec2(228.0, 100.0),
                Button::new(t!("action.discord.warning.reconnect_button")),
            )
            .clicked()
        {
            // TODO: send any error to gui
            tokio::runtime::Handle::current().spawn(async move {
                let mut client = DISCORD_CLIENT.get().unwrap().lock().await;
                match client.reconnect() {
                    Ok(_) => auth_client(config, &mut client, false).await,
                    Err(_) => Ok(()),
                }
            });
        }
    }
}

fn discord_toggle_mute(
    client: &mut DiscordIpcClient,
    muted: bool,
    device_uid: &String,
    input_key: InputKey,
) -> Result<(InputKey, bool), ActionError> {
    client
        .set_voice_settings(VoiceSettings::new().mute(muted))
        .map(|_| (input_key, true))
        .map_err(|e| {
            ActionError::new(
                device_uid,
                input_key,
                t!(
                    "action.discord.err.set_mute_state",
                    state = muted,
                    error = format!("{:?}", e)
                ),
            )
        })
}

fn discord_toggle_deafen(
    client: &mut DiscordIpcClient,
    deafened: bool,
    device_uid: &String,
    input_key: InputKey,
) -> Result<(InputKey, bool), ActionError> {
    client
        .set_voice_settings(VoiceSettings::new().mute(deafened).deaf(deafened))
        .map(|_| (input_key, true))
        .map_err(|e| {
            ActionError::new(
                device_uid,
                input_key,
                t!(
                    "action.discord.err.set_deafen_state",
                    state = deafened,
                    error = format!("{:?}", e)
                ),
            )
        })
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiscordToggleMute {}
impl DiscordToggleMute {
    pub async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(InputKey, bool), ActionError> {
        if DISCORD_CLIENT.get().is_none() {
            create_client(config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        let muted = !DISCORD_MUTED.load(Ordering::Relaxed);
        DISCORD_MUTED.store(muted, Ordering::Relaxed);

        if !muted {
            DISCORD_DEAFENED.store(false, Ordering::Relaxed);
        }

        discord_toggle_mute(&mut client, muted, &device_uid, input_key)
    }

    pub async fn on_release(
        &self,
        _device_uid: &String,
        input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(InputKey, bool), ActionError> {
        Ok((input_key, false))
    }

    pub fn get_type(&self) -> String {
        AID_DISCORD_TOGGLE_MUTE.into()
    }

    pub fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, config)
    }

    pub fn help(&self) -> &str {
        "action.discord.toggle_mute.help"
    }

    pub fn icon_state(&self) -> u8 {
        if DISCORD_MUTED.load(Ordering::Relaxed) {
            1
        } else {
            0
        }
    }

    pub fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_MUTE, ICON_MUTED]
    }

    pub fn icon_state_count(&self) -> u8 {
        2
    }

    pub fn icon_state_descriptions(&self) -> &[&str] {
        &[
            "action.discord.toggle_mute.icon_state_0",
            "action.discord.toggle_mute.icon_state_1",
        ]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiscordToggleDeafen {}
impl DiscordToggleDeafen {
    pub async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(InputKey, bool), ActionError> {
        if DISCORD_CLIENT.get().is_none() {
            create_client(config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        let deafened = !DISCORD_DEAFENED.load(Ordering::Relaxed);
        DISCORD_DEAFENED.store(deafened, Ordering::Relaxed);
        DISCORD_MUTED.store(deafened, Ordering::Relaxed);

        discord_toggle_deafen(&mut client, deafened, &device_uid, input_key)
    }

    pub async fn on_release(
        &self,
        _device_uid: &String,
        input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(InputKey, bool), ActionError> {
        Ok((input_key, false))
    }

    pub fn get_type(&self) -> String {
        AID_DISCORD_TOGGLE_DEAFEN.into()
    }

    pub fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, config)
    }

    pub fn help(&self) -> &str {
        "action.discord.toggle_deafen.help"
    }

    pub fn icon_state(&self) -> u8 {
        if DISCORD_DEAFENED.load(Ordering::Relaxed) {
            1
        } else {
            0
        }
    }

    pub fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_DEAFEN, ICON_DEAFENED]
    }

    pub fn icon_state_count(&self) -> u8 {
        2
    }

    pub fn icon_state_descriptions(&self) -> &[&str] {
        &[
            "action.discord.toggle_deafen.icon_state_0",
            "action.discord.toggle_deafen.icon_state_1",
        ]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiscordPushToTalk {}
impl DiscordPushToTalk {
    pub async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(InputKey, bool), ActionError> {
        if DISCORD_CLIENT.get().is_none() {
            create_client(config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_mute(&mut client, false, &device_uid, input_key)
    }

    pub async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(InputKey, bool), ActionError> {
        if DISCORD_CLIENT.get().is_none() {
            create_client(config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_mute(&mut client, true, &device_uid, input_key)
    }

    pub fn get_type(&self) -> String {
        AID_DISCORD_PUSH_TO_TALK.into()
    }

    pub fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, config)
    }

    pub fn help(&self) -> &str {
        "action.discord.push_to_talk.help"
    }

    pub fn icon_state(&self) -> u8 {
        0
    }

    pub fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_PUSH_TO_TALK]
    }

    pub fn icon_state_count(&self) -> u8 {
        1
    }

    pub fn icon_state_descriptions(&self) -> &[&str] {
        &[""]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiscordPushToMute {}
impl DiscordPushToMute {
    pub async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(InputKey, bool), ActionError> {
        if DISCORD_CLIENT.get().is_none() {
            create_client(config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_mute(&mut client, true, &device_uid, input_key)
    }

    pub async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(InputKey, bool), ActionError> {
        if DISCORD_CLIENT.get().is_none() {
            create_client(config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_mute(&mut client, false, &device_uid, input_key)
    }

    pub fn get_type(&self) -> String {
        AID_DISCORD_PUSH_TO_MUTE.into()
    }

    pub fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, config)
    }

    pub fn help(&self) -> &str {
        "action.discord.push_to_mute.help"
    }

    pub fn icon_state(&self) -> u8 {
        0
    }

    pub fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_PUSH_TO_MUTE]
    }

    pub fn icon_state_count(&self) -> u8 {
        1
    }

    pub fn icon_state_descriptions(&self) -> &[&str] {
        &[""]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiscordPushToDeafen {}
impl DiscordPushToDeafen {
    pub async fn on_press(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(InputKey, bool), ActionError> {
        if DISCORD_CLIENT.get().is_none() {
            create_client(config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_deafen(&mut client, true, &device_uid, input_key)
    }

    pub async fn on_release(
        &self,
        device_uid: &String,
        input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(InputKey, bool), ActionError> {
        if DISCORD_CLIENT.get().is_none() {
            create_client(config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_deafen(&mut client, false, &device_uid, input_key)
    }

    pub fn get_type(&self) -> String {
        AID_DISCORD_PUSH_TO_DEAFEN.into()
    }

    pub fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        account_warning(ui, config)
    }

    pub fn help(&self) -> &str {
        "action.discord.push_to_deafen.help"
    }

    pub fn icon_state(&self) -> u8 {
        0
    }

    pub fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_PUSH_TO_DEAFEN]
    }

    pub fn icon_state_count(&self) -> u8 {
        1
    }

    pub fn icon_state_descriptions(&self) -> &[&str] {
        &[""]
    }
}
