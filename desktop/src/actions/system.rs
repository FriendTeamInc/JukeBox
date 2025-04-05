use std::sync::OnceLock;
use std::{process::Command, sync::Arc};

use eframe::egui::{include_image, ComboBox, ImageSource, Slider, TextWrapMode, Ui};
use egui_phosphor::regular as phos;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::{sync::Mutex, task::spawn_blocking};

use crate::single_fire;
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

enum AudioCommand {
    GetInputDevices,
    GetOutputDevices,
    AdjustInputDevice(String, i8),
    AdjustOutputDevice(String, i8),
}

#[cfg(target_os = "linux")]
use pactl::controllers::{types::DeviceInfo, DeviceControl, SinkController, SourceController};
#[cfg(target_os = "windows")]
use windows::Win32::{
    Devices::FunctionDiscovery::*,
    Media::{
        Audio::{Endpoints::IAudioEndpointVolume, *},
        KernelStreaming::GUID_NULL,
    },
    System::{Com::*, Variant::VT_EMPTY},
};

static SYSTEM_AUDIO_CMD_TX: OnceLock<UnboundedSender<AudioCommand>> = OnceLock::new();
static SYSTEM_SOURCES: OnceLock<Mutex<Option<Vec<String>>>> = OnceLock::new();
static SYSTEM_SINKS: OnceLock<Mutex<Option<Vec<String>>>> = OnceLock::new();

#[cfg(target_os = "linux")]
fn get_devices(devices: Vec<DeviceInfo>) -> Vec<String> {
    let mut d = Vec::new();

    for device in devices {
        d.push(device.description.unwrap());
    }

    d
}

#[cfg(target_os = "windows")]
fn get_devices(dir: EDataFlow) -> IMMDeviceCollection {
    unsafe {
        let device_enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();

        device_enumerator
            .EnumAudioEndpoints(dir, DEVICE_STATE_ACTIVE)
            .unwrap()
    }
}

#[cfg(target_os = "windows")]
fn get_device_names(dir: EDataFlow) -> Vec<String> {
    unsafe {
        let devices = get_devices(dir);

        let mut output = Vec::new();
        let count = devices.GetCount().unwrap();
        for i in 0..count {
            let item = devices.Item(i).unwrap();

            let properties = item.OpenPropertyStore(STGM_READ).unwrap();
            let friendly_name = properties.GetValue(&PKEY_Device_FriendlyName).unwrap();
            assert_ne!(friendly_name.vt(), VT_EMPTY);
            let name = friendly_name
                .Anonymous
                .Anonymous
                .Anonymous
                .pwszVal
                .to_string()
                .unwrap();

            // let endpoint: IAudioEndpointVolume = item.Activate(CLSCTX_ALL, None)?;

            output.push(name);
        }

        output
    }
}

#[cfg(target_os = "windows")]
fn adjust_device_volume(dir: EDataFlow, device_name: String, adjust: i8) {
    unsafe {
        let devices = get_devices(dir);

        let count = devices.GetCount().unwrap();
        for i in 0..count {
            let item = devices.Item(i).unwrap();

            let properties = item.OpenPropertyStore(STGM_READ).unwrap();
            let friendly_name = properties.GetValue(&PKEY_Device_FriendlyName).unwrap();
            assert_ne!(friendly_name.vt(), VT_EMPTY);
            let name = friendly_name
                .Anonymous
                .Anonymous
                .Anonymous
                .pwszVal
                .to_string()
                .unwrap();

            if name == device_name {
                let endpoint: IAudioEndpointVolume = item.Activate(CLSCTX_ALL, None).unwrap();

                let current_volume = endpoint.GetMasterVolumeLevelScalar().unwrap();
                let new_volume = current_volume + (adjust as f32) / 100.0;
                endpoint
                    .SetMasterVolumeLevelScalar(new_volume, &GUID_NULL)
                    .unwrap();

                break;
            }
        }
    }
}

fn system_audio_control_loop(mut cmd_rx: UnboundedReceiver<AudioCommand>) {
    #[cfg(target_os = "linux")]
    let (mut source_controller, mut sink_controller) = {
        (
            SourceController::create().expect("failed to create source controller"),
            SinkController::create().expect("failed to create sink controller"),
        )
    };

    #[cfg(target_os = "windows")]
    unsafe {
        let _ = CoInitializeEx(None, COINIT_SPEED_OVER_MEMORY);
    }

    while let Some(cmd) = cmd_rx.blocking_recv() {
        match cmd {
            AudioCommand::GetInputDevices => {
                #[cfg(target_os = "linux")]
                let devices = get_devices(
                    source_controller
                        .list_devices()
                        .expect("failed to get list of source devices"),
                );
                #[cfg(target_os = "windows")]
                let devices = get_device_names(eCapture);

                let mut system_sources = SYSTEM_SOURCES.get().unwrap().blocking_lock();
                *system_sources = Some(devices);
            }
            AudioCommand::GetOutputDevices => {
                #[cfg(target_os = "linux")]
                let devices = get_devices(
                    sink_controller
                        .list_devices()
                        .expect("failed to get list of sink devices"),
                );
                #[cfg(target_os = "windows")]
                let devices = get_device_names(eRender);

                let mut system_sinks = SYSTEM_SINKS.get().unwrap().blocking_lock();
                *system_sinks = Some(devices);
            }
            AudioCommand::AdjustInputDevice(source, adjust) => {
                #[cfg(target_os = "linux")]
                {
                    let sources = source_controller
                        .list_devices()
                        .expect("failed to get list of source devices");

                    for s in sources {
                        if s.description.unwrap_or_default() == *source {
                            let vol = (adjust as f64) / 100.0;
                            if vol > 0.0 {
                                source_controller.increase_device_volume_by_percent(s.index, vol);
                            } else if vol < 0.0 {
                                source_controller.decrease_device_volume_by_percent(s.index, -vol);
                            }
                            break;
                        }
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    adjust_device_volume(eCapture, source, adjust);
                }
            }
            AudioCommand::AdjustOutputDevice(sink, adjust) => {
                #[cfg(target_os = "linux")]
                {
                    let sinks = sink_controller
                        .list_devices()
                        .expect("failed to get list of sink devices");

                    for s in sinks {
                        if s.description.unwrap_or_default() == *sink {
                            let vol = (adjust as f64) / 100.0;
                            if vol > 0.0 {
                                sink_controller.increase_device_volume_by_percent(s.index, vol);
                            } else if vol < 0.0 {
                                sink_controller.decrease_device_volume_by_percent(s.index, -vol);
                            }
                            break;
                        }
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    adjust_device_volume(eRender, sink, adjust);
                }
            }
        }
    }
}

#[rustfmt::skip]
pub fn system_action_list() -> (String, Vec<(String, Box<dyn Action>, String)>) {
    let (cmd_tx, cmd_rx) = unbounded_channel();
    SYSTEM_AUDIO_CMD_TX.get_or_init(|| cmd_tx);
    SYSTEM_SOURCES.get_or_init(|| Mutex::new(None));
    SYSTEM_SINKS.get_or_init(|| Mutex::new(None));

    tokio::spawn(async move {
        spawn_blocking(move || {
            system_audio_control_loop(cmd_rx);
        });
    });

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
        // TODO: error handling
        if let Some(input_device) = self.input_device.clone() {
            let adjust = self.vol_adjust;
            let _ = SYSTEM_AUDIO_CMD_TX
                .get()
                .unwrap()
                .send(AudioCommand::AdjustInputDevice(input_device, adjust));
        }
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

        single_fire!(ComboBox::is_open(ui.ctx(), ir.response.id), {
            *SYSTEM_SOURCES.get().unwrap().blocking_lock() = None;
            let _ = SYSTEM_AUDIO_CMD_TX
                .get()
                .unwrap()
                .send(AudioCommand::GetInputDevices);
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
        // TODO: error handling
        if let Some(output_device) = self.output_device.clone() {
            let adjust = self.vol_adjust;
            let _ = SYSTEM_AUDIO_CMD_TX
                .get()
                .unwrap()
                .send(AudioCommand::AdjustOutputDevice(output_device, adjust));
        }
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

        single_fire!(ComboBox::is_open(ui.ctx(), ir.response.id), {
            *SYSTEM_SINKS.get().unwrap().blocking_lock() = None;
            let _ = SYSTEM_AUDIO_CMD_TX
                .get()
                .unwrap()
                .send(AudioCommand::GetOutputDevices);
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
