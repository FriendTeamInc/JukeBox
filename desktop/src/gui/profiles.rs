use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use eframe::egui::{Color32, ComboBox, RichText, TextBuffer, TextEdit, Ui};
use egui_phosphor::regular as phos;
use jukebox_util::{peripheral::DeviceType, rgb::RgbProfile, screen::ScreenProfile};
use uuid::Uuid;

use crate::{
    actions::{meta::MetaSwitchProfile, types::ActionMap},
    config::{DeviceConfig, ProfileConfig},
    serial::SerialCommand,
};

use super::gui::{GuiTab, JukeBoxGui};

impl JukeBoxGui {
    pub fn draw_profile_management(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(self.gui_tab == GuiTab::Device, |ui| {
            // Profile select/edit
            if self.profile_renaming {
                let edit =
                    ui.add(TextEdit::singleline(&mut self.profile_name_entry).desired_width(142.0));
                if edit.lost_focus() && self.profile_name_entry.len() > 0 {
                    self.profile_renaming = false;

                    let contains = {
                        let conf = self.config.blocking_lock();
                        let profile_name_list: HashSet<String> = conf
                            .profiles
                            .values()
                            .map(|p| p.profile_name.clone())
                            .collect();
                        profile_name_list.contains(&self.profile_name_entry)
                    };

                    if !contains && self.profile_name_entry.chars().count() <= 18 {
                        {
                            let mut conf = self.config.blocking_lock();

                            let p = conf.current_profile.clone();
                            let current_profile = conf.profiles.get_mut(&p).unwrap();
                            current_profile.profile_name = self.profile_name_entry.clone();

                            conf.save();
                        }

                        let device = self.current_device.clone();
                        self.set_device_profile_name(&device);
                    }
                }
                if !edit.has_focus() {
                    edit.request_focus();
                }
            } else {
                let (profiles, current) = {
                    let conf = self.config.blocking_lock();

                    let mut profiles: Vec<_> = conf
                        .profiles
                        .iter()
                        .map(|(k, v)| (k.clone(), v.profile_name.clone()))
                        .collect();
                    profiles.sort_by(|a, b| a.1.cmp(&b.1));
                    let current = conf.current_profile.clone();

                    (profiles, current)
                };
                ComboBox::from_id_salt("ProfileSelect")
                    .selected_text(current.clone())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for k in &profiles {
                            let u = ui.selectable_label(*k.0 == current, k.1.clone());
                            if u.clicked() {
                                {
                                    let mut conf = self.config.blocking_lock();
                                    conf.current_profile = k.0.clone();
                                    conf.save();
                                }

                                let device = self.current_device.clone();
                                self.set_device_profile(&device);
                            }
                        }
                    })
                    .response
                    .on_hover_text_at_pointer(t!("help.profile.select"));
            }

            // Profile management
            ui.add_enabled_ui(!self.profile_renaming, |ui| {
                let new_btn = ui
                    .button(RichText::new(phos::PLUS_CIRCLE))
                    .on_hover_text_at_pointer(t!("help.profile.new"));
                if new_btn.clicked() {
                    let mut conf = self.config.blocking_lock();
                    let mut idx = conf.profiles.keys().len() + 1;
                    let profile_name_list: HashSet<String> = conf
                        .profiles
                        .values()
                        .map(|p| p.profile_name.clone())
                        .collect();
                    let name = loop {
                        let name: String = t!("profile_name_new", idx = idx).into();
                        if !profile_name_list.contains(&name) {
                            break name;
                        }
                        idx += 1;
                    };
                    let mut m = HashMap::new();
                    for (d, t) in &self.devices {
                        let device_type = t.device_info.device_type;
                        let rgb_profile = match device_type {
                            DeviceType::KeyPad => Some(RgbProfile::default_gui_profile()),
                            _ => None,
                        };
                        let screen_profile = match device_type {
                            DeviceType::KeyPad => Some(ScreenProfile::default_profile()),
                            _ => None,
                        };

                        m.insert(
                            d.clone(),
                            DeviceConfig {
                                key_map: ActionMap::default_action_config(
                                    t.device_info.device_type.into(),
                                ),
                                rgb_profile: rgb_profile.clone(),
                                screen_profile: screen_profile.clone(),
                            },
                        );
                    }
                    let p = ProfileConfig {
                        profile_name: name,
                        device_configs: m,
                    };
                    let uuid = Uuid::new_v4().to_string();
                    conf.profiles.insert(uuid.clone(), p);
                    conf.current_profile = uuid;
                    conf.save();
                    drop(conf);

                    let devices: Vec<_> = self.devices.keys().cloned().collect();
                    for k in devices {
                        self.set_device_profile(&k);
                    }
                }

                let edit_btn = ui
                    .button(RichText::new(phos::NOTE_PENCIL))
                    .on_hover_text_at_pointer(t!("help.profile.edit_name"));
                if edit_btn.clicked() {
                    let conf = self.config.blocking_lock();
                    self.profile_renaming = true;
                    let profile_name = &conf
                        .profiles
                        .get(&conf.current_profile)
                        .unwrap()
                        .profile_name;
                    self.profile_name_entry.replace_with(profile_name);
                }

                let dupe_btn = ui
                    .button(RichText::new(phos::COPY_SIMPLE))
                    .on_hover_text_at_pointer(t!("help.profile.duplicate"));
                if dupe_btn.clicked() {
                    let mut conf = self.config.blocking_lock();
                    let mut idx = conf.profiles.keys().len() + 1;
                    let name = loop {
                        let name: String = t!("profile_name_new", idx = idx).into();
                        if !conf.profiles.contains_key(&name) {
                            break name;
                        }
                        idx += 1;
                    };
                    let duped_profile = conf.profiles.get(&conf.current_profile).unwrap().clone();
                    conf.profiles.insert(name.clone(), duped_profile);
                    conf.current_profile = name;
                    conf.save();
                    drop(conf);

                    let devices: Vec<_> = self.devices.keys().cloned().collect();
                    for k in devices {
                        self.set_device_profile(&k);
                    }
                }

                if self.config.blocking_lock().profiles.keys().len() <= 1 {
                    ui.disable();
                }
                ui.scope(|ui| {
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::RED;

                    let delete_btn = ui
                        .button(RichText::new(phos::TRASH))
                        .on_hover_text_at_pointer(t!("help.profile.delete"));

                    // TODO: add confirmation dialogue for deleting profile
                    if delete_btn.clicked() {
                        let mut conf = self.config.blocking_lock();
                        let old_profile = conf.current_profile.clone();
                        conf.profiles.remove(&old_profile);
                        conf.current_profile = conf.profiles.keys().next().unwrap().clone();

                        for k in conf
                            .profiles
                            .values_mut()
                            .flat_map(|p| p.device_configs.values_mut())
                            .flat_map(|d| d.key_map.values_mut())
                        {
                            if k.action.is::<MetaSwitchProfile>() {
                                let msp = k.action.downcast_ref::<MetaSwitchProfile>().unwrap();
                                if msp.profile == old_profile {
                                    k.action = Arc::new(MetaSwitchProfile {
                                        profile: self.profile_name_entry.clone(),
                                    });
                                }
                            }
                        }

                        conf.save();
                        drop(conf);

                        let devices: Vec<_> = self.devices.keys().cloned().collect();
                        for k in devices {
                            self.set_device_profile(&k);
                        }
                    }
                });
            });
        });
    }

    pub fn set_device_profile(&mut self, device_uid: &String) {
        self.set_device_rgb(device_uid);
        self.set_device_screen(device_uid);
        self.set_device_action_icons(device_uid);
        self.set_device_hardware_input(device_uid);
        self.set_device_profile_name(device_uid);
    }

    pub fn set_device_profile_name(&mut self, device_uid: &String) {
        if self
            .devices
            .get(device_uid)
            .map(|d| d.device_info.device_type)
            .unwrap_or(DeviceType::Unknown)
            != DeviceType::KeyPad
        {
            return;
        }

        if self
            .devices
            .get(device_uid)
            .map(|d| d.connected)
            .unwrap_or(false)
        {
            let c = self.config.blocking_lock();
            let txs = self.scmd_txs.blocking_lock();
            if let Some(tx) = txs.get(device_uid) {
                let p = c.current_profile.clone();
                let profile = c.profiles.get(&p).unwrap();
                let _ = tx.send(SerialCommand::SetProfileName(profile.profile_name.clone()));
            }
        }
    }
}
