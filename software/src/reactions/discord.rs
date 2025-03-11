use std::collections::HashMap;

use anyhow::Result;
use eframe::egui::{vec2, Button, Ui};
use egui_phosphor::regular as phos;
use serde::{Deserialize, Serialize};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Reaction, ReactionType as RT};

#[rustfmt::skip]
pub fn discord_reaction_list() -> (String, Vec<(RT, String)>) {
    (
        format!("{} Discord", phos::DISCORD_LOGO),
        vec![
            (RT::DiscordToggleMute, "Toggle Mute".to_string()),
            (RT::DiscordToggleDeafen, "Toggle Deafen".to_string()),
            (RT::DiscordPushToTalk, "Push to Talk".to_string()),
            (RT::DiscordPushToMute, "Push to Mute".to_string()),
            (RT::DiscordToggleCamera, "Toggle Camera".to_string()),
        ],
    )
}

#[rustfmt::skip]
pub fn discord_enum_map() -> HashMap<RT, Box<dyn Reaction>> {
    let mut h: HashMap<RT, Box<dyn Reaction>> = HashMap::new();
    
    h.insert(RT::DiscordToggleMute, Box::new(DiscordToggleMute::default()));
    h.insert(RT::DiscordToggleDeafen, Box::new(DiscordToggleDeafen::default()));
    h.insert(RT::DiscordPushToTalk, Box::new(DiscordPushToTalk::default()));
    h.insert(RT::DiscordPushToMute, Box::new(DiscordPushToMute::default()));
    h.insert(RT::DiscordToggleCamera, Box::new(DiscordToggleCamera::default()));

    h
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
impl Reaction for DiscordToggleMute {
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

    fn get_type(&self) -> RT {
        RT::DiscordToggleMute
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
impl Reaction for DiscordToggleDeafen {
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

    fn get_type(&self) -> RT {
        RT::DiscordToggleDeafen
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
impl Reaction for DiscordPushToTalk {
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

    fn get_type(&self) -> RT {
        RT::DiscordPushToTalk
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
impl Reaction for DiscordPushToMute {
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

    fn get_type(&self) -> RT {
        RT::DiscordPushToMute
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
impl Reaction for DiscordToggleCamera {
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

    fn get_type(&self) -> RT {
        RT::DiscordToggleCamera
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
