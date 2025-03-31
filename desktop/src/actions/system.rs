use std::{process::Command, sync::Arc};

use anyhow::Result;
use eframe::egui::{include_image, ComboBox, ImageSource, Slider, Ui};
use egui_phosphor::regular as phos;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::Action;

const ICON_LAUNCH_APPLICATION: ImageSource =
    include_image!("../../../assets/action-icons/system-appopen.bmp");
const ICON_OPEN_WEBSITE: ImageSource =
    include_image!("../../../assets/action-icons/system-webopen.bmp");
const ICON_INPUT_CONTROL: ImageSource =
    include_image!("../../../assets/action-icons/system-inputcontrol.bmp");
const ICON_OUTPUT_CONTROL: ImageSource =
    include_image!("../../../assets/action-icons/system-outputcontrol.bmp");

#[rustfmt::skip]
pub fn system_action_list() -> (String, Vec<(String, Box<dyn Action>, String)>) {
    (
        t!("action.system.title", icon = phos::DESKTOP_TOWER).into(),
        vec![
            ("SystemLaunchApplication".into(),  Box::new(SystemLaunchApplication::default()),  t!("action.system.launch_app.title").into()),
            ("SystemOpenWebsite".into(),        Box::new(SystemOpenWebsite::default()),        t!("action.system.open_website.title").into()),
            ("SystemAudioInputControl".into(),  Box::new(SystemAudioInputControl::default()),  t!("action.system.audio_input_control.title").into()),
            ("SystemAudioOutputControl".into(), Box::new(SystemAudioOutputControl::default()), t!("action.system.audio_output_control.title").into()),
        ],
    )
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemLaunchApplication {
    filepath: String,
    arguments: Vec<String>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for SystemLaunchApplication {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
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
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> String {
        "SystemLaunchApplication".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        if ui
            .button(t!("action.system.launch_app.choose_file"))
            .clicked()
        {
            if let Some(f) = FileDialog::new().pick_file() {
                self.filepath = f.to_str().unwrap().to_owned();
            }
        }
        ui.text_edit_singleline(&mut self.filepath);
        ui.horizontal(|ui| {
            ui.label(t!("action.system.launch_app.add_arguments"));
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
        t!("action.system.launch_app.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_LAUNCH_APPLICATION
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemOpenWebsite {
    url: String,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for SystemOpenWebsite {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        let _ = open::that(self.url.clone());
        // TODO: error handling
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> String {
        "SystemOpenWebsite".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.label(t!("action.system.open_website.url"));
        ui.text_edit_singleline(&mut self.url);
    }

    fn help(&self) -> String {
        t!("action.system.open_website.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_OPEN_WEBSITE
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemAudioInputControl {
    input_device: String,
    vol_adjust: i8,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for SystemAudioInputControl {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> String {
        "SystemAudioInputControl".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.label(t!("action.system.audio_input_control.input_device"));
        ComboBox::from_id_salt("SystemAudioInputControlDeviceSelect")
            .selected_text(self.input_device.clone())
            .width(228.0)
            .show_ui(ui, |_ui| {
                // TODO
            });

        ui.label(t!("action.system.audio_input_control.volume_adjust"));
        ui.add(Slider::new(&mut self.vol_adjust, -100..=100));
    }

    fn help(&self) -> String {
        t!("action.system.audio_input_control.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_INPUT_CONTROL
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemAudioOutputControl {
    input_device: String,
    vol_adjust: i8,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for SystemAudioOutputControl {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> String {
        "SystemAudioOutputControl".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.label(t!("action.system.audio_output_control.output_device"));
        ComboBox::from_id_salt("SystemAudioOutputControlDeviceSelect")
            .selected_text(self.input_device.clone())
            .width(228.0)
            .show_ui(ui, |_ui| {
                // TODO
            });

        ui.label(t!("action.system.audio_output_control.volume_adjust"));
        ui.add(Slider::new(&mut self.vol_adjust, -100..=100));
    }

    fn help(&self) -> String {
        t!("action.system.audio_output_control.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_OUTPUT_CONTROL
    }
}
