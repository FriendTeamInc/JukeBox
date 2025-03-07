use eframe::egui::{vec2, Button, Ui};
use serde::{Deserialize, Serialize};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Reaction, ReactionType};

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
#[typetag::serde]
impl Reaction for DiscordToggleMute {
    fn on_press(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        // TODO
    }

    fn on_release(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {}

    fn get_type(&self) -> ReactionType {
        ReactionType::DiscordToggleMute
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: String,
        _key: InputKey,
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
#[typetag::serde]
impl Reaction for DiscordToggleDeafen {
    fn on_press(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        // TODO
    }

    fn on_release(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {}

    fn get_type(&self) -> ReactionType {
        ReactionType::DiscordToggleDeafen
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: String,
        _key: InputKey,
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
#[typetag::serde]
impl Reaction for DiscordPushToTalk {
    fn on_press(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        // TODO
    }

    fn on_release(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        // TODO
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::DiscordPushToTalk
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: String,
        _key: InputKey,
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
#[typetag::serde]
impl Reaction for DiscordPushToMute {
    fn on_press(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        // TODO
    }

    fn on_release(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        // TODO
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::DiscordPushToMute
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: String,
        _key: InputKey,
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
#[typetag::serde]
impl Reaction for DiscordToggleCamera {
    fn on_press(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        // TODO
    }

    fn on_release(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        // TODO
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::DiscordToggleCamera
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: String,
        _key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        account_warning(ui);
    }

    fn help(&self) -> String {
        "Toggles your camera on Discord when pressed.".to_string()
    }
}
