// Graphical User Interface (pronounced like GIF)

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Duration, Instant};

use eframe::egui::{
    vec2, Align, CentralPanel, Context, Id, Layout, Modal, RichText, Ui, UserAttentionType,
    ViewportBuilder, ViewportCommand,
};
use eframe::Frame;
use egui_extras::install_image_loaders;
use egui_phosphor::regular as phos;
use jukebox_util::peripheral::DeviceType;
use jukebox_util::rgb::RgbProfile;
use jukebox_util::screen::ScreenProfile;
use jukebox_util::stats::SystemStats;
use rand::prelude::*;
use tokio::task::spawn_blocking;
use tokio::{
    runtime::Runtime,
    spawn,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::actions::types::ActionError;
use crate::actions::{
    action::action_task,
    meta::MetaNoAction,
    types::{Action, ActionMap},
};
use crate::config::{ActionIcon, DeviceConfig, DeviceInfo, JukeBoxConfig};
use crate::input::InputKey;
use crate::serial::{serial_task, SerialCommand, SerialEvent};
use crate::splash::SPLASH_MESSAGES;
use crate::system::system_task;
use crate::update::UpdateStatus;

const APP_ICON: &[u8] = include_bytes!("../../../assets/applogo.png");
static CLOSE_WINDOW: OnceLock<Mutex<(bool, bool)>> = OnceLock::new();
static SHOW_WINDOW_ID: OnceLock<Mutex<MenuId>> = OnceLock::new();
static QUIT_WINDOW_ID: OnceLock<Mutex<MenuId>> = OnceLock::new();

#[derive(PartialEq)]
pub enum GuiTab {
    Device,
    EditingAction,
    EditingRGB,
    EditingScreen,
    Settings,
    Updating,
}

pub struct DeviceInfoExt {
    pub device_info: DeviceInfo,
    pub firmware_version: String,
    pub connected: bool,
    pub device_inputs: HashSet<InputKey>,
}

pub struct JukeBoxGui {
    pub splash_timer: Instant,
    pub splash_index: usize,

    pub gui_tab: GuiTab,

    pub current_device: String,
    // Device UID -> (DeviceType, Device Nickname, Firmware Version, Connected?, Device Inputs)
    pub devices: HashMap<String, DeviceInfoExt>,

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

    pub editing_screen: ScreenProfile,

    pub exit_save_modal: bool,

    pub update_progress: f32,
    pub update_status: UpdateStatus,

    pub thread_breaker: Arc<AtomicBool>,
    pub sg_rx: UnboundedReceiver<SerialEvent>,
    pub scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
    pub us_tx: UnboundedSender<UpdateStatus>,
    pub us_rx: UnboundedReceiver<UpdateStatus>,
    pub ae_rx: UnboundedReceiver<ActionError>,

    pub action_map: ActionMap,

    pub action_errors: VecDeque<ActionError>,
}
impl eframe::App for JukeBoxGui {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // TODO: give ctx to other threads?, so ui can be updated as necessary.
        // but only once

        // TODO: move this to App::tick when available
        // https://github.com/emilk/egui/issues/5113
        self.handle_serial_events(ctx);

        CentralPanel::default().show(ctx, |ui| self.ui(ui));

        let mut close_window = CLOSE_WINDOW.get().unwrap().blocking_lock();

        while let Ok(_) = TrayIconEvent::receiver().try_recv() {}
        let show_window_id = SHOW_WINDOW_ID.get().unwrap().blocking_lock();
        let quit_window_id = QUIT_WINDOW_ID.get().unwrap().blocking_lock();
        while let Ok(e) = MenuEvent::receiver().try_recv() {
            if e.id == *show_window_id {
                close_window.0 = false;
            } else if e.id == *quit_window_id {
                close_window.1 = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            if close_window.1 {
                for (_k, tx) in self.scmd_txs.blocking_lock().iter() {
                    let _ = tx.send(SerialCommand::Disconnect);
                    // .expect(&format!("could not send disconnect signal to device {}", k));
                }

                self.thread_breaker
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                return;
            } else {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                close_window.0 = true;
            }
        }

        ctx.send_viewport_cmd(ViewportCommand::Visible(!close_window.0));

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
        let devices: HashMap<String, DeviceInfoExt> = config
            .devices
            .clone()
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    DeviceInfoExt {
                        device_info: DeviceInfo {
                            device_type: v.device_type,
                            nickname: v.nickname.clone(),
                        },
                        firmware_version: "?".into(),
                        connected: false,
                        device_inputs: HashSet::new(),
                    },
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
        let (ae_tx, ae_rx) = unbounded_channel::<ActionError>(); // action thread sends action errors to gui thread

        let serial_scmd_txs = scmd_txs.clone();
        let action_config = config.clone();
        let action_scmd_txs = scmd_txs.clone();

        let system_stats: Arc<Mutex<SystemStats>> = Arc::new(Mutex::new(SystemStats::default()));
        let serial_ss = system_stats.clone();

        spawn(
            async move { serial_task(brkr_serial, serial_scmd_txs, sg_tx, sr_tx, serial_ss).await },
        );
        spawn(async move { action_task(sr_rx, action_config, action_scmd_txs, ae_tx).await });
        spawn(async move { spawn_blocking(|| system_task(system_stats)) });

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

            editing_rgb: RgbProfile::default_gui_profile(),

            editing_screen: ScreenProfile::default_profile(),

            exit_save_modal: false,

            update_progress: 0.0,
            update_status: UpdateStatus::Start,

            thread_breaker: thread_breaker,
            sg_rx: sg_rx,
            scmd_txs: scmd_txs,
            us_tx: us_tx,
            us_rx: us_rx,
            ae_rx: ae_rx,

            action_map: ActionMap::new(),

            action_errors: VecDeque::new(),
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
            GuiTab::EditingScreen => self.draw_edit_screen(ui),
            GuiTab::Updating => self.draw_update_page(ui),
        });

        ui.separator();

        ui.columns_const(|[c1, c2]| {
            c1.with_layout(Layout::left_to_right(Align::BOTTOM), |ui| {
                self.draw_device_management(ui);
            });

            self.draw_splash_text(c2);
        });

        if self.action_errors.len() > 0 {
            let action = self.action_errors.get(0).unwrap().clone();
            Modal::new(Id::new("ActionErrorModal")).show(ui.ctx(), |ui| {
                ui.set_width(400.0);

                ui.heading(t!("help.action.modal_title"));

                ui.add_space(10.0);

                ui.label(action.msg);

                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    if ui.button(t!("help.action.modal_exit")).clicked() {
                        self.action_errors.pop_front();
                    }

                    ui.add_space(10.0);

                    ui.label(t!(
                        "help.action.modal_input_key",
                        input_key = action.input_key
                    ));

                    ui.add_space(10.0);

                    let device = if let Some(d) = self.devices.get(&action.device_uid) {
                        &d.device_info.nickname
                    } else {
                        &action.device_uid
                    };
                    ui.label(t!("help.action.modal_device", device = device));
                });
            });
        }
    }

    fn handle_serial_events(&mut self, ctx: &Context) {
        // recieve action error events
        let mut bring_back_up = false;
        while let Ok(error) = self.ae_rx.try_recv() {
            self.action_errors.push_back(error);
            bring_back_up = true;
        }
        if bring_back_up {
            let mut close_window = CLOSE_WINDOW.get().unwrap().blocking_lock();
            close_window.0 = false;
            ctx.send_viewport_cmd(ViewportCommand::RequestUserAttention(
                UserAttentionType::Informational,
            ));
        }

        // recieve serial events
        while let Ok(event) = self.sg_rx.try_recv() {
            match event {
                SerialEvent::Connected { device_info } => {
                    // TODO: We shouldn't really keep this in the gui thread.
                    let device_uid = device_info.device_uid;
                    let device_type: DeviceType = device_info.input_identifier.into();
                    let firmware_version = device_info.firmware_version;

                    let short_uid = device_uid[..4].to_string();

                    // TODO: double check that the device name is fine to use
                    let device_name: String = match device_type {
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
                                DeviceInfo {
                                    device_type: device_type,
                                    nickname: device_name.clone(),
                                },
                            );

                            let rgb_profile = match device_type {
                                DeviceType::KeyPad => Some(RgbProfile::default_gui_profile()),
                                _ => None,
                            };
                            let screen_profile = match device_type {
                                DeviceType::KeyPad => Some(ScreenProfile::default_profile()),
                                _ => None,
                            };

                            for (_, v) in conf.profiles.iter_mut() {
                                if !v.contains_key(&device_uid) {
                                    v.insert(
                                        device_uid.clone(),
                                        DeviceConfig {
                                            key_map: self
                                                .action_map
                                                .default_action_config(device_type.into()),
                                            rgb_profile: rgb_profile.clone(),
                                            screen_profile: screen_profile.clone(),
                                        },
                                    );
                                }
                            }
                        }
                        conf.save();
                    }

                    if self.current_device.is_empty()
                        || self.devices.iter().all(|(_, d)| !d.connected)
                    {
                        self.current_device = device_uid.clone();
                    }

                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.device_info.device_type = device_type.into();
                        // v.device_info.nickname = device_name;
                        v.firmware_version = firmware_version;
                        v.connected = true;
                        v.device_inputs.clear();
                    } else {
                        self.devices.insert(
                            device_uid.clone(),
                            DeviceInfoExt {
                                device_info: DeviceInfo {
                                    device_type: device_type.into(),
                                    nickname: device_name,
                                },
                                firmware_version: firmware_version,
                                connected: true,
                                device_inputs: HashSet::new(),
                            },
                        );
                    }
                }
                SerialEvent::LostConnection { device_uid } => {
                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.connected = false;
                        v.device_inputs.clear();
                    }
                    let mut scmd_txs = self.scmd_txs.blocking_lock();
                    scmd_txs.remove(&device_uid);
                }
                SerialEvent::Disconnected { device_uid } => {
                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.connected = false;
                        v.device_inputs.clear();
                    }
                    let mut scmd_txs = self.scmd_txs.blocking_lock();
                    scmd_txs.remove(&device_uid);
                }
                SerialEvent::GetInputKeys { device_uid, keys } => {
                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.device_inputs = keys;
                    }
                }
            }
        }
    }

    fn save_edit(&mut self) {
        match self.gui_tab {
            GuiTab::EditingAction => self.save_action_and_exit(),
            GuiTab::EditingRGB => self.save_rgb_and_exit(),
            GuiTab::EditingScreen => self.save_screen_and_exit(),
            _ => (),
        }
    }

    fn draw_back_button(&mut self, ui: &mut Ui) {
        // back button
        let enabled = self.gui_tab != GuiTab::Device
            && (self.update_status == UpdateStatus::Start
                || self.update_status == UpdateStatus::End);
        ui.add_enabled_ui(enabled, |ui| {
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
                        self.save_edit();
                        self.gui_tab = GuiTab::Device;
                    } else {
                        self.exit_save_modal = true;
                    }
                } else {
                    self.gui_tab = GuiTab::Device;
                }
            }
        });

        if self.exit_save_modal {
            Modal::new(Id::new("ExitSaveModal")).show(ui.ctx(), |ui| {
                ui.set_width(200.0);
                ui.heading(t!("help.back.modal_title"));

                ui.add_space(32.0);

                ui.horizontal(|ui| {
                    if ui.button(t!("help.back.modal_save")).clicked() {
                        self.exit_save_modal = false;
                        self.save_edit();
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
            .with_icon(eframe::icon_data::from_png_bytes(&APP_ICON[..]).unwrap()),
        centered: true,
        ..Default::default()
    };

    let _ = CLOSE_WINDOW.set(Mutex::new((false, false)));

    #[cfg(target_os = "linux")]
    std::thread::spawn(|| {
        gtk::init().unwrap();
        let _tray_icon = build_tray_icon();
        gtk::main();
    });

    #[cfg(not(target_os = "linux"))]
    let mut _tray_icon = std::rc::Rc::new(std::cell::RefCell::new(None));
    #[cfg(not(target_os = "linux"))]
    let tray_c = _tray_icon.clone();

    // TODO: error handle this
    let _ = eframe::run_native(
        "JukeBoxDesktop",
        native_options,
        Box::new(|cc| {
            #[cfg(not(target_os = "linux"))]
            tray_c.borrow_mut().replace(build_tray_icon());

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

fn build_tray_icon() -> TrayIcon {
    let icon = {
        let image = image::load_from_memory(&APP_ICON[..])
            .expect("failed to parse app icon for tray")
            .into_rgba8();
        let (w, h) = image.dimensions();
        let rgba = image.into_raw();
        tray_icon::Icon::from_rgba(rgba, w, h).expect("failed to load app icon for tray")
    };

    let tray_menu = Menu::new();
    let show_i = MenuItem::new("Show", true, None);
    let quit_i = MenuItem::new("Quit", true, None);

    let _ = SHOW_WINDOW_ID.set(Mutex::new(show_i.id().clone()));
    let _ = QUIT_WINDOW_ID.set(Mutex::new(quit_i.id().clone()));

    let _ = tray_menu.append_items(&[
        &PredefinedMenuItem::about(Some(&t!("window_title")), None),
        &PredefinedMenuItem::separator(),
        &show_i,
        &PredefinedMenuItem::separator(),
        &quit_i,
    ]);

    TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(tray_menu))
        .with_title(t!("window_title"))
        .with_tooltip(t!("window_title"))
        .build()
        .unwrap()
}
