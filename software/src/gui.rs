// Graphical User Interface (pronounced like GIF)

use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::Builder;
use std::time::{Duration, Instant};

use eframe::egui::scroll_area::ScrollBarVisibility;
use eframe::egui::{
    vec2, Align, Button, CentralPanel, CollapsingHeader, Color32, ComboBox, Grid, Layout, RichText,
    ScrollArea, TextBuffer, TextEdit, Ui, ViewportBuilder,
};
use egui_phosphor::regular as phos;
use egui_theme_switch::global_theme_switch;
use jukebox_util::peripheral::{
    IDENT_KEY_INPUT, IDENT_KNOB_INPUT, IDENT_PEDAL_INPUT, IDENT_UNKNOWN_INPUT,
};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::JukeBoxConfig;
use crate::input::InputKey;
use crate::reaction::reaction_task;
use crate::reactions::meta::MetaNoAction;
use crate::reactions::types::{
    default_reaction_config, reaction_enum_to_new, reaction_ui_list, Reaction, ReactionType,
};
use crate::serial::{serial_task, SerialCommand, SerialEvent};
use crate::splash::SPLASH_MESSAGES;

const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(PartialEq)]
enum GuiTab {
    Device,
    Editing,
    Settings,
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
    config_editing_reaction_type: ReactionType,
    config_editing_reaction: Box<dyn Reaction>,
    config_enable_splash: bool,
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
            config_editing_reaction_type: ReactionType::MetaNoAction,
            config_editing_reaction: Box::new(MetaNoAction::default()),
            config_enable_splash: config_enable_splash,
        }
    }

    fn run(mut self) {
        // channels cannot be a part of Self due to partial move errors

        let (sr_tx, sr_rx) = channel::<SerialEvent>(); // serial threads send events to reaction thread
        let (sg_tx, sg_rx) = channel::<SerialEvent>(); // serial threads send events to gui thread
        let gs_cmd_txs: Arc<Mutex<HashMap<String, Sender<SerialCommand>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        // gui thread sends events to serial threads (specific Device ID -> Device specific Serial Thread)
        // serial thread spawns the channels and gives the sender to the gui thread through this

        // when gui exits, we use these to signal the other threads to stop
        let brkr = Arc::new(AtomicBool::new(false)); // ends other threads from gui
        let brkr_serial = brkr.clone();
        let gs_cmd_txs_serial = gs_cmd_txs.clone();
        let gs_cmd_txs_end = gs_cmd_txs.clone();

        // serial comms thread
        let thread_serial = Builder::new()
            .name("thread_serial_main".to_string())
            .spawn(move || serial_task(brkr_serial, gs_cmd_txs_serial, sg_tx, sr_tx))
            .unwrap();

        // reaction comms thread
        let reaction_config = self.config.clone();
        let thread_reaction = Builder::new()
            .name("thread_reaction_main".to_string())
            .spawn(move || reaction_task(sr_rx, reaction_config))
            .unwrap();

        let options = eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_title("JukeBox Desktop")
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

        let reaction_ui_list = reaction_ui_list();

        eframe::run_simple_native("JukeBox Desktop", options, move |ctx, _frame| {
            ctx.set_zoom_factor(2.0);
            let mut fonts = eframe::egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            ctx.set_fonts(fonts);

            self.handle_serial_events(&sg_rx); // TODO: give ctx to other threads?, so ui can be updated as necessary

            CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.draw_profile_management(ui);
                    self.draw_settings_toggle(ui);
                });

                ui.separator();

                ui.allocate_ui(vec2(464.0, 245.0), |ui| match self.gui_tab {
                    GuiTab::Device => self.draw_device_page(ui, &gs_cmd_txs),
                    GuiTab::Settings => self.draw_settings_page(ui),
                    GuiTab::Editing => self.draw_edit_reaction(ui, &reaction_ui_list),
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

        for (k, tx) in gs_cmd_txs_end.lock().unwrap().iter() {
            tx.send(SerialCommand::DisconnectDevice)
                .expect(&format!("could not send disconnect signal to device {}", k));
        }

        brkr.store(true, std::sync::atomic::Ordering::Relaxed);

        let _ = thread_serial
            .join()
            .expect("could not rejoin serial thread");

        let _ = thread_reaction
            .join()
            .expect("could not rejoin reaction thread");
    }

    fn handle_serial_events(&mut self, s_evnt_rx: &Receiver<SerialEvent>) {
        while let Ok(event) = s_evnt_rx.try_recv() {
            match event {
                SerialEvent::Connected(d) => {
                    let device_uid = d.device_uid;
                    let device_type = d.input_identifier;
                    let firmware_version = d.firmware_version;

                    let short_uid = device_uid[..4].to_string();
                    let device_name = match Into::<DeviceType>::into(device_type) {
                        DeviceType::Unknown => format!("Unknown Device {}", device_uid.clone()),
                        DeviceType::KeyPad => format!("JukeBox KeyPad {}", short_uid),
                        DeviceType::KnobPad => format!("JukeBox KnobPad {}", short_uid),
                        DeviceType::PedalPad => format!("JukeBox PedalPad {}", short_uid),
                    };

                    let mut conf = self.config.lock().unwrap();
                    if !conf.devices.contains_key(&device_uid) {
                        conf.devices.insert(
                            device_uid.clone(),
                            (device_type.into(), device_name.clone()),
                        );
                        for (_, v) in conf.profiles.iter_mut() {
                            if !v.contains_key(&device_uid) {
                                v.insert(
                                    device_uid.clone(),
                                    default_reaction_config(device_type.into()),
                                );
                            }
                        }
                    }
                    conf.save();
                    drop(conf);

                    if self.current_device.is_empty() {
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
                SerialEvent::LostConnection(device_uid) => {
                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.3 = false;
                        v.4.clear();
                    }
                }
                SerialEvent::Disconnected(device_uid) => {
                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.3 = false;
                        v.4.clear();
                    }
                }
                SerialEvent::GetInputKeys((device_uid, input_keys)) => {
                    if self.devices.contains_key(&device_uid) {
                        let v = self.devices.get_mut(&device_uid).unwrap();
                        v.4 = input_keys;
                    }
                }
            }
        }
    }

    fn draw_device_page(
        &mut self,
        ui: &mut Ui,
        gs_cmd_txs: &Arc<Mutex<HashMap<String, Sender<SerialCommand>>>>,
    ) {
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
            DeviceType::KeyPad => self.draw_keypad_device(ui, gs_cmd_txs),
            DeviceType::KnobPad => self.draw_unknown_device(ui),
            DeviceType::PedalPad => self.draw_pedalpad_device(ui, gs_cmd_txs),
        }
    }

    fn draw_settings_page(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("JukeBox Desktop")
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
            .checkbox(
                &mut self.config_enable_splash,
                " - Enable splash text in bottom right.",
            )
            .changed()
        {
            let mut conf = self.config.lock().unwrap();
            conf.enable_splash = self.config_enable_splash;
            conf.save();
        }

        ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                ui.hyperlink_to("Donate", "https://www.youtube.com/watch?v=dQw4w9WgXcQ");
                ui.label(" - ");
                ui.hyperlink_to("Repository", "https://github.com/FriendTeamInc/JukeBox");
                ui.label(" - ");
                ui.hyperlink_to("Homepage", "https://friendteam.biz");
            });
            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                ui.label("Made w/ <3 by Friend Team Inc. (c) 2024");
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
                    // TODO: check that a device with this name doesnt already exist!
                    self.config_renaming_device = false;
                    let d = self.devices.get_mut(&self.current_device).expect("");
                    d.1 = self.config_device_name_entry.clone();

                    let mut conf = self.config.lock().unwrap();
                    let c = conf.devices.get_mut(&self.current_device).expect("");
                    c.1 = self.config_device_name_entry.clone();
                    conf.save();
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
                    ComboBox::from_id_salt("DeviceSelect")
                        .selected_text(current_name.clone())
                        .width(200.0)
                        .truncate()
                        .show_ui(ui, |ui| {
                            // TODO: sort this alphabetically
                            for (k, v) in &self.devices {
                                let u = ui.selectable_label(v.1 == *current_name, v.1.clone());
                                if u.clicked() {
                                    self.current_device = k.clone();
                                }
                            }
                        })
                        .response
                        .on_hover_text_at_pointer("Device Select");
                });
            }

            ui.add_enabled_ui(!self.config_renaming_device, |ui| {
                if self.devices.keys().len() <= 0 {
                    ui.disable();
                }

                let edit_btn = ui
                    .button(RichText::new(phos::NOTE_PENCIL))
                    .on_hover_text_at_pointer("Edit Device Name");
                if edit_btn.clicked() {
                    self.config_renaming_device = true;
                    self.config_device_name_entry
                        .replace_with(&self.devices.get(&self.current_device).unwrap().1);
                }

                let delete_btn = ui
                    .button(RichText::new(phos::TRASH))
                    .on_hover_text_at_pointer("Forget Device");
                if delete_btn.clicked() {
                    let old_device = self.current_device.clone();
                    self.devices.remove(&old_device);
                    self.current_device = self
                        .devices
                        .keys()
                        .next()
                        .unwrap_or(&String::new())
                        .to_string();

                    let mut conf = self.config.lock().unwrap();
                    conf.devices.remove(&old_device);

                    for (_, p) in conf.profiles.iter_mut() {
                        p.remove_entry(&old_device);
                    }

                    conf.save();

                    // TODO: disconnect device over serial?
                }
            })
        });
    }

    fn draw_profile_management(&mut self, ui: &mut Ui) {
        // back button
        ui.add_enabled_ui(self.gui_tab != GuiTab::Device, |ui| {
            if ui
                .button(RichText::new(phos::ARROW_BEND_UP_LEFT))
                .on_hover_text_at_pointer("Back")
                .clicked()
            {
                match self.gui_tab {
                    GuiTab::Editing => self.save_reaction_and_exit(),
                    _ => self.gui_tab = GuiTab::Device,
                }
            }
        });

        ui.add_enabled_ui(self.gui_tab == GuiTab::Device, |ui| {
            // Profile select/edit
            if self.config_renaming_profile {
                // TODO: this shifts everything down a bit too much, fix later
                let edit = ui.add(
                    TextEdit::singleline(&mut self.config_profile_name_entry).desired_width(142.0),
                );
                if edit.lost_focus() && self.config_profile_name_entry.len() > 0 {
                    self.config_renaming_profile = false;
                    let mut conf = self.config.lock().unwrap();

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
                                    use ReactionType as r;
                                    match k.get_type() {
                                        r::MetaSwitchProfile => {
                                            *k = reaction_enum_to_new(r::MetaSwitchProfile);
                                        }
                                        r::MetaCopyFromProfile => {
                                            *k = reaction_enum_to_new(r::MetaCopyFromProfile);
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
                let mut conf = self.config.lock().unwrap();
                let profiles = conf.profiles.clone();
                let current = conf.current_profile.clone();
                ComboBox::from_id_salt("ProfileSelect")
                    .selected_text(conf.current_profile.clone())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        // TODO: sort this alphabetically
                        for (k, _) in &profiles {
                            let u = ui.selectable_label(*k == current, &*k.clone());
                            if u.clicked() {
                                conf.current_profile = k.to_string();
                                conf.save();
                            }
                        }
                    })
                    .response
                    .on_hover_text_at_pointer("Profie Select");
            }

            // Profile management
            ui.add_enabled_ui(!self.config_renaming_profile, |ui| {
                let new_btn = ui
                    .button(RichText::new(phos::PLUS_CIRCLE))
                    .on_hover_text_at_pointer("New Profile");
                if new_btn.clicked() {
                    let mut conf = self.config.lock().unwrap();
                    let mut idx = conf.profiles.keys().len() + 1;
                    let name = loop {
                        let name = format!("Profile {}", idx);
                        if !conf.profiles.contains_key(&name) {
                            break name;
                        }
                        idx += 1;
                    };
                    let mut m = HashMap::new();
                    for (d, t) in &self.devices {
                        m.insert(d.clone(), default_reaction_config(t.0.into()));
                    }
                    conf.profiles.insert(name, m);
                    conf.save();
                }

                let edit_btn = ui
                    .button(RichText::new(phos::NOTE_PENCIL))
                    .on_hover_text_at_pointer("Edit Profile Name");
                if edit_btn.clicked() {
                    let conf = self.config.lock().unwrap();
                    self.config_renaming_profile = true;
                    self.config_profile_name_entry
                        .replace_with(&conf.current_profile);
                }

                let mut conf = self.config.lock().unwrap();
                if conf.profiles.keys().len() <= 1 {
                    ui.disable();
                }
                let delete_btn = ui
                    .button(RichText::new(phos::TRASH))
                    .on_hover_text_at_pointer("Delete Profile");
                if delete_btn.clicked() {
                    let p = conf.current_profile.clone();
                    conf.profiles.remove(&p);
                    conf.current_profile = conf.profiles.keys().next().unwrap().clone();

                    for (_, p) in conf.profiles.iter_mut() {
                        for (_, d) in p.iter_mut() {
                            for (_, k) in d.iter_mut() {
                                use ReactionType as r;
                                match k.get_type() {
                                    r::MetaSwitchProfile => {
                                        *k = reaction_enum_to_new(r::MetaSwitchProfile);
                                    }
                                    r::MetaCopyFromProfile => {
                                        *k = reaction_enum_to_new(r::MetaCopyFromProfile);
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
                    .on_hover_text_at_pointer("Settings");
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
            |ui| ui.label("Please connect a device."),
        );
    }

    fn draw_unknown_device(&mut self, ui: &mut Ui) {
        ui.with_layout(
            Layout::centered_and_justified(eframe::egui::Direction::TopDown),
            |ui| ui.label("Unknown device registered."),
        );
        // ui.allocate_space(ui.available_size_before_wrap());
    }

    fn draw_device_firmware_management(
        &mut self,
        ui: &mut Ui,
        gs_cmd_txs: &Arc<Mutex<HashMap<String, Sender<SerialCommand>>>>,
    ) {
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
                            .on_hover_text_at_pointer("Connected!")
                            .clicked()
                        {
                            log::info!("TODO: Identify Device");
                        }

                        if ui
                            .button(phos::DOWNLOAD)
                            .on_hover_text_at_pointer("Update Device")
                            .clicked()
                        {
                            // TODO: more comprehensive updating
                            let gs_cmd_txs = gs_cmd_txs.lock().unwrap();
                            if let Some(tx) = gs_cmd_txs.get(&self.current_device) {
                                tx.send(SerialCommand::UpdateDevice)
                                    .expect("failed to send update command");
                            }
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

    fn draw_keypad_device(
        &mut self,
        ui: &mut Ui,
        gs_cmd_txs: &Arc<Mutex<HashMap<String, Sender<SerialCommand>>>>,
    ) {
        ui.allocate_space(vec2(0.0, 4.0));
        ui.horizontal_top(|ui| {
            ui.allocate_ui(vec2(62.0, 231.5), |ui| {
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                        if ui
                            .button(phos::SIREN)
                            .on_hover_text_at_pointer("RGB Control")
                            .clicked()
                        {
                            log::info!("TODO: RGB Control");
                        }
                        if ui
                            .button(phos::MONITOR)
                            .on_hover_text_at_pointer("Screen Control")
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
                            self.enter_reaction_editor(k.to_owned());
                        }
                    }
                    ui.end_row();
                }
            });

            self.draw_device_firmware_management(ui, gs_cmd_txs);
        });
        ui.allocate_space(ui.available_size_before_wrap());
    }

    fn draw_pedalpad_device(
        &mut self,
        ui: &mut Ui,
        gs_cmd_txs: &Arc<Mutex<HashMap<String, Sender<SerialCommand>>>>,
    ) {
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
                            self.enter_reaction_editor(b);
                        }
                    };

                    i(c1, p1, InputKey::PedalLeft);
                    i(c2, p2, InputKey::PedalMiddle);
                    i(c3, p3, InputKey::PedalRight);
                });
            });

            self.draw_device_firmware_management(ui, gs_cmd_txs);
        });
        ui.allocate_space(ui.available_size_before_wrap());
    }

    fn enter_reaction_editor(&mut self, key: InputKey) {
        // log::info!("{:?} ui clicked", key);
        self.config_renaming_device = false;
        self.config_renaming_profile = false;
        self.gui_tab = GuiTab::Editing;
        self.config_editing_key = key;
        {
            let c = self.config.lock().unwrap();
            if let Some(r) = c
                .profiles
                .get(&c.current_profile)
                .and_then(|p| p.get(&self.current_device))
                .and_then(|d| d.get(&self.config_editing_key))
            {
                self.config_editing_reaction_type = r.get_type();
                self.config_editing_reaction = r.clone();
            } else {
                self.config_editing_reaction_type = ReactionType::MetaNoAction;
                self.config_editing_reaction =
                    reaction_enum_to_new(self.config_editing_reaction_type);
            }
        };
    }

    fn reset_editing_reaction(&mut self) {
        self.config_editing_reaction = reaction_enum_to_new(self.config_editing_reaction_type);
    }

    fn save_reaction_and_exit(&mut self) {
        // TODO: have config validate input?
        self.gui_tab = GuiTab::Device;
        let mut c = self.config.lock().unwrap();
        let current_profile = c.current_profile.clone();
        let profile = c.profiles.get_mut(&current_profile).unwrap();
        if let Some(d) = profile.get_mut(&self.current_device) {
            d.insert(
                self.config_editing_key.clone(),
                self.config_editing_reaction.clone(),
            );
        } else {
            let mut d = HashMap::new();
            d.insert(
                self.config_editing_key.clone(),
                self.config_editing_reaction.clone(),
            );
            profile.insert(self.current_device.clone(), d);
        }
        c.save();
    }

    fn draw_reaction_list(
        &mut self,
        ui: &mut Ui,
        reaction_ui_list: &Vec<(String, Vec<(ReactionType, String)>)>,
    ) {
        for (header, options) in reaction_ui_list {
            CollapsingHeader::new(RichText::new(header).strong())
                .default_open(true)
                .show(ui, |ui| {
                    for (reaction_type, label) in options {
                        if ui
                            .selectable_value(
                                &mut self.config_editing_reaction_type,
                                *reaction_type,
                                label,
                            )
                            .changed()
                        {
                            self.reset_editing_reaction();
                        };
                    }
                });
            ui.end_row();
        }
    }

    fn draw_edit_reaction(
        &mut self,
        ui: &mut Ui,
        reaction_ui_list: &Vec<(String, Vec<(ReactionType, String)>)>,
    ) {
        ui.columns_const(|[c1, c2]| {
            c1.horizontal(|ui| {
                let test_btn = ui
                    .add_sized(
                        [50.0, 50.0],
                        Button::new(RichText::new(phos::APERTURE).heading().strong()),
                    )
                    .on_hover_text_at_pointer("Test reaction");
                if test_btn.clicked() {
                    let mut c = self.config.lock().unwrap().clone();
                    self.config_editing_reaction.on_press(
                        self.current_device.clone(),
                        self.config_editing_key,
                        &mut c,
                    );
                    self.config_editing_reaction.on_release(
                        self.current_device.clone(),
                        self.config_editing_key,
                        &mut c,
                    );
                }
                ui.vertical(|ui| {
                    ui.allocate_space(vec2(0.0, 2.0));
                    if ui
                        .button(RichText::new(phos::FOLDER))
                        .on_hover_text_at_pointer("Choose image icon")
                        .clicked()
                    {
                        log::info!("TODO: choose image icon");
                    }
                    if ui
                        .button(RichText::new(phos::SEAL))
                        .on_hover_text_at_pointer("Choose glyph icon")
                        .clicked()
                    {
                        log::info!("TODO: choose glyph icon");
                    }
                });
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(self.config_editing_reaction.help()));
                });
            });
            c1.allocate_space(vec2(0.0, 2.0));
            c1.separator();
            c1.allocate_space(vec2(0.0, 2.0));
            c1.allocate_ui(vec2(228.0, 170.0), |ui| {
                ScrollArea::vertical()
                    .id_salt("ReactionEdit")
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                            let mut c = self.config.lock().unwrap().clone();
                            self.config_editing_reaction.edit_ui(
                                ui,
                                self.current_device.clone(),
                                self.config_editing_key,
                                &mut c,
                            );
                            ui.allocate_space(ui.available_size_before_wrap());
                        });
                    });
            });

            c2.allocate_ui(vec2(228.0, 245.0), |ui| {
                ScrollArea::vertical()
                    .id_salt("ReactionChooser")
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        Grid::new("ReactionsGrid")
                            .num_columns(1)
                            .min_col_width(228.0)
                            .striped(true)
                            .show(ui, |ui| {
                                self.draw_reaction_list(ui, reaction_ui_list);
                            });
                        ui.allocate_space(ui.available_size_before_wrap());
                    });
            });
        });
    }

    fn draw_splash_text(&mut self, ui: &mut Ui) {
        if Instant::now() > self.splash_timer {
            loop {
                let new_index = rand::thread_rng().gen_range(0..SPLASH_MESSAGES.len());
                if new_index != self.splash_index {
                    self.splash_index = new_index;
                    break;
                }
            }
            self.splash_timer = Instant::now() + Duration::from_secs(30);
        }
        if self.config_enable_splash {
            ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                ui.label(
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
