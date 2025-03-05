use std::process::Command;

use eframe::egui::Ui;
use egui_phosphor::regular as phos;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Reaction, ReactionType};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemLaunchApplication {
    filepath: String,
    arguments: Vec<String>,
}
#[typetag::serde]
impl Reaction for SystemLaunchApplication {
    fn on_press(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        let _ = Command::new(self.filepath.clone())
            .args(self.arguments.clone())
            .spawn();
    }

    fn on_release(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {}

    fn get_type(&self) -> ReactionType {
        ReactionType::SystemLaunchApplication
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: String,
        _key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        if ui.button("Choose File").clicked() {
            if let Some(f) = FileDialog::new().pick_file() {
                self.filepath = f.to_str().unwrap().to_owned();
            }
        }
        ui.text_edit_singleline(&mut self.filepath);
        ui.horizontal(|ui| {
            ui.label("Arguments:");
            if ui.button("+").clicked() {
                self.arguments.push(String::new());
            }
        });
        let mut delete = Vec::new();
        for (i, a) in self.arguments.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui.button(phos::TRASH).clicked() {
                    delete.push(i);
                }
                ui.text_edit_singleline(a);
            });
        }
        delete.reverse();
        for i in delete {
            self.arguments.remove(i);
        }
    }

    fn help(&self) -> String {
        "Launches a system application on press.".to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemOpenWebsite {
    url: String,
}
#[typetag::serde]
impl Reaction for SystemOpenWebsite {
    fn on_press(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        let _ = open::that(self.url.clone());
    }

    fn on_release(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {}

    fn get_type(&self) -> ReactionType {
        ReactionType::SystemOpenWebsite
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: String,
        _key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        ui.label("URL:");
        ui.text_edit_singleline(&mut self.url);
    }

    fn help(&self) -> String {
        "Opens a website on press.".to_string()
    }
}
