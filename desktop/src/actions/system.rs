use std::{process::Command, sync::Arc};

use eframe::egui::{include_image, ComboBox, ImageSource, Slider, Ui};
use egui_phosphor::regular as phos;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Action, ActionError};

pub const AID_SYSTEM_OPEN_APP: &str = "SystemOpenApp";
pub const AID_SYSTEM_OPEN_WEB: &str = "SystemOpenWeb";
pub const AID_SYSTEM_SND_IN_CTRL: &str = "SystemSndInCtrl";
pub const AID_SYSTEM_SND_OUT_CTRL: &str = "SystemSndOutCtrl";

const ICON_OPEN_APP: ImageSource =
    include_image!("../../../assets/action-icons/system-appopen.bmp");
const ICON_OPEN_WEB: ImageSource =
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
            (AID_SYSTEM_OPEN_APP.into(),     Box::new(SystemOpenApp::default()),    t!("action.system.open_app.title").into()),
            (AID_SYSTEM_OPEN_WEB.into(),     Box::new(SystemOpenWeb::default()),    t!("action.system.open_web.title").into()),
            // (AID_SYSTEM_SND_IN_CTRL.into(),  Box::new(SystemSndInCtrl::default()),  t!("action.system.snd_in_ctrl.title").into()),
            // (AID_SYSTEM_SND_OUT_CTRL.into(), Box::new(SystemSndOutCtrl::default()), t!("action.system.snd_out_ctrl.title").into()),
        ],
    )
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemOpenApp {
    filepath: String,
    arguments: Vec<String>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for SystemOpenApp {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
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
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn get_type(&self) -> String {
        AID_SYSTEM_OPEN_APP.into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        if ui
            .button(t!("action.system.open_app.choose_file"))
            .clicked()
        {
            if let Some(f) = FileDialog::new().pick_file() {
                self.filepath = f.to_str().unwrap().to_owned();
            }
        }
        ui.text_edit_singleline(&mut self.filepath);
        ui.horizontal(|ui| {
            ui.label(t!("action.system.open_app.add_arguments"));
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
        t!("action.system.open_app.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_OPEN_APP
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemOpenWeb {
    url: String,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for SystemOpenWeb {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        let _ = open::that(self.url.clone());
        // TODO: error handling
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn get_type(&self) -> String {
        AID_SYSTEM_OPEN_WEB.into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.label(t!("action.system.open_web.url"));
        ui.text_edit_singleline(&mut self.url);
    }

    fn help(&self) -> String {
        t!("action.system.open_web.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_OPEN_WEB
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemSndInCtrl {
    input_device: String,
    vol_adjust: i8,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for SystemSndInCtrl {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        // TODO
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn get_type(&self) -> String {
        AID_SYSTEM_SND_IN_CTRL.into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.label(t!("action.system.snd_in_ctrl.input_device"));
        ComboBox::from_id_salt("SystemAudioInputControlDeviceSelect")
            .selected_text(self.input_device.clone())
            .width(228.0)
            .show_ui(ui, |_ui| {
                // TODO
            });

        ui.label(t!("action.system.snd_in_ctrl.volume_adjust"));
        ui.add(Slider::new(&mut self.vol_adjust, -100..=100));
    }

    fn help(&self) -> String {
        t!("action.system.snd_in_ctrl.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_INPUT_CONTROL
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SystemSndOutCtrl {
    input_device: String,
    vol_adjust: i8,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for SystemSndOutCtrl {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        // TODO
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn get_type(&self) -> String {
        AID_SYSTEM_SND_OUT_CTRL.into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.label(t!("action.system.snd_out_ctrl.output_device"));
        ComboBox::from_id_salt("SystemAudioOutputControlDeviceSelect")
            .selected_text(self.input_device.clone())
            .width(228.0)
            .show_ui(ui, |_ui| {
                // TODO
            });

        ui.label(t!("action.system.snd_out_ctrl.volume_adjust"));
        ui.add(Slider::new(&mut self.vol_adjust, -100..=100));
    }

    fn help(&self) -> String {
        t!("action.system.snd_out_ctrl.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_OUTPUT_CONTROL
    }
}
