use std::collections::HashSet;

use eframe::egui::{
    vec2, Align, Button, Color32, ComboBox, Direction, Grid, Image, ImageSource, Layout, RichText,
    TextBuffer, TextEdit, TextureFilter, TextureOptions, TextureWrapMode, Ui,
};
use egui_phosphor::regular as phos;
use jukebox_util::peripheral::DeviceType;
use jukebox_util::rgb::RgbProfile;
use jukebox_util::screen::ScreenProfile;

use crate::firmware_update::FirmwareUpdateStatus;
use crate::serial::SerialCommand;
use crate::{config::ActionIcon, input::InputKey};

use super::gui::{GuiTab, JukeBoxGui};

impl JukeBoxGui {
    pub fn draw_device_page(&mut self, ui: &mut Ui) {
        let devices = &self.devices;
        let current_device = &self.current_device;

        if devices.len() <= 0 || current_device.is_empty() {
            self.draw_no_device(ui);
            return;
        }

        let device_type = if let Some(b) = devices.get(current_device) {
            b.device_info.device_type
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

    fn draw_no_device(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            ui.label(t!("help.no_device"));
        });
    }

    fn draw_unknown_device(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
            ui.label(t!("help.unknown_device"));
        });
        // TODO: add update and identify button
        // ui.allocate_space(ui.available_size_before_wrap());
    }

    fn draw_keypad_device(&mut self, ui: &mut Ui) {
        ui.allocate_space(vec2(0.0, 4.0));
        ui.horizontal_top(|ui| {
            self.draw_device_extension_management(ui);

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

                let c = {
                    let c = self.config.blocking_lock();
                    let d = c
                        .profiles
                        .get(&c.current_profile)
                        .unwrap()
                        .get(&self.current_device)
                        .unwrap()
                        .clone();
                    d
                };

                for k in keys.iter() {
                    for k in k.iter() {
                        let ac = c.key_map.get(&k).map(|c| c.clone()).unwrap_or_default();
                        let a = ac.action;
                        let i = ac.icon;

                        let mut b = match i {
                            ActionIcon::ImageIcon(s) => {
                                let p = String::new() + "file://" + &s;
                                let i = Image::new(ImageSource::Uri(p.into()))
                                    .texture_options(TextureOptions {
                                        magnification: TextureFilter::Nearest,
                                        minification: TextureFilter::Nearest,
                                        wrap_mode: TextureWrapMode::ClampToEdge,
                                        mipmap_mode: None,
                                    })
                                    .corner_radius(2.0);
                                Button::new(i)
                            }
                            ActionIcon::DefaultActionIcon => {
                                let i = a.icon();
                                Button::new(i)
                            }
                        };

                        let inputs = if let Some(s) = self.devices.get(&self.current_device) {
                            s.device_inputs.clone()
                        } else {
                            HashSet::new()
                        };
                        if inputs.contains(k) {
                            b = b.corner_radius(20u8);
                        }
                        let btn = ui
                            .add_sized([75.0, 75.0], b)
                            .on_hover_text_at_pointer(format!("{}: {}", k, a.help()));

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
                        s.device_inputs.clone()
                    } else {
                        HashSet::new()
                    };

                    let c = {
                        let c = self.config.blocking_lock();
                        let d = c
                            .profiles
                            .get(&c.current_profile)
                            .unwrap()
                            .get(&self.current_device)
                            .unwrap()
                            .clone();
                        d
                    };

                    let mut i = |ui: &mut Ui, b| {
                        let ac = c.key_map.get(&b).map(|c| c.clone()).unwrap_or_default();
                        let a = ac.action;
                        let i = ac.icon;

                        let mut p = match i {
                            ActionIcon::ImageIcon(s) => {
                                let p = String::new() + "file://" + &s;
                                let i = Image::new(ImageSource::Uri(p.into()))
                                    .texture_options(TextureOptions {
                                        magnification: TextureFilter::Nearest,
                                        minification: TextureFilter::Nearest,
                                        wrap_mode: TextureWrapMode::ClampToEdge,
                                        mipmap_mode: None,
                                    })
                                    .corner_radius(2.0)
                                    .max_size(vec2(64.0, 64.0));
                                Button::image(i)
                            }
                            ActionIcon::DefaultActionIcon => {
                                let i = a.icon();
                                Button::image(i)
                            }
                        };

                        if inputs.contains(&b) {
                            p = p.corner_radius(20u8);
                        }
                        let btn = ui
                            .add_sized([100.0, 231.0], p)
                            .on_hover_text_at_pointer(format!("{}: {}", b, a.help()));

                        if btn.clicked() {
                            self.enter_action_editor(b);
                        }
                    };

                    i(c1, InputKey::PedalLeft);
                    i(c2, InputKey::PedalMiddle);
                    i(c3, InputKey::PedalRight);
                });
            });

            self.draw_device_firmware_management(ui);
        });
        ui.allocate_space(ui.available_size_before_wrap());
    }

    fn draw_device_firmware_management(&mut self, ui: &mut Ui) {
        ui.allocate_ui(vec2(60.0, 231.5), |ui| {
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                let i = self.devices.get(&self.current_device).unwrap();
                ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
                    let s = match i.connected {
                        true => RichText::new(phos::PLUGS_CONNECTED)
                            .color(Color32::from_rgb(63, 192, 63)),
                        false => RichText::new(phos::PLUGS).color(Color32::from_rgb(192, 63, 63)),
                    };

                    let hint = match i.connected {
                        true => t!("help.device.connected"),
                        false => t!("help.device.disconnected"),
                    };

                    if ui.button(s).on_hover_text_at_pointer(hint).clicked() && i.connected {
                        let scmd_txs = self.scmd_txs.blocking_lock();
                        let tx = scmd_txs.get(&self.current_device).unwrap();
                        let _ = tx.send(SerialCommand::Identify);
                    }

                    ui.scope(|ui| {
                        let mut btn = Button::new(phos::DOWNLOAD);
                        let mut hint_text = t!("help.device.update");
                        if let Some(version) = &self
                            .devices
                            .get(&self.current_device)
                            .unwrap()
                            .firmware_version
                        {
                            if *version < self.available_version {
                                btn = btn.fill(Color32::from_rgb(32, 64, 200));
                                hint_text = t!("help.device.update_available");
                            }
                        }

                        if ui.add(btn).on_hover_text_at_pointer(hint_text).clicked() && i.connected
                        {
                            self.gui_tab = GuiTab::Updating;
                            self.update_progress = 0.0;
                            self.update_status = FirmwareUpdateStatus::Start;
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
                        RichText::new(format!(
                            "Firmware: {}",
                            i.firmware_version
                                .as_ref()
                                .and_then(|v| Some(v.to_string()))
                                .unwrap_or("?".into())
                        ))
                        .monospace()
                        .size(5.0),
                    );
                });
                ui.allocate_space(ui.available_size_before_wrap());
            });
        });
    }

    fn draw_device_extension_management(&mut self, ui: &mut Ui) {
        ui.allocate_ui(vec2(62.0, 231.5), |ui| {
            ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                    if ui
                        .button(phos::SIREN)
                        .on_hover_text_at_pointer(t!("help.device.rgb"))
                        .clicked()
                    {
                        self.editing_rgb = {
                            let c = self.config.blocking_lock();
                            c.profiles
                                .get(&c.current_profile)
                                .and_then(|p| p.get(&self.current_device))
                                .and_then(|d| d.rgb_profile.clone())
                                .unwrap_or(RgbProfile::default_gui_profile())
                        };
                        self.gui_tab = GuiTab::EditingRGB;
                    }
                    if ui
                        .button(phos::MONITOR)
                        .on_hover_text_at_pointer(t!("help.device.screen"))
                        .clicked()
                    {
                        self.editing_screen = {
                            let c = self.config.blocking_lock();
                            c.profiles
                                .get(&c.current_profile)
                                .and_then(|p| p.get(&self.current_device))
                                .and_then(|d| d.screen_profile.clone())
                                .unwrap_or(ScreenProfile::default_profile())
                        };
                        self.gui_tab = GuiTab::EditingScreen;
                    }
                });
                ui.allocate_space(ui.available_size_before_wrap());
            });
        });
    }

    pub fn draw_device_management(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(self.gui_tab == GuiTab::Device, |ui| {
            if self.device_renaming {
                let edit =
                    ui.add(TextEdit::singleline(&mut self.device_name_entry).desired_width(192.0));
                if edit.lost_focus() && self.device_name_entry.len() > 0 {
                    self.device_renaming = false;

                    let contains = self
                        .devices
                        .iter()
                        .any(|(_, d)| d.device_info.nickname == self.device_name_entry);

                    if !contains {
                        let d = self.devices.get_mut(&self.current_device).expect("");
                        d.device_info.nickname = self.device_name_entry.clone();

                        let mut conf = self.config.blocking_lock();
                        let c = conf.devices.get_mut(&self.current_device).expect("");
                        c.nickname = self.device_name_entry.clone();
                        conf.save();
                    }
                }
                if !edit.has_focus() {
                    edit.request_focus();
                }
            } else {
                let current_name = if !self.current_device.is_empty() {
                    &self
                        .devices
                        .get(&self.current_device)
                        .unwrap()
                        .device_info
                        .nickname
                } else {
                    &String::new()
                };
                ui.add_enabled_ui(self.devices.iter().count() != 0, |ui| {
                    let mut devices = self.devices.iter().map(|v| v.clone()).collect::<Vec<_>>();
                    devices.sort_by(|a, b| a.1.device_info.nickname.cmp(&b.1.device_info.nickname));

                    ComboBox::from_id_salt("DeviceSelect")
                        .selected_text(current_name.clone())
                        .width(200.0)
                        .truncate()
                        .show_ui(ui, |ui| {
                            for (k, v) in &devices {
                                let u = ui.selectable_label(
                                    v.device_info.nickname == *current_name,
                                    v.device_info.nickname.clone(),
                                );
                                if u.clicked() {
                                    self.current_device = k.to_string();
                                }
                            }
                        })
                        .response
                        .on_hover_text_at_pointer(t!("help.device.select"));
                });
            }

            ui.add_enabled_ui(!self.device_renaming, |ui| {
                if self.devices.keys().len() <= 0 {
                    ui.disable();
                }

                let edit_btn = ui
                    .button(RichText::new(phos::NOTE_PENCIL))
                    .on_hover_text_at_pointer(t!("help.device.edit_name"));
                if edit_btn.clicked() {
                    self.device_renaming = true;
                    self.device_name_entry.replace_with(
                        &self
                            .devices
                            .get(&self.current_device)
                            .unwrap()
                            .device_info
                            .nickname,
                    );
                }

                ui.scope(|ui| {
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::RED;

                    let delete_btn = ui
                        .button(RichText::new(phos::TRASH))
                        .on_hover_text_at_pointer(t!("help.device.forget"));

                    if delete_btn.clicked() {
                        let old_device = self.current_device.clone();
                        self.devices.remove(&old_device);
                        self.current_device =
                            self.devices.keys().next().unwrap_or(&String::new()).into();

                        let mut conf = self.config.blocking_lock();
                        conf.devices.remove(&old_device);
                        for (_, p) in conf.profiles.iter_mut() {
                            p.remove_entry(&old_device);
                        }
                        conf.save();

                        // we don't disconnect over serial because otherwise it would just immediately reconnect
                    }
                });
            })
        });
    }
}
