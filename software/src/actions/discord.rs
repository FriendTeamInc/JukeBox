use anyhow::Result;
use eframe::egui::{vec2, Button, Ui};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Action, ActionType as AT};

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

fn account_warning(ui: &mut Ui) {
    ui.vertical_centered(|ui| ui.label(t!("action.discord.warning.help")));
    ui.label("");
    if ui
        .add_sized(
            vec2(228.0, 110.0),
            Button::new(t!("action.discord.warning.button")),
        )
        .clicked()
    {
        log::info!("TODO: Discord integration");
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
