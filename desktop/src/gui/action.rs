use std::collections::HashMap;

use eframe::egui::{
    scroll_area::ScrollBarVisibility, vec2, Align, Button, CollapsingHeader, Grid, Image,
    ImageSource, Layout, RichText, ScrollArea, Ui,
};
use jukebox_util::{input::KeyboardEvent, peripheral::DeviceType};

use crate::{
    actions::{
        input::{InputKeyboard, InputMouse},
        meta::AID_META_NO_ACTION,
        types::get_icon_bytes,
    },
    config::{ActionConfig, ActionIcon, DeviceConfig},
    input::InputKey,
    serial::SerialCommand,
};

use super::gui::{GuiTab, JukeBoxGui};

impl JukeBoxGui {
    pub fn enter_action_editor(&mut self, key: InputKey) {
        self.device_renaming = false;
        self.profile_renaming = false;
        self.gui_tab = GuiTab::EditingAction;
        self.editing_key = key;
        {
            let c = self.config.blocking_lock();
            if let Some(r) = c
                .profiles
                .get(&c.current_profile)
                .and_then(|p| p.get(&self.current_device))
                .and_then(|d| d.key_map.get(&self.editing_key))
            {
                self.editing_action_icon = r.icon.clone();
                self.editing_action_type = r.action.get_type();
                self.editing_action = r.action.clone();
            } else {
                self.editing_action_type = AID_META_NO_ACTION.into();
                self.editing_action = self.action_map.enum_new(self.editing_action_type.clone());
                self.editing_action_icon = ActionIcon::default();
            }
        };
    }

    fn set_device_action_icon(&mut self, device_uid: &String) {
        if self
            .devices
            .get(device_uid)
            .map(|d| d.device_info.device_type)
            .unwrap_or(DeviceType::Unknown)
            != DeviceType::KeyPad
        {
            return;
        }

        let icon = if let Some(icon) = {
            let c = self.config.blocking_lock().clone();
            c.profiles
                .clone()
                .get(&c.current_profile)
                .and_then(|d| d.get(device_uid))
                .and_then(|p| p.key_map.get(&self.editing_key))
                .map(|a| a.action.icon_source())
        } {
            get_icon_bytes(icon)
        } else {
            return;
        };

        let slot = self.editing_key.slot();

        if self
            .devices
            .get(device_uid)
            .map(|d| d.connected)
            .unwrap_or(false)
        {
            let txs = self.scmd_txs.blocking_lock();
            if let Some(tx) = txs.get(device_uid) {
                let _ = tx.send(SerialCommand::SetScrIcon(slot, icon));
            }
        }
    }

    fn set_device_hardware_input(&mut self, device_uid: &String) {
        let action = if let Some(action) = {
            let c = self.config.blocking_lock().clone();
            c.profiles
                .clone()
                .get(&c.current_profile)
                .and_then(|d| d.get(device_uid))
                .and_then(|p| p.key_map.get(&self.editing_key))
                .map(|a| a.action.clone())
        } {
            action
        } else {
            return;
        };

        let slot = self.editing_key.slot();

        if self
            .devices
            .get(device_uid)
            .map(|d| d.connected)
            .unwrap_or(false)
        {
            let txs = self.scmd_txs.blocking_lock();
            if let Some(tx) = txs.get(device_uid) {
                let _ = if let Some(kb) = action.downcast_ref::<InputKeyboard>() {
                    tx.send(SerialCommand::SetKeyboardInput(
                        slot,
                        kb.get_keyboard_event(),
                    ))
                } else if let Some(mouse) = action.downcast_ref::<InputMouse>() {
                    tx.send(SerialCommand::SetMouseInput(slot, mouse.get_mouse_event()))
                } else {
                    tx.send(SerialCommand::SetKeyboardInput(
                        slot,
                        KeyboardEvent::empty_event(),
                    ))
                };
            }
        }
    }

    pub fn save_action_and_exit(&mut self) {
        // TODO: have config validate input?

        {
            let mut c = self.config.blocking_lock();
            let current_profile = c.current_profile.clone();
            let profile = c.profiles.get_mut(&current_profile).unwrap();
            if let Some(d) = profile.get_mut(&self.current_device) {
                d.key_map.insert(
                    self.editing_key.clone(),
                    ActionConfig {
                        action: self.editing_action.clone(),
                        icon: self.editing_action_icon.clone(),
                    },
                );
            } else {
                let mut d = HashMap::new();
                d.insert(
                    self.editing_key.clone(),
                    ActionConfig {
                        action: self.editing_action.clone(),
                        icon: self.editing_action_icon.clone(),
                    },
                );
                profile.insert(
                    self.current_device.clone(),
                    DeviceConfig {
                        key_map: d,
                        rgb_profile: None,
                    },
                );
            }
            c.save();
        }

        self.set_device_action_icon(&self.current_device.clone());

        self.set_device_hardware_input(&self.current_device.clone());
    }

    pub fn draw_edit_action(&mut self, ui: &mut Ui) {
        ui.columns_const(|[c1, c2]| {
            c1.horizontal(|ui| {
                let test_btn = match &self.editing_action_icon {
                    ActionIcon::ImageIcon(s) => {
                        let i = Image::new(ImageSource::Uri(s.into()))
                            .corner_radius(2.0)
                            .max_size(vec2(64.0, 64.0));
                        Button::image(i)
                    }
                    ActionIcon::DefaultActionIcon => {
                        let i = self.editing_action.icon();
                        Button::image(i)
                    }
                };
                let test_btn = ui
                    .add_sized([50.0, 50.0], test_btn)
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
                // ui.vertical(|ui| {
                //     ui.allocate_space(vec2(0.0, 2.0));
                //     if ui
                //         .button(RichText::new(phos::FOLDER))
                //         .on_hover_text_at_pointer(t!("help.action.image_icon"))
                //         .clicked()
                //     {
                //         log::info!("TODO: choose image icon");
                //     }
                //     if ui
                //         .button(RichText::new(phos::SEAL))
                //         .on_hover_text_at_pointer(t!("help.action.glyph_icon"))
                //         .clicked()
                //     {
                //         log::info!("TODO: choose glyph icon");
                //     }
                // });
                ui.with_layout(
                    Layout::centered_and_justified(eframe::egui::Direction::TopDown)
                        .with_cross_justify(false),
                    |ui| {
                        ui.label(RichText::new(self.editing_action.help()).size(10.0));
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
                            self.editing_action.edit_ui(
                                ui,
                                &self.current_device,
                                self.editing_key,
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

    fn draw_action_list(&mut self, ui: &mut Ui) {
        for (header, options) in self.action_map.ui_list() {
            CollapsingHeader::new(RichText::new(header).strong())
                .default_open(true)
                .show(ui, |ui| {
                    for (action_type, label) in options {
                        if ui
                            .selectable_value(&mut self.editing_action_type, action_type, label)
                            .changed()
                        {
                            self.reset_editing_action();
                        };
                    }
                });
            ui.end_row();
        }
    }

    fn reset_editing_action(&mut self) {
        self.editing_action = self.action_map.enum_new(self.editing_action_type.clone());
    }
}
