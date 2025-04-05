use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::{process::Command, sync::Arc};

use eframe::egui::{include_image, ComboBox, ImageSource, Slider, TextWrapMode, Ui};
use egui_phosphor::regular as phos;
use pactl::controllers::DeviceControl;
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

#[cfg(target_os = "linux")]
use pactl::controllers::SinkController;
#[cfg(target_os = "linux")]
use pactl::controllers::SourceController;
#[cfg(target_os = "linux")]
static mut SOURCE_CONTROLLER: Mutex<Option<SourceController>> = Mutex::const_new(None);
#[cfg(target_os = "linux")]
static mut SINK_CONTROLLER: Mutex<Option<SinkController>> = Mutex::const_new(None);

static SYSTEM_SOURCES: OnceLock<Mutex<Option<Vec<String>>>> = OnceLock::new();
static SYSTEM_GET_SOURCES: OnceLock<AtomicBool> = OnceLock::new();
static SYSTEM_SINKS: OnceLock<Mutex<Option<Vec<String>>>> = OnceLock::new();
static SYSTEM_GET_SINKS: OnceLock<AtomicBool> = OnceLock::new();

#[rustfmt::skip]
pub fn system_action_list() -> (String, Vec<(String, Box<dyn Action>, String)>) {
    SYSTEM_GET_SOURCES.get_or_init(|| true.into());
    SYSTEM_SOURCES.get_or_init(|| Mutex::new(None));
    SYSTEM_GET_SINKS.get_or_init(|| true.into());
    SYSTEM_SINKS.get_or_init(|| Mutex::new(None));

    (
        t!("action.system.title", icon = phos::DESKTOP_TOWER).into(),
        vec![
            (AID_SYSTEM_OPEN_APP.into(),     Box::new(SystemOpenApp::default()),    t!("action.system.open_app.title").into()),
            (AID_SYSTEM_OPEN_WEB.into(),     Box::new(SystemOpenWeb::default()),    t!("action.system.open_web.title").into()),
            (AID_SYSTEM_SND_IN_CTRL.into(),  Box::new(SystemSndInCtrl::default()),  t!("action.system.snd_in_ctrl.title").into()),
            (AID_SYSTEM_SND_OUT_CTRL.into(), Box::new(SystemSndOutCtrl::default()), t!("action.system.snd_out_ctrl.title").into()),
        ],
    )
}

fn list_sources() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        #[allow(static_mut_refs)]
        let mut source_controller = unsafe { SOURCE_CONTROLLER.blocking_lock() };
        if source_controller.is_none() {
            *source_controller =
                Some(SourceController::create().expect("failed to create source controller"));
        }

        let mut devices = Vec::new();
        if let Some(handler) = source_controller.as_mut() {
            let sinks = handler
                .list_devices()
                .expect("failed to get list of source devices");

            for s in sinks {
                devices.push(s.description.unwrap_or_default());
            }
        }

        devices
    }
    #[cfg(target_os = "windows")]
    {
        Vec::new()
    }
}

fn list_sinks() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        #[allow(static_mut_refs)]
        let mut sink_controller = unsafe { SINK_CONTROLLER.blocking_lock() };
        if sink_controller.is_none() {
            *sink_controller =
                Some(SinkController::create().expect("failed to create sink controller"));
        }

        let mut devices = Vec::new();
        if let Some(handler) = sink_controller.as_mut() {
            let sinks = handler
                .list_devices()
                .expect("failed to get list of sink devices");

            for s in sinks {
                devices.push(s.description.unwrap_or_default());
            }
        }

        devices
    }
    #[cfg(target_os = "windows")]
    {
        Vec::new()
    }
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
    input_device: Option<String>,
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
        let ir = ComboBox::from_id_salt("SystemAudioInputControlDeviceSelect")
            .selected_text(self.input_device.clone().unwrap_or_default())
            .width(200.0)
            .wrap_mode(TextWrapMode::Truncate)
            .show_ui(ui, |ui| {
                let sources = SYSTEM_SOURCES.get().unwrap().blocking_lock();
                if let Some(sources) = &*sources {
                    for source in sources {
                        let selected = *source == self.input_device.clone().unwrap_or_default();
                        let l = ui.selectable_label(selected, source);
                        if l.clicked() {
                            self.input_device = Some(source.clone());
                        }
                    }
                } else {
                    ui.label(t!("action.system.snd_in_ctrl.loading"));
                }
            });

        if ComboBox::is_open(ui.ctx(), ir.response.id) {
            if SYSTEM_GET_SOURCES.get().unwrap().load(Ordering::Relaxed) {
                *SYSTEM_SOURCES.get().unwrap().blocking_lock() = None;
                tokio::spawn(async move {
                    spawn_blocking(move || {
                        *SYSTEM_SOURCES.get().unwrap().blocking_lock() = Some(list_sources());
                    });
                });
            }
            let _ = SYSTEM_GET_SOURCES.set(false.into());
        } else {
            let _ = SYSTEM_GET_SOURCES.set(true.into());
        }

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
    output_device: Option<String>,
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
        let ir = ComboBox::from_id_salt("SystemAudioOutputControlDeviceSelect")
            .selected_text(self.output_device.clone().unwrap_or_default())
            .width(200.0)
            .wrap_mode(TextWrapMode::Truncate)
            .show_ui(ui, |ui| {
                let sinks = SYSTEM_SINKS.get().unwrap().blocking_lock();
                if let Some(sinks) = &*sinks {
                    for sink in sinks {
                        let selected = *sink == self.output_device.clone().unwrap_or_default();
                        let l = ui.selectable_label(selected, sink);
                        if l.clicked() {
                            self.output_device = Some(sink.clone());
                        }
                    }
                } else {
                    ui.label(t!("action.system.snd_out_ctrl.loading"));
                }
            });

        if ComboBox::is_open(ui.ctx(), ir.response.id) {
            if SYSTEM_GET_SINKS.get().unwrap().load(Ordering::Relaxed) {
                *SYSTEM_SINKS.get().unwrap().blocking_lock() = None;
                tokio::spawn(async move {
                    spawn_blocking(move || {
                        *SYSTEM_SINKS.get().unwrap().blocking_lock() = Some(list_sinks());
                    });
                });
            }
            let _ = SYSTEM_GET_SINKS.set(false.into());
        } else {
            let _ = SYSTEM_GET_SINKS.set(true.into());
        }

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
