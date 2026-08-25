use std::{
    collections::HashMap,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use discord_rich_presence::{DiscordIpc, DiscordIpcClient, voice_settings::VoiceSettings};
use eframe::egui::{Button, ImageSource, Ui, include_image, vec2};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    actions::types::{
        Action, ActionError, ActionModuleConfig, ActionOk, ActionResult, ActionTrait,
    },
    get_reqwest_client,
    input::InputKey,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct DiscordOauthAccess {
    pub access_token: String,
    pub refresh_token: String,
}

pub const AMID_DISCORD: &str = "JB.Discord";
pub const AID_DISCORD_TOGGLE_MUTE: &str = "JB.Discord.ToggleMute";
pub const AID_DISCORD_TOGGLE_DEAFEN: &str = "JB.Discord.ToggleDeafen";
// pub const AID_DISCORD_TOGGLE_NOISE_SUPPRESSION: &str = "JB.Discord.ToggleNoiseSuppression";
pub const AID_DISCORD_PUSH_TO_TALK: &str = "JB.Discord.PushToTalk";
pub const AID_DISCORD_PUSH_TO_MUTE: &str = "JB.Discord.PushToMute";
pub const AID_DISCORD_PUSH_TO_DEAFEN: &str = "JB.Discord.PushToDeafen";

const ICON_MUTE: ImageSource =
    include_image!("../../../assets/action-icons/discord-microphone-1.bmp");
const ICON_MUTED: ImageSource =
    include_image!("../../../assets/action-icons/discord-microphone-2.bmp");
const ICON_DEAFEN: ImageSource =
    include_image!("../../../assets/action-icons/discord-headphones-1.bmp");
const ICON_DEAFENED: ImageSource =
    include_image!("../../../assets/action-icons/discord-headphones-2.bmp");
// const ICON_NOISE_SUPPRESSION_ON: ImageSource =
//     include_image!("../../../assets/action-icons/discord-noise-suppression-1.bmp");
// const ICON_NOISE_SUPPRESSION_OFF: ImageSource =
//     include_image!("../../../assets/action-icons/discord-noise-suppression-2.bmp");

// TODO: make new icons for push actions
const ICON_PUSH_TO_TALK: ImageSource =
    include_image!("../../../assets/action-icons/discord-talking-1.bmp");
const ICON_PUSH_TO_MUTE: ImageSource =
    include_image!("../../../assets/action-icons/discord-microphone-1.bmp");
const ICON_PUSH_TO_DEAFEN: ImageSource =
    include_image!("../../../assets/action-icons/discord-headphones-1.bmp");

const DISCORD_CLIENT_ID: Option<&str> = option_env!("DISCORD_CLIENT_ID");
const DISCORD_CLIENT_SECRET: Option<&str> = option_env!("DISCORD_CLIENT_SECRET");
const DISCORD_ACCESS_TOKEN: &str = "access_token";
const DISCORD_REFRESH_TOKEN: &str = "refresh_token";
static DISCORD_CLIENT: OnceLock<Mutex<DiscordIpcClient>> = OnceLock::new();
static DISCORD_MUTED: AtomicBool = AtomicBool::new(false);
static DISCORD_DEAFENED: AtomicBool = AtomicBool::new(false);
static DISCORD_NOISE_SUPPRESSION: AtomicBool = AtomicBool::new(false);

#[rustfmt::skip]
#[allow(dead_code)]
pub fn init_actions_discord(config: ActionModuleConfig) -> (String, Vec<Action>) {
    // init discord connection (if we have a config saved for it)
    let _ = tokio::runtime::Handle::current()
        .spawn(async move { create_client(config, true).await });

    (
        t!("action.discord.title", icon = phos::DISCORD_LOGO).into(),
        vec![
            Box::new(DiscordToggleMute::default()),
            Box::new(DiscordToggleDeafen::default()),
            Box::new(DiscordPushToTalk::default()),
            Box::new(DiscordPushToMute::default()),
            Box::new(DiscordPushToDeafen::default()),
        ],
    )
}

async fn discord_access_token_request(
    code: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<DiscordOauthAccess, ActionError> {
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
        .map_err(|e| ActionError::msg(t!("action.discord.err.oauth_request", error = e)))?;

    r.json()
        .await
        .map_err(|e| ActionError::msg(t!("action.discord.err.oauth_request", error = e)))
}

async fn discord_refresh_access_token(
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<DiscordOauthAccess, ActionError> {
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
        .map_err(|e| ActionError::msg(t!("action.discord.err.oauth_refresh", error = e)))?;

    r.json()
        .await
        .map_err(|e| ActionError::msg(t!("action.discord.err.oauth_refresh", error = e)))
}

async fn auth_client(config: ActionModuleConfig, client: &mut DiscordIpcClient) -> ActionResult {
    let mut config = config.lock().await;

    if config.get(DISCORD_ACCESS_TOKEN).is_none() || config.get(DISCORD_REFRESH_TOKEN).is_none() {
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
        .await?;

        config.insert(DISCORD_ACCESS_TOKEN.into(), oauth.access_token);
        config.insert(DISCORD_REFRESH_TOKEN.into(), oauth.refresh_token);
    } else {
        let refresh_token = config.get(DISCORD_REFRESH_TOKEN).unwrap();
        let oauth = discord_refresh_access_token(
            refresh_token,
            DISCORD_CLIENT_ID.unwrap(),
            DISCORD_CLIENT_SECRET.unwrap(),
        )
        .await?;

        config.insert(DISCORD_ACCESS_TOKEN.into(), oauth.access_token);
        config.insert(DISCORD_REFRESH_TOKEN.into(), oauth.refresh_token);
    }

    let access_token = config.get(DISCORD_ACCESS_TOKEN).unwrap();
    client.authenticate(access_token).map_err(|e| {
        ActionError::msg(t!(
            "action.discord.err.authenticate",
            error = format!("{:?}", e)
        ))
    })?;

    Ok(ActionOk::new().save_module_config(true))
}

async fn create_client(config: ActionModuleConfig, skip_if_no_auth: bool) -> ActionResult {
    if DISCORD_CLIENT_ID.is_none() || DISCORD_CLIENT_SECRET.is_none() {
        log::error!("discord: missing client id and secret from compile");
        return Err(ActionError::msg(t!("action.discord.err.compile")));
    }

    let mut client = DiscordIpcClient::new(DISCORD_CLIENT_ID.unwrap());
    client.connect().map_err(|e| {
        ActionError::msg(t!("action.discord.err.connect", error = format!("{:?}", e)))
    })?;

    let config_has_no_access_token = {
        let c = config.lock().await;
        c.get(DISCORD_ACCESS_TOKEN).is_none()
    };

    if skip_if_no_auth && config_has_no_access_token {
        return Ok(ActionOk::new());
    }

    let r = auth_client(config, &mut client).await?;

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
        if let Some(noise_suppression) = v.noise_suppression {
            DISCORD_NOISE_SUPPRESSION.store(noise_suppression, Ordering::Relaxed);
        }
    }

    // TODO theres a race condition somewhere causing this. we should investigate later.
    if DISCORD_CLIENT.get().is_none() {
        DISCORD_CLIENT
            .set(Mutex::new(client))
            .expect("failed to set DISCORD_CLIENT");
    }

    Ok(r)
}

fn account_warning(ui: &mut Ui, config: ActionModuleConfig) {
    if DISCORD_CLIENT.get().is_none() {
        let has_oauth = config.blocking_lock().get(DISCORD_ACCESS_TOKEN).is_some();
        if has_oauth {
            // TODO: send any error to gui
            // let _ = tokio::runtime::Handle::current()
            //     .block_on(async move { create_client(config, false).await });
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
                    Ok(_) => auth_client(config, &mut client).await,
                    Err(_) => Ok(ActionOk::new()),
                }
            });
        }
    }
}

fn discord_toggle_mute(
    client: &mut DiscordIpcClient,
    muted: bool,
    device_uid: &String,
    input_key: &InputKey,
) -> ActionResult {
    client
        .set_voice_settings(VoiceSettings::new().mute(muted))
        .map(|_| ActionOk::new())
        .map_err(|e| {
            ActionError::new(
                device_uid,
                *input_key,
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
    input_key: &InputKey,
) -> ActionResult {
    client
        .set_voice_settings(VoiceSettings::new().mute(deafened).deaf(deafened))
        .map(|_| ActionOk::new())
        .map_err(|e| {
            ActionError::new(
                device_uid,
                *input_key,
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
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for DiscordToggleMute {
    fn get_type(&self) -> &'static str {
        AID_DISCORD_TOGGLE_MUTE
    }
    fn get_module(&self) -> &'static str {
        AMID_DISCORD
    }
    fn get_title(&self) -> &'static str {
        "action.discord.toggle_mute.title"
    }
    fn get_description(&self) -> &'static str {
        "action.discord.toggle_mute.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        if DISCORD_CLIENT.get().is_none() {
            create_client(module_config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        let muted = !DISCORD_MUTED.load(Ordering::Relaxed);
        DISCORD_MUTED.store(muted, Ordering::Relaxed);

        if !muted {
            DISCORD_DEAFENED.store(false, Ordering::Relaxed);
        }

        discord_toggle_mute(&mut client, muted, device_uid, input_key).map(|o| o.change_icon(true))
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config)
    }

    fn icon_state(&self) -> u8 {
        if DISCORD_MUTED.load(Ordering::Relaxed) {
            1
        } else {
            0
        }
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_MUTE, ICON_MUTED]
    }

    fn icon_state_count(&self) -> u8 {
        2
    }

    fn icon_state_descriptions(&self) -> &[&str] {
        &[
            "action.discord.toggle_mute.icon_state_0",
            "action.discord.toggle_mute.icon_state_1",
        ]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiscordToggleDeafen {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for DiscordToggleDeafen {
    fn get_type(&self) -> &'static str {
        AID_DISCORD_TOGGLE_DEAFEN
    }
    fn get_module(&self) -> &'static str {
        AMID_DISCORD
    }
    fn get_title(&self) -> &'static str {
        "action.discord.toggle_deafen.title"
    }
    fn get_description(&self) -> &'static str {
        "action.discord.toggle_deafen.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        if DISCORD_CLIENT.get().is_none() {
            create_client(module_config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        let deafened = !DISCORD_DEAFENED.load(Ordering::Relaxed);
        DISCORD_DEAFENED.store(deafened, Ordering::Relaxed);
        DISCORD_MUTED.store(deafened, Ordering::Relaxed);

        discord_toggle_deafen(&mut client, deafened, device_uid, input_key)
            .map(|o| o.change_icon(true))
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config)
    }

    fn icon_state(&self) -> u8 {
        if DISCORD_DEAFENED.load(Ordering::Relaxed) {
            1
        } else {
            0
        }
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_DEAFEN, ICON_DEAFENED]
    }

    fn icon_state_count(&self) -> u8 {
        2
    }

    fn icon_state_descriptions(&self) -> &[&str] {
        &[
            "action.discord.toggle_deafen.icon_state_0",
            "action.discord.toggle_deafen.icon_state_1",
        ]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiscordPushToTalk {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for DiscordPushToTalk {
    fn get_type(&self) -> &'static str {
        AID_DISCORD_PUSH_TO_TALK
    }
    fn get_module(&self) -> &'static str {
        AMID_DISCORD
    }
    fn get_title(&self) -> &'static str {
        "action.discord.push_to_talk.title"
    }
    fn get_description(&self) -> &'static str {
        "action.discord.push_to_talk.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        if DISCORD_CLIENT.get().is_none() {
            create_client(module_config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_mute(&mut client, false, device_uid, input_key)
    }

    async fn on_release(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        if DISCORD_CLIENT.get().is_none() {
            create_client(module_config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_mute(&mut client, true, device_uid, input_key)
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config)
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_PUSH_TO_TALK]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiscordPushToMute {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for DiscordPushToMute {
    fn get_type(&self) -> &'static str {
        AID_DISCORD_PUSH_TO_MUTE
    }
    fn get_module(&self) -> &'static str {
        AMID_DISCORD
    }
    fn get_title(&self) -> &'static str {
        "action.discord.push_to_mute.title"
    }
    fn get_description(&self) -> &'static str {
        "action.discord.push_to_mute.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        if DISCORD_CLIENT.get().is_none() {
            create_client(module_config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_mute(&mut client, true, device_uid, input_key)
    }

    async fn on_release(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        if DISCORD_CLIENT.get().is_none() {
            create_client(module_config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_mute(&mut client, false, device_uid, input_key)
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config)
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_PUSH_TO_MUTE]
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DiscordPushToDeafen {}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for DiscordPushToDeafen {
    fn get_type(&self) -> &'static str {
        AID_DISCORD_PUSH_TO_DEAFEN
    }
    fn get_module(&self) -> &'static str {
        AMID_DISCORD
    }
    fn get_title(&self) -> &'static str {
        "action.discord.push_to_deafen.title"
    }
    fn get_description(&self) -> &'static str {
        "action.discord.push_to_deafen.help"
    }

    async fn on_press(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        if DISCORD_CLIENT.get().is_none() {
            create_client(module_config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_deafen(&mut client, true, device_uid, input_key)
    }

    async fn on_release(
        &mut self,
        module_config: ActionModuleConfig,
        device_uid: &String,
        input_key: &InputKey,
    ) -> ActionResult {
        if DISCORD_CLIENT.get().is_none() {
            create_client(module_config.clone(), false).await?;
        }
        let mut client = DISCORD_CLIENT.get().unwrap().lock().await;

        discord_toggle_deafen(&mut client, false, device_uid, input_key)
    }

    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>),
        module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        ui: &mut Ui,
    ) {
        account_warning(ui, module_config)
    }

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_PUSH_TO_DEAFEN]
    }
}
