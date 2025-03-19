use std::collections::HashMap;

use eframe::egui::{
    scroll_area::ScrollBarVisibility, vec2, Align, Button, CollapsingHeader, Grid, Layout,
    RichText, ScrollArea, Ui,
};
use egui_phosphor::regular as phos;

use crate::{
    config::{ActionConfig, ActionIcon, DeviceConfig},
    input::InputKey,
};

use super::gui::{GuiTab, JukeBoxGui};

impl JukeBoxGui {
    pub fn enter_action_editor(&mut self, key: InputKey) {
        self.config_renaming_device = false;
        self.config_renaming_profile = false;
        self.gui_tab = GuiTab::EditingAction;
        self.config_editing_key = key;
        {
            let c = self.config.blocking_lock();
            if let Some(r) = c
                .profiles
                .get(&c.current_profile)
                .and_then(|p| p.get(&self.current_device))
                .and_then(|d| d.key_map.get(&self.config_editing_key))
            {
                self.config_editing_action_type = r.action.get_type();
                self.config_editing_action = r.action.clone();
            } else {
                self.config_editing_action_type = "MetaNoAction".into();
                self.config_editing_action = self
                    .action_map
                    .enum_new(self.config_editing_action_type.clone());
            }
        };
    }

    pub fn save_action_and_exit(&mut self) {
        // TODO: have config validate input?
        self.gui_tab = GuiTab::Device;
        let mut c = self.config.blocking_lock();
        let current_profile = c.current_profile.clone();
        let profile = c.profiles.get_mut(&current_profile).unwrap();
        if let Some(d) = profile.get_mut(&self.current_device) {
            d.key_map.insert(
                self.config_editing_key.clone(),
                ActionConfig {
                    action: self.config_editing_action.clone(),
                    icon: ActionIcon::GlyphIcon(phos::SEAL_QUESTION.into()),
                },
            );
        } else {
            let mut d = HashMap::new();
            d.insert(
                self.config_editing_key.clone(),
                ActionConfig {
                    action: self.config_editing_action.clone(),
                    icon: ActionIcon::GlyphIcon(phos::SEAL_QUESTION.into()),
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

    pub fn draw_edit_action(&mut self, ui: &mut Ui) {
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

    fn reset_editing_action(&mut self) {
        self.config_editing_action = self
            .action_map
            .enum_new(self.config_editing_action_type.clone());
    }
}
