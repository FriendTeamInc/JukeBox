use std::process::Command;

use anyhow::Result;
use eframe::egui::{ComboBox, Slider, Ui};
use egui_phosphor::regular as phos;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Reaction, ReactionType};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemLaunchApplication {
    filepath: String,
    arguments: Vec<String>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Reaction for SystemLaunchApplication {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        let filepath = self.filepath.clone();
        let arguments = self.arguments.clone();
        let _ = spawn_blocking(move || {
            let _ = Command::new(filepath).args(arguments).spawn();
        })
        .await;

        // TODO: error handling

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

    fn get_type(&self) -> ReactionType {
        ReactionType::SystemLaunchApplication
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
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
#[async_trait::async_trait]
#[typetag::serde]
impl Reaction for SystemOpenWebsite {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        let _ = open::that(self.url.clone());
        // TODO: error handling
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

    fn get_type(&self) -> ReactionType {
        ReactionType::SystemOpenWebsite
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        ui.label("URL:");
        ui.text_edit_singleline(&mut self.url);
    }

    fn help(&self) -> String {
        "Opens a website on press.".to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemAudioInputControl {
    input_device: String,
    vol_adjust: i8,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Reaction for SystemAudioInputControl {
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

    fn get_type(&self) -> ReactionType {
        ReactionType::SystemAudioInputControl
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        ui.label("Output device:");
        ComboBox::from_id_salt("SystemAudioInputControlDeviceSelect")
            .selected_text(self.input_device.clone())
            .width(228.0)
            .show_ui(ui, |_ui| {
                // TODO
            });

        ui.label("Volume Adjust:");
        ui.add(Slider::new(&mut self.vol_adjust, -100..=100));
    }

    fn help(&self) -> String {
        "Adjust an Audio Input Device volume by specified amount on press.".to_string()
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemAudioOutputControl {
    input_device: String,
    vol_adjust: i8,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Reaction for SystemAudioOutputControl {
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

    fn get_type(&self) -> ReactionType {
        ReactionType::SystemAudioOutputControl
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        ui.label("Output device:");
        ComboBox::from_id_salt("SystemAudioOutputControlDeviceSelect")
            .selected_text(self.input_device.clone())
            .width(228.0)
            .show_ui(ui, |_ui| {
                // TODO
            });

        ui.label("Volume Adjust:");
        ui.add(Slider::new(&mut self.vol_adjust, -100..=100));
    }

    fn help(&self) -> String {
        "Adjust an Audio Output Device volume by specified amount on press.".to_string()
    }
}
