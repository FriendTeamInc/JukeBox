// Graphical User Interface (pronounced like GIF)

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Duration, Instant};

use eframe::egui::{
    vec2, Align, CentralPanel, Context, Id, Layout, Modal, RichText, ScrollArea, Ui,
    UserAttentionType, ViewportBuilder, ViewportCommand,
};
use eframe::Frame;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use egui_extras::install_image_loaders;
use egui_phosphor::regular as phos;
use jukebox_util::peripheral::DeviceType;
use jukebox_util::rgb::RgbProfile;
use jukebox_util::screen::ScreenProfile;
use jukebox_util::stats::SystemStats;
use rand::prelude::*;
use semver::Version;
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
use crate::firmware_update::{FirmwareUpdateStatus, UpdateError};
use crate::input::InputKey;
use crate::serial::{serial_task, SerialCommand, SerialEvent};
use crate::software_update::software_update_task;
use crate::splash::SPLASH_MESSAGES;
use crate::system::system_task;

const APP_ICON: &[u8] = include_bytes!("../../../assets/applogo.png");
static QUIT_APP: OnceLock<Mutex<bool>> = OnceLock::new();
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
    pub firmware_version: Option<Version>,
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
    pub config_ignore_update_notifications: bool,

    pub profile_renaming: bool,
    pub profile_name_entry: String,

    pub device_renaming: bool,
    pub device_name_entry: String,

    pub editing_key: InputKey,
    pub editing_action_icon: ActionIcon,
    pub editing_action_type: String,
    pub editing_action: Action,

    pub editing_rgb: RgbProfile,

    pub editing_screen: ScreenProfile,

    pub exit_save_modal: bool,

    pub update_progress: f32,
    pub update_status: FirmwareUpdateStatus,
    pub update_error: Option<UpdateError>,

    pub thread_breaker: Arc<AtomicBool>,
    pub sg_rx: UnboundedReceiver<SerialEvent>,
    pub scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,

    pub us_tx: UnboundedSender<FirmwareUpdateStatus>,
    pub us_rx: UnboundedReceiver<FirmwareUpdateStatus>,

    pub gu_rx: UnboundedReceiver<(Version, String)>,
    pub available_version: Version,
    pub current_version: Version,
    pub version_notes: String,
    pub commonmark_cache: CommonMarkCache,
    pub dismissed_update_notif: bool,

    pub generic_errors: VecDeque<String>,

    pub action_map: ActionMap,
    pub ae_rx: UnboundedReceiver<ActionError>,
    pub action_errors: VecDeque<ActionError>,
}
impl eframe::App for JukeBoxGui {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // Call a new frame every frame, bypassing the limited updates.
        // NOTE: This is a bad idea, we should probably change this later
        // and only update the window as necessary.
        // TODO: give ctx to other threads?, so ui can be updated as necessary.
        // but only once
        ctx.request_repaint();

        // TODO: refactor all this to App::tick when available
        // https://github.com/emilk/egui/issues/5113
        self.handle_serial_events(ctx);
        let mut quit_app = QUIT_APP.get().unwrap().blocking_lock();
        #[cfg(target_os = "linux")]
        {
            while let Ok(_e) = TrayIconEvent::receiver().try_recv() {}
            let show_window_id = SHOW_WINDOW_ID.get().unwrap().blocking_lock();
            let quit_window_id = QUIT_WINDOW_ID.get().unwrap().blocking_lock();
            while let Ok(e) = MenuEvent::receiver().try_recv() {
                log::info!("menuevent: {:?}", e);
                if e.id == *show_window_id {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                } else if e.id == *quit_window_id {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                    *quit_app = true;
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            #[cfg(not(target_os = "linux"))]
            {
                *quit_app = true;
            }

            if *quit_app {
                for (_k, tx) in self.scmd_txs.blocking_lock().iter() {
                    let _ = tx.send(SerialCommand::Disconnect);
                    // .expect(&format!("could not send disconnect signal to device {}", k));
                }

                self.thread_breaker
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                return;
            } else {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(ViewportCommand::Visible(false));
            }
        }

        CentralPanel::default().show(ctx, |ui| self.ui(ui));
    }
}
impl JukeBoxGui {
    fn new() -> Self {
        let (config, config_failed_to_load) = JukeBoxConfig::load();
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
                        firmware_version: None,
                        connected: false,
                        device_inputs: HashSet::new(),
                    },
                )
            })
            .collect();
        let current_device = devices.keys().next().unwrap_or(&String::new()).into();
        let config_enable_splash = config.enable_splash;
        let config_always_save_on_exit = config.always_save_on_exit;
        let config_ignore_update_notifications = config.ignore_update_notifications;
        let config = Arc::new(Mutex::new(JukeBoxConfig::load().0));

        // when gui exits, we use these to signal the other threads to stop
        let thread_breaker = Arc::new(AtomicBool::new(false)); // ends other threads from gui
        let brkr_serial = thread_breaker.clone();

        let (sr_tx, sr_rx) = unbounded_channel::<SerialEvent>(); // serial threads send events to action thread
        let (sg_tx, sg_rx) = unbounded_channel::<SerialEvent>(); // serial threads send events to gui thread

        let scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // gui thread sends events to serial threads (specific Device ID -> Device specific Serial Thread)
        // serial thread spawns the channels and gives the sender to the gui thread through this

        let (us_tx, us_rx) = unbounded_channel::<FirmwareUpdateStatus>(); // update thread sends update statuses to gui thread
        let (ae_tx, ae_rx) = unbounded_channel::<ActionError>(); // action thread sends action errors to gui thread
        let (gu_tx, gu_rx) = unbounded_channel::<(Version, String)>(); // software update thread sends update available signal to gui thread

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
        spawn(async move { software_update_task(gu_tx).await });

        let mut generic_errors = VecDeque::new();

        if config_failed_to_load {
            generic_errors.push_back(
                t!(
                    "help.generic.err.config_failed_to_load",
                    config_dir = JukeBoxConfig::get_dir().display()
                )
                .into(),
            );
        }

        JukeBoxGui {
            splash_timer: Instant::now(),
            splash_index: 0usize,

            gui_tab: GuiTab::Device,

            current_device: current_device,
            devices: devices,

            config: config,
            config_enable_splash: config_enable_splash,
            config_always_save_on_exit: config_always_save_on_exit,
            config_ignore_update_notifications: config_ignore_update_notifications,

            profile_renaming: false,
            profile_name_entry: String::new(),

            device_renaming: false,
            device_name_entry: String::new(),

            editing_key: InputKey::UnknownKey,
            editing_action_icon: ActionIcon::DefaultActionIcon,
            editing_action_type: "MetaNoAction".into(),
            editing_action: Action::MetaNoAction(MetaNoAction::default()),

            editing_rgb: RgbProfile::default_gui_profile(),

            editing_screen: ScreenProfile::default_profile(),

            exit_save_modal: false,

            update_progress: 0.0,
            update_status: FirmwareUpdateStatus::Start,

            thread_breaker: thread_breaker,
            sg_rx: sg_rx,
            scmd_txs: scmd_txs,

            us_tx: us_tx,
            us_rx: us_rx,
            update_error: None,

            gu_rx: gu_rx,
            available_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            current_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            version_notes: String::new(),
            commonmark_cache: CommonMarkCache::default(),
            dismissed_update_notif: true,

            generic_errors: generic_errors,

            action_map: ActionMap::new(),
            ae_rx: ae_rx,
            action_errors: VecDeque::new(),
        }
    }

    fn ui(&mut self, ui: &mut Ui) {
        ui.allocate_ui(vec2(464.0, 22.0), |ui| {
            ui.horizontal(|ui| {
                self.draw_back_button(ui);
                self.draw_profile_management(ui);
                self.draw_settings_toggle(ui);
            });
            ui.allocate_space(ui.available_size_before_wrap());
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

        if self.update_error.is_some() {
            let update_error = self.update_error.clone().unwrap();
            Modal::new(Id::new("UpdateErrorModal")).show(ui.ctx(), |ui| {
                ui.set_width(400.0);

                ui.heading(t!("help.update.error_modal_title"));

                ui.add_space(10.0);

                ui.label(update_error.msg);

                ui.add_space(15.0);

                ui.label(t!("help.update.error_modal_reconnect_or_manual_update"));

                ui.add_space(15.0);

                ui.horizontal_centered(|ui| {
                    if ui.button(t!("help.update.error_modal_exit")).clicked() {
                        self.update_error = None;
                        self.gui_tab = GuiTab::Device;
                    }
                });
            });
        }

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

        if self.current_version != self.available_version && !self.dismissed_update_notif {
            Modal::new(Id::new("UpdateAvailableModal")).show(ui.ctx(), |ui| {
                ui.set_width(400.0);
                ui.set_height(200.0);
                ui.heading(t!("help.update.modal_title"));
                ui.label(format!(
                    "v{} -> v{}",
                    self.current_version, self.available_version
                ));
                ui.add_space(10.0);

                ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    CommonMarkViewer::new().show(
                        ui,
                        &mut self.commonmark_cache,
                        &self.version_notes,
                    );
                });

                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    if ui.button(t!("help.update.modal_remind_me_later")).clicked() {
                        self.dismissed_update_notif = true;
                    }

                    if ui.button(t!("help.update.modal_ignore_updates")).clicked() {
                        self.dismissed_update_notif = true;
                        self.config_ignore_update_notifications = true;

                        let mut conf = self.config.blocking_lock();
                        conf.ignore_update_notifications = self.config_ignore_update_notifications;
                        conf.save();
                    }

                    if ui.button(t!("help.update.modal_download_update")).clicked() {
                        self.dismissed_update_notif = true;
                        let _ =
                            open::that("https://github.com/FriendTeamInc/JukeBox/releases/latest");
                    }
                })
            });
        }

        if self.generic_errors.len() > 0 {
            let generic_error = self.generic_errors.get(0).unwrap().clone();
            Modal::new(Id::new("GenericErrorModal")).show(ui.ctx(), |ui| {
                ui.set_width(400.0);

                ui.heading(t!("help.generic.modal_title"));

                ui.add_space(10.0);

                CommonMarkViewer::new().show(ui, &mut self.commonmark_cache, &generic_error);

                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    if ui.button(t!("help.generic.modal_exit")).clicked() {
                        self.generic_errors.pop_front();
                    }
                });
            });
        }
    }

    fn handle_serial_events(&mut self, ctx: &Context) {
        let mut bring_back_up = false;

        // recieve update available events
        while let Ok((new_version, version_notes)) = self.gu_rx.try_recv() {
            self.available_version = new_version;
            self.version_notes = version_notes;
            if !self.config_ignore_update_notifications {
                self.dismissed_update_notif = false;
                bring_back_up = true;
            }
        }

        // recieve action error events
        while let Ok(error) = self.ae_rx.try_recv() {
            self.action_errors.push_back(error);
            bring_back_up = true;
        }
        if bring_back_up {
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
                        v.firmware_version = Some(Version::parse(&firmware_version).unwrap());
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
                                firmware_version: Some(Version::parse(&firmware_version).unwrap()),
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
            GuiTab::EditingAction => self.save_action(),
            GuiTab::EditingRGB => self.save_rgb(),
            GuiTab::EditingScreen => self.save_screen(),
            _ => (),
        }
    }

    fn draw_back_button(&mut self, ui: &mut Ui) {
        // back button
        let enabled = self.gui_tab != GuiTab::Device
            && (self.update_status == FirmwareUpdateStatus::Start
                || self.update_status == FirmwareUpdateStatus::End);
        ui.add_enabled_ui(enabled, |ui| {
            let saveable = match self.gui_tab {
                GuiTab::EditingAction => self.is_action_changed(),
                GuiTab::EditingRGB => self.is_rgb_changed(),
                GuiTab::EditingScreen => self.is_screen_changed(),
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
            .with_inner_size([960.0, 680.0])
            .with_maximize_button(false)
            .with_resizable(false)
            .with_icon(eframe::icon_data::from_png_bytes(&APP_ICON[..]).unwrap()),
        centered: true,
        ..Default::default()
    };

    let _ = QUIT_APP.set(Mutex::new(false));

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
