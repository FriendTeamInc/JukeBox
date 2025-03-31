// Graphical User Interface (pronounced like GIF)

use std::collections::{HashMap, HashSet};
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Duration, Instant};

use eframe::egui::{
    vec2, Align, CentralPanel, Context, Id, Layout, Modal, RichText, Ui, ViewportBuilder,
};
use eframe::Frame;
use egui_extras::install_image_loaders;
use egui_phosphor::regular as phos;
use jukebox_util::color::RgbProfile;
use jukebox_util::peripheral::DeviceType;
use rand::prelude::*;
use tokio::{
    runtime::Runtime,
    spawn,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};

use crate::actions::{
    action::action_task,
    meta::MetaNoAction,
    types::{Action, ActionMap},
};
use crate::config::{ActionIcon, DeviceConfig, JukeBoxConfig};
use crate::input::InputKey;
use crate::serial::{serial_task, SerialCommand, SerialEvent};
use crate::splash::SPLASH_MESSAGES;
use crate::update::UpdateStatus;

#[derive(PartialEq)]
pub enum GuiTab {
    Device,
    EditingAction,
    EditingRGB,
    EditingScreen,
    Settings,
    Updating,
}

pub struct JukeBoxGui {
    pub splash_timer: Instant,
    pub splash_index: usize,

    pub gui_tab: GuiTab,

    pub current_device: String,
    // Device UID -> (DeviceType, Device Nickname, Firmware Version, Connected?, Device Inputs)
    pub devices: HashMap<String, (DeviceType, String, String, bool, HashSet<InputKey>)>,

    pub config: Arc<Mutex<JukeBoxConfig>>,
    pub config_enable_splash: bool,
    pub config_always_save_on_exit: bool,

    pub profile_renaming: bool,
    pub profile_name_entry: String,

    pub device_renaming: bool,
    pub device_name_entry: String,

    pub editing_key: InputKey,
    pub editing_action_icon: ActionIcon,
    pub editing_action_type: String,
    pub editing_action: Box<dyn Action>,

    pub editing_rgb: RgbProfile,

    pub exit_save_modal: bool,

    pub update_progress: f32,
    pub update_status: UpdateStatus,

    pub thread_breaker: Arc<AtomicBool>,
    pub sg_rx: UnboundedReceiver<SerialEvent>,
    pub scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
    pub us_tx: UnboundedSender<UpdateStatus>,
    pub us_rx: UnboundedReceiver<UpdateStatus>,

    pub action_map: ActionMap,
}
impl eframe::App for JukeBoxGui {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // TODO: give ctx to other threads?, so ui can be updated as necessary.
        // but only once

        // TODO: move this to App::tick when available
        // https://github.com/emilk/egui/issues/5113
        self.handle_serial_events();

        if ctx.input(|i| i.viewport().close_requested()) {
            // TODO: handle this as going to system tray

            for (_k, tx) in self.scmd_txs.blocking_lock().iter() {
                let _ = tx.send(SerialCommand::Disconnect);
                // .expect(&format!("could not send disconnect signal to device {}", k));
            }

            self.thread_breaker
                .store(true, std::sync::atomic::Ordering::Relaxed);

            return;
        }

        CentralPanel::default().show(ctx, |ui| self.ui(ui));

        // Call a new frame every frame, bypassing the limited updates.
        // NOTE: This is a bad idea, we should probably change this later
        // and only update the window as necessary.
        ctx.request_repaint();
    }
}
impl JukeBoxGui {
    fn new() -> Self {
        let config = JukeBoxConfig::load();
        config.save(); // Immediately save, in case the config was the loaded default
        let devices: HashMap<String, (DeviceType, String, String, bool, HashSet<InputKey>)> =
            config
                .devices
                .clone()
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        (v.0, v.1.clone(), "?".into(), false, HashSet::new()),
                    )
                })
                .collect();
        let current_device = devices.keys().next().unwrap_or(&String::new()).into();
        let config_enable_splash = config.enable_splash;
        let config_always_save_on_exit = config.always_save_on_exit;
        let config = Arc::new(Mutex::new(JukeBoxConfig::load()));

        // when gui exits, we use these to signal the other threads to stop
        let thread_breaker = Arc::new(AtomicBool::new(false)); // ends other threads from gui
        let brkr_serial = thread_breaker.clone();

        let (sr_tx, sr_rx) = unbounded_channel::<SerialEvent>(); // serial threads send events to action thread
        let (sg_tx, sg_rx) = unbounded_channel::<SerialEvent>(); // serial threads send events to gui thread

        let scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // gui thread sends events to serial threads (specific Device ID -> Device specific Serial Thread)
        // serial thread spawns the channels and gives the sender to the gui thread through this

        let (us_tx, us_rx) = unbounded_channel::<UpdateStatus>(); // update thread sends update statuses to gui thread

        let serial_scmd_txs = scmd_txs.clone();
        let action_config = config.clone();
        let action_scmd_txs = scmd_txs.clone();

        spawn(async move { serial_task(brkr_serial, serial_scmd_txs, sg_tx, sr_tx).await });
        spawn(async move { action_task(sr_rx, action_config, action_scmd_txs).await });

        JukeBoxGui {
            splash_timer: Instant::now(),
            splash_index: 0usize,

            gui_tab: GuiTab::Device,

            current_device: current_device,
            devices: devices,

            config: config,
            config_enable_splash: config_enable_splash,
            config_always_save_on_exit: config_always_save_on_exit,

            profile_renaming: false,
            profile_name_entry: String::new(),

            device_renaming: false,
            device_name_entry: String::new(),

            editing_key: InputKey::UnknownKey,
            editing_action_icon: ActionIcon::DefaultActionIcon,
            editing_action_type: "MetaNoAction".into(),
            editing_action: Box::new(MetaNoAction::default()),

            editing_rgb: RgbProfile::Off,

            exit_save_modal: false,

            update_progress: 0.0,
            update_status: UpdateStatus::Start,

            thread_breaker: thread_breaker,
            sg_rx: sg_rx,
            scmd_txs: scmd_txs,
            us_tx: us_tx,
            us_rx: us_rx,

            action_map: ActionMap::new(),
        }
    }

    fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            self.draw_back_button(ui);
            self.draw_profile_management(ui);
            self.draw_settings_toggle(ui);
        });

        ui.separator();

        ui.allocate_ui(vec2(464.0, 245.0), |ui| match self.gui_tab {
            GuiTab::Device => self.draw_device_page(ui),
            GuiTab::Settings => self.draw_settings_page(ui),
            GuiTab::EditingAction => self.draw_edit_action(ui),
            GuiTab::EditingRGB => self.draw_edit_rgb(ui),
            GuiTab::EditingScreen => todo!(),
            GuiTab::Updating => self.draw_update_page(ui),
        });

        ui.separator();

        ui.columns_const(|[c1, c2]| {
            c1.with_layout(Layout::left_to_right(Align::BOTTOM), |ui| {
                self.draw_device_management(ui);
            });

            self.draw_splash_text(c2);
        });
    }

    fn handle_serial_events(&mut self) {
        while let Ok(event) = self.sg_rx.try_recv() {
            match event {
                SerialEvent::Connected { device_info } => {
                    // TODO: We shouldn't really keep this in the gui thread.
                    let device_uid = device_info.device_uid;
                    let device_type = device_info.input_identifier;
                    let firmware_version = device_info.firmware_version;

                    let short_uid = device_uid[..4].to_string();

                    // TODO: double check that the device is fine to use
                    let device_name: String = match Into::<DeviceType>::into(device_type) {
                        DeviceType::Unknown => t!("device_name.unknown", uid = device_uid.clone()),
                        DeviceType::KeyPad => t!("device_name.keypad", uid = short_uid),
                        DeviceType::KnobPad => t!("device_name.knobpad", uid = short_uid),
                        DeviceType::PedalPad => t!("device_name.pedalpad", uid = short_uid),
                    }
                    .to_string();

                    {
                        let mut conf = self.config.blocking_lock();
                        if !conf.devices.contains_key(&device_uid) {
                            conf.devices.insert(
                                device_uid.clone(),
                                (device_type.into(), device_name.clone()),
                            );
                            for (_, v) in conf.profiles.iter_mut() {
                                if !v.contains_key(&device_uid) {
                                    v.insert(
                                        device_uid.clone(),
                                        DeviceConfig {
                                            key_map: self
                                                .action_map
                                                .default_action_config(device_type.into()),
                                            rgb_profile: None,
                                        },
                                    );
                                }
                            }
                        }
                        conf.save();
                    }

                    if self.current_device.is_empty() || self.devices.iter().all(|(_, d)| !d.3) {
                        self.current_device = device_uid.clone();
                    }

                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.0 = device_type.into();
                        // v.1 = device_name;
                        v.2 = firmware_version;
                        v.3 = true;
                        v.4.clear();
                    } else {
                        self.devices.insert(
                            device_uid.clone(),
                            (
                                device_type.into(),
                                device_name,
                                firmware_version,
                                true,
                                HashSet::new(),
                            ),
                        );
                    }
                }
                SerialEvent::LostConnection { device_uid } => {
                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.3 = false;
                        v.4.clear();
                    }
                    let mut scmd_txs = self.scmd_txs.blocking_lock();
                    scmd_txs.remove(&device_uid);
                }
                SerialEvent::Disconnected { device_uid } => {
                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.3 = false;
                        v.4.clear();
                    }
                    let mut scmd_txs = self.scmd_txs.blocking_lock();
                    scmd_txs.remove(&device_uid);
                }
                SerialEvent::GetInputKeys { device_uid, keys } => {
                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.4 = keys;
                    }
                }
            }
        }
    }

    fn draw_back_button(&mut self, ui: &mut Ui) {
        // back button
        ui.add_enabled_ui(
            self.gui_tab != GuiTab::Device
                && (self.update_status == UpdateStatus::Start
                    || self.update_status == UpdateStatus::End),
            |ui| {
                let saveable = match self.gui_tab {
                    GuiTab::EditingAction | GuiTab::EditingRGB | GuiTab::EditingScreen => true,
                    _ => false,
                };
                if ui
                    .button(RichText::new(phos::ARROW_BEND_UP_LEFT))
                    .on_hover_text_at_pointer(match saveable {
                        true => t!("help.back.save_button"),
                        false => t!("help.back.button"),
                    })
                    .clicked()
                {
                    if saveable {
                        if self.config_always_save_on_exit {
                            match self.gui_tab {
                                GuiTab::EditingAction => self.save_action_and_exit(),
                                GuiTab::EditingRGB => self.save_rgb_and_exit(),
                                // GuiTab::EditingScreen => self.save_screen_and_exit(),
                                _ => self.gui_tab = GuiTab::Device,
                            }
                        } else {
                            self.exit_save_modal = true;
                        }
                    } else {
                        self.gui_tab = GuiTab::Device;
                    }
                }
            },
        );

        if self.exit_save_modal {
            Modal::new(Id::new("ExitSaveModal")).show(ui.ctx(), |ui| {
                ui.set_width(200.0);
                ui.heading(t!("help.back.modal_title"));

                ui.add_space(32.0);

                ui.horizontal(|ui| {
                    if ui.button(t!("help.back.modal_save")).clicked() {
                        self.exit_save_modal = false;
                        match self.gui_tab {
                            GuiTab::EditingAction => self.save_action_and_exit(),
                            GuiTab::EditingRGB => self.save_rgb_and_exit(),
                            // GuiTab::EditingScreen => self.save_screen_and_exit(),
                            _ => (),
                        }
                        self.gui_tab = GuiTab::Device;
                    }
                    if ui.button(t!("help.back.modal_exit")).clicked() {
                        self.exit_save_modal = false;
                        self.gui_tab = GuiTab::Device;
                    }
                    if ui.button(t!("help.back.modal_cancel")).clicked() {
                        self.exit_save_modal = false;
                    }
                });
            });
        }
    }

    fn draw_splash_text(&mut self, ui: &mut Ui) {
        if Instant::now() > self.splash_timer {
            loop {
                let new_index = rand::rng().random_range(0..SPLASH_MESSAGES.len());
                if new_index != self.splash_index {
                    self.splash_index = new_index;
                    break;
                }
            }
            self.splash_timer = Instant::now() + Duration::from_secs(30);
        }
        // TODO: display error message from key here if relevant
        if self.config_enable_splash {
            ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                ui.label(
                    // splash text will remain untranslated for the foreseeable future
                    RichText::new(SPLASH_MESSAGES[self.splash_index])
                        .monospace()
                        .size(6.0),
                );
            });
        }
    }
}

pub fn basic_gui() {
    let rt = Runtime::new().expect("unable to create tokio runtime");
    let _guard = rt.enter();

    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(t!("window_title"))
            .with_inner_size([960.0, 640.0])
            .with_maximize_button(false)
            .with_resizable(false)
            .with_icon(
                eframe::icon_data::from_png_bytes(
                    &include_bytes!("../../../assets/applogo.png")[..],
                )
                .unwrap(),
            ),
        centered: true,
        ..Default::default()
    };

    // TODO: error handle this
    let _ = eframe::run_native(
        "JukeBoxDesktop",
        native_options,
        Box::new(|cc| {
            let ctx = &cc.egui_ctx;
            ctx.set_zoom_factor(2.0);
            let mut fonts = eframe::egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            ctx.set_fonts(fonts);
            install_image_loaders(ctx);

            Ok(Box::new(JukeBoxGui::new()))
        }),
    );

    rt.shutdown_timeout(Duration::from_secs(1));
}
