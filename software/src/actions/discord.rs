use anyhow::Result;
use eframe::egui::{vec2, Button, Ui};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Action, ActionType as AT};

#[rustfmt::skip]
pub fn discord_action_list() -> (String, Vec<(AT, Box<dyn Action>, String)>) {
    (
        format!("{} Discord", phos::DISCORD_LOGO),
        vec![
            (AT::DiscordToggleMute,   Box::new(DiscordToggleMute::default()),   "Toggle Mute".to_string()),
            (AT::DiscordToggleDeafen, Box::new(DiscordToggleDeafen::default()), "Toggle Deafen".to_string()),
            (AT::DiscordPushToTalk,   Box::new(DiscordPushToTalk::default()),   "Push to Talk".to_string()),
            (AT::DiscordPushToMute,   Box::new(DiscordPushToMute::default()),   "Push to Mute".to_string()),
            (AT::DiscordToggleCamera, Box::new(DiscordToggleCamera::default()), "Toggle Camera".to_string()),
        ],
    )
}

fn account_warning(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.label("You need to connect your Discord Account before using this function.")
    });
    ui.label("");
    if ui
        .add_sized(
            vec2(228.0, 110.0),
            Button::new("Connect to Discord Account"),
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
        "Toggle mutes your microphone on Discord when pressed.".to_string()
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
        "Toggle deafens your audio on Discord when pressed.".to_string()
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
        "Unmutes your microphone on Discord while held.".to_string()
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
        "Mutes your microphone on Discord while held.".to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct DiscordToggleCamera {}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for DiscordToggleCamera {
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
        AT::DiscordToggleCamera
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
        "Toggles your camera on Discord when pressed.".to_string()
    }
}
