use std::collections::HashMap;

use eframe::egui::{ComboBox, RichText, TextBuffer, TextEdit, Ui};
use egui_phosphor::regular as phos;

use crate::config::DeviceConfig;

use super::gui::{GuiTab, JukeBoxGui};

impl JukeBoxGui {
    pub fn draw_profile_management(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(self.gui_tab == GuiTab::Device, |ui| {
            // Profile select/edit
            if self.profile_renaming {
                // TODO: this shifts everything down a bit too much, fix later
                let edit =
                    ui.add(TextEdit::singleline(&mut self.profile_name_entry).desired_width(142.0));
                if edit.lost_focus() && self.profile_name_entry.len() > 0 {
                    self.profile_renaming = false;

                    let mut conf = self.config.blocking_lock();

                    if !conf.profiles.contains_key(&self.profile_name_entry) {
                        let p = conf.current_profile.clone();
                        let c = conf.profiles.remove(&p).expect("");
                        conf.profiles.insert(self.profile_name_entry.clone(), c);
                        conf.current_profile.replace_with(&self.profile_name_entry);

                        // TODO: edit configs to reference new profile instead of wiping it
                        for (_, p) in conf.profiles.iter_mut() {
                            for (_, d) in p.iter_mut() {
                                for (_, k) in d.key_map.iter_mut() {
                                    match k.action.get_type().as_ref() {
                                        "MetaSwitchProfile" => {
                                            k.action = self
                                                .action_map
                                                .enum_new("MetaSwitchProfile".into());
                                        }
                                        "MetaCopyFromProfile" => {
                                            k.action = self
                                                .action_map
                                                .enum_new("MetaCopyFromProfile".into());
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
                                conf.current_profile = k.into();
                                conf.save();
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
                    let name = loop {
                        let name = t!("profile_name_new", idx = idx).into();
                        if !conf.profiles.contains_key(&name) {
                            break name;
                        }
                        idx += 1;
                    };
                    let mut m = HashMap::new();
                    for (d, t) in &self.devices {
                        m.insert(
                            d.clone(),
                            DeviceConfig {
                                key_map: self
                                    .action_map
                                    .default_action_config(t.device_info.device_type.into()),
                                rgb_profile: None,
                            },
                        );
                    }
                    conf.profiles.insert(name, m);
                    conf.save();
                }

                let edit_btn = ui
                    .button(RichText::new(phos::NOTE_PENCIL))
                    .on_hover_text_at_pointer(t!("help.profile.edit_name"));
                if edit_btn.clicked() {
                    let conf = self.config.blocking_lock();
                    self.profile_renaming = true;
                    self.profile_name_entry.replace_with(&conf.current_profile);
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
                            for (_, k) in d.key_map.iter_mut() {
                                match k.action.get_type().as_ref() {
                                    "MetaSwitchProfile" => {
                                        k.action =
                                            self.action_map.enum_new("MetaSwitchProfile".into());
                                    }
                                    "MetaCopyFromProfile" => {
                                        k.action =
                                            self.action_map.enum_new("MetaCopyFromProfile".into());
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
}
