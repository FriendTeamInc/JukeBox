use std::process::Command;

use eframe::egui::Ui;
use egui_phosphor::regular as phos;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};

use crate::input::InputKey;

use super::types::{Reaction, ReactionType};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ActSystemLaunchApplication {
    filepath: String,
    arguments: Vec<String>,
}
#[typetag::serde]
impl Reaction for ActSystemLaunchApplication {
    fn on_press(&self, _key: InputKey) -> () {}

    fn on_release(&self, _key: InputKey) -> () {
        let _ = Command::new(self.filepath.clone())
            .args(self.arguments.clone())
            .spawn();
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::SystemLaunchApplication
    }

    fn edit_ui(&mut self, ui: &mut Ui) {
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
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ActSystemOpenWebsite {
    url: String,
}
#[typetag::serde]
impl Reaction for ActSystemOpenWebsite {
    fn on_press(&self, _key: InputKey) -> () {}

    fn on_release(&self, _key: InputKey) -> () {
        let _ = open::that(self.url.clone());
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::SystemOpenWebsite
    }

    fn edit_ui(&mut self, ui: &mut Ui) {
        ui.label("URL:");
        ui.text_edit_singleline(&mut self.url);
    }
}
