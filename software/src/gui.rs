// Graphical User Interface (pronounced like GIF)

use std::collections::{HashMap, HashSet};
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Duration, Instant};

use eframe::egui::scroll_area::ScrollBarVisibility;
use eframe::egui::{
    vec2, Align, Button, CentralPanel, CollapsingHeader, Color32, ComboBox, Grid, Layout,
    ProgressBar, RichText, ScrollArea, TextBuffer, TextEdit, Ui, ViewportBuilder,
};
use egui_phosphor::regular as phos;
use egui_theme_switch::global_theme_switch;
use jukebox_util::peripheral::{
    IDENT_KEY_INPUT, IDENT_KNOB_INPUT, IDENT_PEDAL_INPUT, IDENT_UNKNOWN_INPUT,
};
use rand::prelude::*;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use tokio::{
    runtime::Runtime,
    spawn,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};

use crate::action::action_task;
use crate::actions::meta::MetaNoAction;
use crate::actions::types::{Action, ActionMap, ActionType};
use crate::config::JukeBoxConfig;
use crate::input::InputKey;
use crate::serial::{serial_task, SerialCommand, SerialEvent};
use crate::splash::SPLASH_MESSAGES;
use crate::update::{update_task, UpdateStatus};

const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(PartialEq)]
enum GuiTab {
    Device,
    Editing,
    Settings,
    Updating,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum DeviceType {
    Unknown,
    KeyPad,
    KnobPad,
    PedalPad,
}
impl Into<DeviceType> for u8 {
    fn into(self) -> DeviceType {
        match self {
            IDENT_KEY_INPUT => DeviceType::KeyPad,
            IDENT_KNOB_INPUT => DeviceType::KnobPad,
            IDENT_PEDAL_INPUT => DeviceType::PedalPad,
            _ => DeviceType::Unknown,
        }
    }
}
impl Into<u8> for DeviceType {
    fn into(self) -> u8 {
        match self {
            DeviceType::Unknown => IDENT_UNKNOWN_INPUT,
            DeviceType::KeyPad => IDENT_KEY_INPUT,
            DeviceType::KnobPad => IDENT_KNOB_INPUT,
            DeviceType::PedalPad => IDENT_PEDAL_INPUT,
        }
    }
}

struct JukeBoxGui {
    splash_timer: Instant,
    splash_index: usize,

    gui_tab: GuiTab,

    current_device: String,
    // Device UID -> (DeviceType, Device Nickname, Firmware Version, Connected?, Device Inputs)
    devices: HashMap<String, (DeviceType, String, String, bool, HashSet<InputKey>)>,

    config: Arc<Mutex<JukeBoxConfig>>,
    config_renaming_profile: bool,
    config_profile_name_entry: String,
    config_renaming_device: bool,
    config_device_name_entry: String,
    config_editing_key: InputKey,
    config_editing_action_type: ActionType,
    config_editing_action: Box<dyn Action>,
    config_enable_splash: bool,

    update_progress: f32,
    update_status: UpdateStatus,

    action_map: ActionMap,
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
                        (v.0, v.1.clone(), "?".to_string(), false, HashSet::new()),
                    )
                })
                .collect();
        let current_device = devices.keys().next().unwrap_or(&String::new()).to_string();
        let config_enable_splash = config.enable_splash;
        let config = Arc::new(Mutex::new(JukeBoxConfig::load()));

        JukeBoxGui {
            splash_timer: Instant::now(),
            splash_index: 0usize,

            gui_tab: GuiTab::Device,

            current_device: current_device,
            devices: devices,

            config: config,
            config_renaming_profile: false,
            config_profile_name_entry: String::new(),
            config_renaming_device: false,
            config_device_name_entry: String::new(),
            config_editing_key: InputKey::UnknownKey,
            config_editing_action_type: ActionType::MetaNoAction,
            config_editing_action: Box::new(MetaNoAction::default()),
            config_enable_splash: config_enable_splash,

            update_progress: 0.0,
            update_status: UpdateStatus::Start,

            action_map: ActionMap::new(),
        }
    }

    fn run(mut self) {
        // channels cannot be a part of Self due to partial move errors

        // when gui exits, we use these to signal the other threads to stop
        let brkr = Arc::new(AtomicBool::new(false)); // ends other threads from gui
        let brkr_serial = brkr.clone();

        let (sr_tx, sr_rx) = unbounded_channel::<SerialEvent>(); // serial threads send events to action thread
        let (sg_tx, mut sg_rx) = unbounded_channel::<SerialEvent>(); // serial threads send events to gui thread

        let (gs_cmd_tx, mut gs_cmd_rx) =
            unbounded_channel::<(String, UnboundedSender<SerialCommand>)>(); // serial threads send "serial command senders" to gui
        let mut gs_cmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let gs_cmd_txs_end = gs_cmd_txs.clone();

        // gui thread sends events to serial threads (specific Device ID -> Device specific Serial Thread)
        // serial thread spawns the channels and gives the sender to the gui thread through this

        let (us_tx, mut us_rx) = unbounded_channel::<UpdateStatus>(); // update thread sends update statuses to gui thread

        let action_config = self.config.clone();

        let rt = Runtime::new().expect("unable to create tokio runtime");
        let _guard = rt.enter();
        spawn(async move { serial_task(brkr_serial, gs_cmd_tx, sg_tx, sr_tx).await });
        spawn(async move { action_task(sr_rx, action_config).await });

        let options = eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_title(t!("window_title"))
                .with_inner_size([960.0, 640.0])
                .with_maximize_button(false)
                .with_resizable(false)
                .with_icon(
                    eframe::icon_data::from_png_bytes(
                        &include_bytes!("../../assets/applogo.png")[..],
                    )
                    .unwrap(),
                ),
            centered: true,
            ..Default::default()
        };

        eframe::run_simple_native(t!("window_title").as_ref(), options, move |ctx, _frame| {
            // TODO: give ctx to other threads?, so ui can be updated as necessary.
            // but only once

            ctx.set_zoom_factor(2.0);
            let mut fonts = eframe::egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            ctx.set_fonts(fonts);

            self.handle_serial_events(&mut gs_cmd_rx, &mut gs_cmd_txs, &mut sg_rx);

            CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.draw_profile_management(ui);
                    self.draw_settings_toggle(ui);
                });

                ui.separator();

                ui.allocate_ui(vec2(464.0, 245.0), |ui| match self.gui_tab {
                    GuiTab::Device => self.draw_device_page(ui),
                    GuiTab::Settings => self.draw_settings_page(ui),
                    GuiTab::Editing => self.draw_edit_action(ui),
                    GuiTab::Updating => self.draw_update_page(ui, &gs_cmd_txs, &us_tx, &mut us_rx),
                });

                ui.separator();

                ui.columns_const(|[c1, c2]| {
                    c1.with_layout(Layout::left_to_right(Align::BOTTOM), |ui| {
                        self.draw_device_management(ui);
                    });

                    self.draw_splash_text(c2);
                });
            });

            // Call a new frame every frame, bypassing the limited updates.
            // NOTE: This is a bad idea, we should probably change this later
            // and only update the window as necessary.
            ctx.request_repaint();
        })
        .expect("eframe error");

        {
            for (_k, tx) in gs_cmd_txs_end.blocking_lock().iter() {
                let _ = tx.send(SerialCommand::DisconnectDevice);
                // .expect(&format!("could not send disconnect signal to device {}", k));
            }
        }

        brkr.store(true, std::sync::atomic::Ordering::Relaxed);

        rt.shutdown_timeout(Duration::from_secs(1));
    }

    fn handle_serial_events(
        &mut self,
        gs_cmd_rx: &mut UnboundedReceiver<(String, UnboundedSender<SerialCommand>)>,
        gs_cmd_txs: &mut Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
        s_evnt_rx: &mut UnboundedReceiver<SerialEvent>,
    ) {
        {
            let mut gs_cmd_txs = gs_cmd_txs.blocking_lock();
            while let Ok((device_uid, gs_cmd_tx)) = gs_cmd_rx.try_recv() {
                gs_cmd_txs.insert(device_uid, gs_cmd_tx);
            }
        }

        while let Ok(event) = s_evnt_rx.try_recv() {
            match event {
                SerialEvent::Connected { device_info } => {
                    let device_uid = device_info.device_uid;
                    let device_type = device_info.input_identifier;
                    let firmware_version = device_info.firmware_version;

                    let short_uid = device_uid[..4].to_string();

                    // TODO: double check that the device is fine to use
                    let device_name = match Into::<DeviceType>::into(device_type) {
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
                                        self.action_map.default_action_config(device_type.into()),
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
                            device_uid,
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
                    let mut gs_cmd_txs = gs_cmd_txs.blocking_lock();
                    gs_cmd_txs.remove(&device_uid);
                }
                SerialEvent::Disconnected { device_uid } => {
                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.3 = false;
                        v.4.clear();
                    }
                    let mut gs_cmd_txs = gs_cmd_txs.blocking_lock();
                    gs_cmd_txs.remove(&device_uid);
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

    fn draw_device_page(&mut self, ui: &mut Ui) {
        let devices = &self.devices;
        let current_device = &self.current_device;

        if devices.len() <= 0 || current_device.is_empty() {
            self.draw_no_device(ui);
            return;
        }

        let device_type = if let Some(b) = devices.get(current_device) {
            b.0
        } else {
            DeviceType::Unknown
        };

        match device_type {
            DeviceType::Unknown => self.draw_unknown_device(ui),
            DeviceType::KeyPad => self.draw_keypad_device(ui),
            DeviceType::KnobPad => self.draw_unknown_device(ui),
            DeviceType::PedalPad => self.draw_pedalpad_device(ui),
        }
    }

    fn draw_settings_page(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(t!("settings.title"))
                    .heading()
                    .color(Color32::from_rgb(255, 200, 100)),
            );
            ui.label(format!("-  v{}", APP_VERSION));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                global_theme_switch(ui);
            });
        });

        ui.label("");

        if ui
            .checkbox(&mut self.config_enable_splash, t!("help.settings.splash"))
            .changed()
        {
            let mut conf = self.config.blocking_lock();
            conf.enable_splash = self.config_enable_splash;
            conf.save();
        }

        ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                ui.hyperlink_to(
                    t!("settings.donate"),
                    "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                );
                ui.label(" - ");
                ui.hyperlink_to(
                    t!("settings.repository"),
                    "https://github.com/FriendTeamInc/JukeBox",
                );
                ui.label(" - ");
                ui.hyperlink_to(t!("settings.homepage"), "https://jukebox.friendteam.biz");
            });
            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                ui.label(t!("settings.copyright"));
            });
        });
    }

    fn draw_device_management(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(self.gui_tab == GuiTab::Device, |ui| {
            if self.config_renaming_device {
                let edit = ui.add(
                    TextEdit::singleline(&mut self.config_device_name_entry).desired_width(192.0),
                );
                if edit.lost_focus() && self.config_device_name_entry.len() > 0 {
                    self.config_renaming_device = false;

                    let contains = self
                        .devices
                        .iter()
                        .any(|(_, d)| d.1 == self.config_device_name_entry);

                    if !contains {
                        let d = self.devices.get_mut(&self.current_device).expect("");
                        d.1 = self.config_device_name_entry.clone();

                        let mut conf = self.config.blocking_lock();
                        let c = conf.devices.get_mut(&self.current_device).expect("");
                        c.1 = self.config_device_name_entry.clone();
                        conf.save();
                    }
                }
                if !edit.has_focus() {
                    edit.request_focus();
                }
            } else {
                let current_name = if !self.current_device.is_empty() {
                    &self.devices.get(&self.current_device).unwrap().1
                } else {
                    &String::new()
                };
                ui.add_enabled_ui(self.devices.iter().count() != 0, |ui| {
                    let mut devices = self.devices.iter().map(|v| v.clone()).collect::<Vec<_>>();
                    devices.sort_by(|a, b| a.1 .1.cmp(&b.1 .1));

                    ComboBox::from_id_salt("DeviceSelect")
                        .selected_text(current_name.clone())
                        .width(200.0)
                        .truncate()
                        .show_ui(ui, |ui| {
                            for (k, v) in &devices {
                                let u = ui.selectable_label(v.1 == *current_name, v.1.clone());
                                if u.clicked() {
                                    self.current_device = k.to_string();
                                }
                            }
                        })
                        .response
                        .on_hover_text_at_pointer(t!("help.device.select"));
                });
            }

            ui.add_enabled_ui(!self.config_renaming_device, |ui| {
                if self.devices.keys().len() <= 0 {
                    ui.disable();
                }

                let edit_btn = ui
                    .button(RichText::new(phos::NOTE_PENCIL))
                    .on_hover_text_at_pointer(t!("help.device.edit_name"));
                if edit_btn.clicked() {
                    self.config_renaming_device = true;
                    self.config_device_name_entry
                        .replace_with(&self.devices.get(&self.current_device).unwrap().1);
                }

                let delete_btn = ui
                    .button(RichText::new(phos::TRASH))
                    .on_hover_text_at_pointer(t!("help.device.forget"));
                if delete_btn.clicked() {
                    // TODO: make red
                }
                if delete_btn.double_clicked() {
                    let old_device = self.current_device.clone();
                    self.devices.remove(&old_device);
                    self.current_device = self
                        .devices
                        .keys()
                        .next()
                        .unwrap_or(&String::new())
                        .to_string();

                    let mut conf = self.config.blocking_lock();
                    conf.devices.remove(&old_device);
                    for (_, p) in conf.profiles.iter_mut() {
                        p.remove_entry(&old_device);
                    }
                    conf.save();

                    // maybe disconnect device over serial?
                }
            })
        });
    }

    fn draw_profile_management(&mut self, ui: &mut Ui) {
        // back button
        ui.add_enabled_ui(
            self.gui_tab != GuiTab::Device
                && (self.update_status == UpdateStatus::Start
                    || self.update_status == UpdateStatus::End),
            |ui| {
                if ui
                    .button(RichText::new(phos::ARROW_BEND_UP_LEFT))
                    .on_hover_text_at_pointer(match self.gui_tab {
                        GuiTab::Editing => t!("help.back.save_button"),
                        _ => t!("help.back.button"),
                    })
                    .clicked()
                {
                    match self.gui_tab {
                        GuiTab::Editing => self.save_action_and_exit(),
                        _ => self.gui_tab = GuiTab::Device,
                    }
                }
            },
        );

        ui.add_enabled_ui(self.gui_tab == GuiTab::Device, |ui| {
            // Profile select/edit
            if self.config_renaming_profile {
                // TODO: this shifts everything down a bit too much, fix later
                let edit = ui.add(
                    TextEdit::singleline(&mut self.config_profile_name_entry).desired_width(142.0),
                );
                if edit.lost_focus() && self.config_profile_name_entry.len() > 0 {
                    self.config_renaming_profile = false;

                    let mut conf = self.config.blocking_lock();

                    if !conf.profiles.contains_key(&self.config_profile_name_entry) {
                        let p = conf.current_profile.clone();
                        let c = conf.profiles.remove(&p).expect("");
                        conf.profiles
                            .insert(self.config_profile_name_entry.clone(), c);
                        conf.current_profile
                            .replace_with(&self.config_profile_name_entry);

                        // TODO: edit configs to reference new profile instead of wiping it
                        for (_, p) in conf.profiles.iter_mut() {
                            for (_, d) in p.iter_mut() {
                                for (_, k) in d.iter_mut() {
                                    use ActionType as r;
                                    match k.get_type() {
                                        r::MetaSwitchProfile => {
                                            *k = self.action_map.enum_new(r::MetaSwitchProfile);
                                        }
                                        r::MetaCopyFromProfile => {
                                            *k = self.action_map.enum_new(r::MetaCopyFromProfile);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }

                        conf.save();
                    }
                }
                if !edit.has_focus() {
                    edit.request_focus();
                }
            } else {
                let mut conf = self.config.blocking_lock();
                let mut profiles = conf.profiles.keys().cloned().collect::<Vec<_>>();
                profiles.sort_by(|a, b| a.cmp(b));
                let current = conf.current_profile.clone();
                ComboBox::from_id_salt("ProfileSelect")
                    .selected_text(current.clone())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for k in &profiles {
                            let u = ui.selectable_label(*k == current, &*k.clone());
                            if u.clicked() {
                                conf.current_profile = k.to_string();
                                conf.save();
                            }
                        }
                    })
                    .response
                    .on_hover_text_at_pointer(t!("help.profile.select"));
            }

            // Profile management
            ui.add_enabled_ui(!self.config_renaming_profile, |ui| {
                let new_btn = ui
                    .button(RichText::new(phos::PLUS_CIRCLE))
                    .on_hover_text_at_pointer(t!("help.profile.new"));
                if new_btn.clicked() {
                    let mut conf = self.config.blocking_lock();
                    let mut idx = conf.profiles.keys().len() + 1;
                    let name = loop {
                        let name = t!("profile_name_new", idx = idx).to_string();
                        if !conf.profiles.contains_key(&name) {
                            break name;
                        }
                        idx += 1;
                    };
                    let mut m = HashMap::new();
                    for (d, t) in &self.devices {
                        m.insert(d.clone(), self.action_map.default_action_config(t.0.into()));
                    }
                    conf.profiles.insert(name, m);
                    conf.save();
                }

                let edit_btn = ui
                    .button(RichText::new(phos::NOTE_PENCIL))
                    .on_hover_text_at_pointer(t!("help.profile.edit_name"));
                if edit_btn.clicked() {
                    let conf = self.config.blocking_lock();
                    self.config_renaming_profile = true;
                    self.config_profile_name_entry
                        .replace_with(&conf.current_profile);
                }

                let mut conf = self.config.blocking_lock();
                if conf.profiles.keys().len() <= 1 {
                    ui.disable();
                }
                let delete_btn = ui
                    .button(RichText::new(phos::TRASH))
                    .on_hover_text_at_pointer(t!("help.profile.delete"));
                if delete_btn.clicked() {
                    // TODO: make red
                }
                if delete_btn.double_clicked() {
                    let p = conf.current_profile.clone();
                    conf.profiles.remove(&p);
                    conf.current_profile = conf.profiles.keys().next().unwrap().clone();

                    for (_, p) in conf.profiles.iter_mut() {
                        for (_, d) in p.iter_mut() {
                            for (_, k) in d.iter_mut() {
                                use ActionType as r;
                                match k.get_type() {
                                    r::MetaSwitchProfile => {
                                        *k = self.action_map.enum_new(r::MetaSwitchProfile);
                                    }
                                    r::MetaCopyFromProfile => {
                                        *k = self.action_map.enum_new(r::MetaCopyFromProfile);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    conf.save();
                }
            });
        });
    }

    fn draw_settings_toggle(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.scope(|ui| {
                if self.gui_tab != GuiTab::Settings && self.gui_tab != GuiTab::Device {
                    ui.disable();
                }

                let settings_btn = ui
                    .selectable_label(
                        self.gui_tab == GuiTab::Settings,
                        RichText::new(phos::GEAR_FINE),
                    )
                    .on_hover_text_at_pointer(t!("help.settings.button"));
                if settings_btn.clicked() {
                    match self.gui_tab {
                        GuiTab::Device => self.gui_tab = GuiTab::Settings,
                        GuiTab::Settings => self.gui_tab = GuiTab::Device,
                        _ => (),
                    }
                }
            });
        });
    }

    fn draw_no_device(&mut self, ui: &mut Ui) {
        ui.with_layout(
            Layout::centered_and_justified(eframe::egui::Direction::TopDown),
            |ui| ui.label(t!("help.no_device")),
        );
    }

    fn draw_unknown_device(&mut self, ui: &mut Ui) {
        ui.with_layout(
            Layout::centered_and_justified(eframe::egui::Direction::TopDown),
            |ui| ui.label(t!("help.unknown_device")),
            // TODO: add update button
        );
        // ui.allocate_space(ui.available_size_before_wrap());
    }

    fn draw_device_firmware_management(&mut self, ui: &mut Ui) {
        ui.allocate_ui(vec2(60.0, 231.5), |ui| {
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                let i = self.devices.get(&self.current_device).unwrap();
                ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
                    let s = match i.3 {
                        true => RichText::new(phos::PLUGS_CONNECTED)
                            .color(Color32::from_rgb(63, 192, 63)),
                        false => RichText::new(phos::PLUGS).color(Color32::from_rgb(192, 63, 63)),
                    };
                    ui.add_enabled_ui(i.3, |ui| {
                        if ui
                            .button(s)
                            .on_hover_text_at_pointer(t!("help.device.identify"))
                            .clicked()
                        {
                            log::info!("TODO: Identify Device");
                        }

                        if ui
                            .button(phos::DOWNLOAD)
                            .on_hover_text_at_pointer(t!("help.device.update"))
                            .clicked()
                        {
                            self.gui_tab = GuiTab::Updating;
                            self.update_progress = 0.0;
                            self.update_status = UpdateStatus::Start;
                        }
                    });
                });
                ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
                    ui.label(
                        RichText::new(format!("ID:{}", self.current_device))
                            .monospace()
                            .size(5.0),
                    );
                });
                ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
                    ui.label(
                        RichText::new(format!("Firmware: {}", i.2))
                            .monospace()
                            .size(5.0),
                    );
                });
                ui.allocate_space(ui.available_size_before_wrap());
            });
        });
    }

    fn draw_keypad_device(&mut self, ui: &mut Ui) {
        ui.allocate_space(vec2(0.0, 4.0));
        ui.horizontal_top(|ui| {
            ui.allocate_ui(vec2(62.0, 231.5), |ui| {
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                        if ui
                            .button(phos::SIREN)
                            .on_hover_text_at_pointer(t!("help.device.rgb"))
                            .clicked()
                        {
                            log::info!("TODO: RGB Control");
                        }
                        if ui
                            .button(phos::MONITOR)
                            .on_hover_text_at_pointer(t!("help.device.screen"))
                            .clicked()
                        {
                            log::info!("TODO: Screen Control");
                        }
                    });
                    ui.allocate_space(ui.available_size_before_wrap());
                });
            });

            Grid::new("KBGrid").show(ui, |ui| {
                let keys = [
                    [
                        InputKey::KeySwitch1,
                        InputKey::KeySwitch2,
                        InputKey::KeySwitch3,
                        InputKey::KeySwitch4,
                    ],
                    [
                        InputKey::KeySwitch5,
                        InputKey::KeySwitch6,
                        InputKey::KeySwitch7,
                        InputKey::KeySwitch8,
                    ],
                    [
                        InputKey::KeySwitch9,
                        InputKey::KeySwitch10,
                        InputKey::KeySwitch11,
                        InputKey::KeySwitch12,
                    ],
                    // [
                    //     InputKey::KeySwitch13,
                    //     InputKey::KeySwitch14,
                    //     InputKey::KeySwitch15,
                    //     InputKey::KeySwitch16,
                    // ],
                ];
                for (y, k) in keys.iter().enumerate() {
                    for (x, k) in k.iter().enumerate() {
                        let s = format!("F{}", 12 + x + y * 4 + 1);
                        let rt = RichText::new(s).heading().strong();
                        let mut b = Button::new(rt);
                        let inputs = if let Some(s) = self.devices.get(&self.current_device) {
                            s.4.clone()
                        } else {
                            HashSet::new()
                        };
                        if inputs.contains(k) {
                            b = b.corner_radius(20u8);
                        }
                        let btn = ui.add_sized([75.0, 75.0], b);
                        // TODO: display some better text in the buttons
                        // TODO: add hover text for button info

                        if btn.clicked() {
                            self.enter_action_editor(k.to_owned());
                        }
                    }
                    ui.end_row();
                }
            });

            self.draw_device_firmware_management(ui);
        });
        ui.allocate_space(ui.available_size_before_wrap());
    }

    fn draw_pedalpad_device(&mut self, ui: &mut Ui) {
        ui.allocate_space(vec2(0.0, 4.0));
        ui.horizontal_top(|ui| {
            ui.allocate_ui(vec2(62.0, 231.5), |ui| {
                ui.allocate_space(ui.available_size_before_wrap());
            });

            ui.allocate_ui([324.0, 231.0].into(), |ui| {
                ui.columns_const(|[c1, c2, c3]| {
                    let inputs = if let Some(s) = self.devices.get(&self.current_device) {
                        s.4.clone()
                    } else {
                        HashSet::new()
                    };

                    let p1 = Button::new(RichText::new("L").heading().strong());
                    let p2 = Button::new(RichText::new("M").heading().strong());
                    let p3 = Button::new(RichText::new("R").heading().strong());

                    let mut i = |c: &mut Ui, mut p: Button<'_>, b| {
                        if inputs.contains(&b) {
                            p = p.corner_radius(20u8);
                        }
                        let btn = c.add_sized([100.0, 231.0], p);
                        // TODO: display some better text in the buttons
                        // TODO: add hover text for button info

                        if btn.clicked() {
                            self.enter_action_editor(b);
                        }
                    };

                    i(c1, p1, InputKey::PedalLeft);
                    i(c2, p2, InputKey::PedalMiddle);
                    i(c3, p3, InputKey::PedalRight);
                });
            });

            self.draw_device_firmware_management(ui);
        });
        ui.allocate_space(ui.available_size_before_wrap());
    }

    fn enter_action_editor(&mut self, key: InputKey) {
        self.config_renaming_device = false;
        self.config_renaming_profile = false;
        self.gui_tab = GuiTab::Editing;
        self.config_editing_key = key;
        {
            let c = self.config.blocking_lock();
            if let Some(r) = c
                .profiles
                .get(&c.current_profile)
                .and_then(|p| p.get(&self.current_device))
                .and_then(|d| d.get(&self.config_editing_key))
            {
                self.config_editing_action_type = r.get_type();
                self.config_editing_action = r.clone();
            } else {
                self.config_editing_action_type = ActionType::MetaNoAction;
                self.config_editing_action =
                    self.action_map.enum_new(self.config_editing_action_type);
            }
        };
    }

    fn reset_editing_action(&mut self) {
        self.config_editing_action = self.action_map.enum_new(self.config_editing_action_type);
    }

    fn save_action_and_exit(&mut self) {
        // TODO: have config validate input?
        self.gui_tab = GuiTab::Device;
        let mut c = self.config.blocking_lock();
        let current_profile = c.current_profile.clone();
        let profile = c.profiles.get_mut(&current_profile).unwrap();
        if let Some(d) = profile.get_mut(&self.current_device) {
            d.insert(
                self.config_editing_key.clone(),
                self.config_editing_action.clone(),
            );
        } else {
            let mut d = HashMap::new();
            d.insert(
                self.config_editing_key.clone(),
                self.config_editing_action.clone(),
            );
            profile.insert(self.current_device.clone(), d);
        }
        c.save();
    }

    fn draw_action_list(&mut self, ui: &mut Ui) {
        for (header, options) in self.action_map.ui_list() {
            CollapsingHeader::new(RichText::new(header).strong())
                .default_open(true)
                .show(ui, |ui| {
                    for (action_type, label) in options {
                        if ui
                            .selectable_value(
                                &mut self.config_editing_action_type,
                                action_type,
                                label,
                            )
                            .changed()
                        {
                            self.reset_editing_action();
                        };
                    }
                });
            ui.end_row();
        }
    }

    fn draw_edit_action(&mut self, ui: &mut Ui) {
        ui.columns_const(|[c1, c2]| {
            c1.horizontal(|ui| {
                let test_btn = ui
                    .add_sized(
                        [50.0, 50.0],
                        Button::new(RichText::new(phos::APERTURE).heading().strong()),
                    )
                    .on_hover_text_at_pointer(t!("help.action.test_input"));
                if test_btn.clicked() {
                    // TODO: fix for discord and hardware inputs
                    // let mut c = self.config.blocking_lock().clone();
                    // let h = Handle::current();
                    // let _ = h.block_on(async {
                    //     self.config_editing_action
                    //         .on_press(&self.current_device, self.config_editing_key, &mut c)
                    //         .await
                    // });
                    // let _ = h.block_on(async {
                    //     self.config_editing_action
                    //         .on_press(&self.current_device, self.config_editing_key, &mut c)
                    //         .await
                    // });
                }
                ui.vertical(|ui| {
                    ui.allocate_space(vec2(0.0, 2.0));
                    if ui
                        .button(RichText::new(phos::FOLDER))
                        .on_hover_text_at_pointer(t!("help.action.image_icon"))
                        .clicked()
                    {
                        log::info!("TODO: choose image icon");
                    }
                    if ui
                        .button(RichText::new(phos::SEAL))
                        .on_hover_text_at_pointer(t!("help.action.glyph_icon"))
                        .clicked()
                    {
                        log::info!("TODO: choose glyph icon");
                    }
                });
                ui.with_layout(
                    Layout::centered_and_justified(eframe::egui::Direction::TopDown)
                        .with_cross_justify(false),
                    |ui| {
                        ui.label(RichText::new(self.config_editing_action.help()).size(10.0));
                    },
                );
            });
            c1.allocate_space(vec2(0.0, 2.0));
            c1.separator();
            c1.allocate_space(vec2(0.0, 2.0));
            c1.allocate_ui(vec2(228.0, 170.0), |ui| {
                ScrollArea::vertical()
                    .id_salt("ActionEdit")
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                            self.config_editing_action.edit_ui(
                                ui,
                                &self.current_device,
                                self.config_editing_key,
                                self.config.clone(),
                            );
                            ui.allocate_space(ui.available_size_before_wrap());
                        });
                    });
            });

            c2.allocate_ui(vec2(228.0, 245.0), |ui| {
                ScrollArea::vertical()
                    .id_salt("ActionChooser")
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        Grid::new("ActionsGrid")
                            .num_columns(1)
                            .min_col_width(228.0)
                            .striped(true)
                            .show(ui, |ui| {
                                self.draw_action_list(ui);
                            });
                        ui.allocate_space(ui.available_size_before_wrap());
                    });
            });
        });
    }

    fn send_update_signal(
        gs_cmd_txs: &Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
        device_uid: String,
        fw_path: String,
        us_tx: &UnboundedSender<UpdateStatus>,
    ) {
        {
            let gs_cmd_txs = gs_cmd_txs.blocking_lock();
            if let Some(tx) = gs_cmd_txs.get(&device_uid) {
                tx.send(SerialCommand::UpdateDevice)
                    .expect("failed to send update command");
            }
        }

        let us_tx2 = us_tx.clone();
        spawn(async move {
            update_task(device_uid, fw_path, us_tx2).await;
        });
    }

    fn draw_update_page(
        &mut self,
        ui: &mut Ui,
        gs_cmd_txs: &Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
        us_tx: &UnboundedSender<UpdateStatus>,
        us_rx: &mut UnboundedReceiver<UpdateStatus>,
    ) {
        ui.vertical_centered(|ui| {
            ui.allocate_space(vec2(0.0, 10.0));
            ui.heading(t!("update.title"));

            // TODO: add some basic info (firmware versions, "do not uplug or power off", etc)
            ui.allocate_space(vec2(0.0, 75.0));

            ui.horizontal(|ui| {
                let dl_update =
                    Button::new(RichText::new(t!("update.button"))).min_size(vec2(150.0, 30.0));
                let cfw_update = Button::new(RichText::new(t!("update.cfw_button")).size(8.0));

                ui.allocate_space(vec2(149.0, 0.0));

                if self.update_status != UpdateStatus::Start {
                    ui.disable();
                }

                if ui.add(dl_update).clicked() {
                    // TODO: download update from GitHub
                    // Self::send_update_signal(
                    //     gs_cmd_txs,
                    //     self.current_device.clone(),
                    //     f.to_string_lossy().to_string(),
                    //     us_tx,
                    // );
                }
                if ui.add(cfw_update).clicked() {
                    // TODO: ask for file, verify its good, then use it to update the device
                    if let Some(f) = FileDialog::new()
                        .add_filter(t!("update.filter_name"), &["uf2"])
                        .set_directory("~")
                        .pick_file()
                    {
                        Self::send_update_signal(
                            gs_cmd_txs,
                            self.current_device.clone(),
                            f.to_string_lossy().to_string(),
                            us_tx,
                        );
                    }
                }
            });
            ui.allocate_space(vec2(0.0, 25.0));
            ui.horizontal(|ui| {
                ui.allocate_space(vec2(149.0, 0.0));

                while let Ok(p) = us_rx.try_recv() {
                    self.update_status = p;
                    match p {
                        UpdateStatus::Start => self.update_progress = 0.0,
                        UpdateStatus::Connecting => self.update_progress = 0.05,
                        UpdateStatus::PreparingFirmware => self.update_progress = 0.1,
                        UpdateStatus::ErasingOldFirmware(n) => self.update_progress = 0.1 + 0.3 * n,
                        UpdateStatus::WritingNewFirmware(n) => self.update_progress = 0.4 + 0.6 * n,
                        UpdateStatus::End => self.update_progress = 1.0,
                    }
                }

                let p = ProgressBar::new(self.update_progress)
                    // .animate(true)
                    .desired_width(150.0)
                    .desired_height(30.0)
                    .show_percentage();
                ui.add(p);
            });
            ui.allocate_space(vec2(0.0, 10.0));
            ui.label(match self.update_status {
                UpdateStatus::Start => t!("update.status.start"),
                UpdateStatus::Connecting => t!("update.status.connecting"),
                UpdateStatus::PreparingFirmware => t!("update.status.preparing"),
                UpdateStatus::ErasingOldFirmware(_) => t!("update.status.erasing"),
                UpdateStatus::WritingNewFirmware(_) => t!("update.status.writing"),
                UpdateStatus::End => t!("update.status.end"),
            });
        });
        ui.allocate_space(ui.available_size_before_wrap());
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
    JukeBoxGui::new().run();
}
