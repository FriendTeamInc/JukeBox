use std::{collections::HashMap, error::Error, fmt, sync::OnceLock};

use anyhow::Result;
use discord_rich_presence_client::{voice_settings::VoiceSettings, DiscordIpc, DiscordIpcClient};
use eframe::egui::{vec2, Button, Ui};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Action, ActionType as AT};

const DISCORD_CLIENT_ID: Option<&str> = option_env!("JUKEBOXDESKTOP_DISCORD_CLIENT_ID");
const DISCORD_CLIENT_SECRET: Option<&str> = option_env!("JUKEBOXDESKTOP_DISCORD_CLIENT_SECRET");
static DISCORD_CLIENT: OnceLock<Mutex<DiscordIpcClient>> = OnceLock::new();

#[rustfmt::skip]
pub fn discord_action_list() -> (String, Vec<(AT, Box<dyn Action>, String)>) {
    (
        t!("action.discord.title", icon = phos::DISCORD_LOGO).to_string(),
        vec![
            (AT::DiscordToggleMute,   Box::new(DiscordToggleMute::default()),   t!("action.discord.toggle_mute.title").to_string()),
            (AT::DiscordToggleDeafen, Box::new(DiscordToggleDeafen::default()), t!("action.discord.toggle_deafen.title").to_string()),
            (AT::DiscordPushToTalk,   Box::new(DiscordPushToTalk::default()),   t!("action.discord.push_to_talk.title").to_string()),
            (AT::DiscordPushToMute,   Box::new(DiscordPushToMute::default()),   t!("action.discord.push_to_mute.title").to_string()),
        ],
    )
}

#[derive(Serialize, Deserialize, Debug)]
struct OauthAccess {
    token_type: String,
    access_token: String,
    expires_in: usize,
    refresh_token: String,
    scope: String,
}
#[derive(Debug)]
struct OauthAccessError {}
impl fmt::Display for OauthAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to gain oauth access")
    }
}
impl Error for OauthAccessError {}

fn discord_access_token_request(
    code: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<OauthAccess, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let params = HashMap::from([
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", "http://localhost:61961"),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ]);

    let r = client
        .post("https://discord.com/api/oauth2/token")
        .form(&params)
        .send()?;

    if let Ok(j) = r.json() {
        log::debug!("{:?}", j);
        Ok(j)
    } else {
        Err(Box::new(OauthAccessError {}))
    }
}

fn account_warning(ui: &mut Ui) {
    if DISCORD_CLIENT.get().is_none() {
        ui.vertical_centered(|ui| ui.label(t!("action.discord.warning.help")));
        ui.label("");
        if ui
            .add_sized(
                vec2(228.0, 110.0),
                Button::new(t!("action.discord.warning.button")),
            )
            .clicked()
        {
            let mut client = DiscordIpcClient::new(DISCORD_CLIENT_ID.unwrap());
            client.connect().expect("cannot connect to discord");
            let code = client
                .authorize(&["rpc", "rpc.voice.read", "rpc.voice.write"])
                .expect("failed to authorize wtih discord");
            let oauth = discord_access_token_request(
                &code,
                DISCORD_CLIENT_ID.unwrap(),
                DISCORD_CLIENT_SECRET.unwrap(),
            )
            .expect("failed to get oauth access token");
            client
                .authenticate(&oauth.access_token)
                .expect("failed to authenticate with discord");

            DISCORD_CLIENT
                .set(Mutex::new(client))
                .expect("failed to set DISCORD_CLIENT");
        }
    } else {
        ui.vertical_centered(|ui| ui.label(t!("action.discord.warning.success")));
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct DiscordToggleMute {
    // muted: bool,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for DiscordToggleMute {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        // TODO
        let _ = spawn_blocking(|| {
            if DISCORD_CLIENT.get().is_none() {
                return;
            }

            let mut client = DISCORD_CLIENT.get().unwrap().blocking_lock();
            client
                .set_voice_settings(VoiceSettings::new().mute(true))
                .expect("fuck");
        })
        .await;

        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> AT {
        AT::DiscordToggleMute
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        account_warning(ui);
    }

    fn help(&self) -> String {
        t!("action.discord.toggle_mute.help").to_string()
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
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> AT {
        AT::DiscordToggleDeafen
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        account_warning(ui);
    }

    fn help(&self) -> String {
        t!("action.discord.toggle_deafen.help").to_string()
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
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    fn get_type(&self) -> AT {
        AT::DiscordPushToTalk
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        account_warning(ui);
    }

    fn help(&self) -> String {
        t!("action.discord.push_to_talk.help").to_string()
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
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    fn get_type(&self) -> AT {
        AT::DiscordPushToMute
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        account_warning(ui);
    }

    fn help(&self) -> String {
        t!("action.discord.push_to_mute.help").to_string()
    }
}
