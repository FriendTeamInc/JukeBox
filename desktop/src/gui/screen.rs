use eframe::egui::{
    vec2, Align, Color32, ComboBox, Layout, RichText, ScrollArea, Sense, Slider, TextEdit, Ui,
};
use egui_phosphor::regular as phos;
use jukebox_util::{
    color::{combine_to_rgb565, map_565_to_888, split_to_rgb565},
    peripheral::DeviceType,
    screen::ScreenProfile,
};

use crate::serial::SerialCommand;

use super::gui::JukeBoxGui;

impl JukeBoxGui {
    fn calculate_rgb565_from_hex_string(s: String) -> Option<(u8, u8, u8)> {
        let s = s.trim().trim_start_matches('#');
        if s.len() != 6 {
            None
        } else {
            u16::from_str_radix(&s, 16).ok().map(split_to_rgb565)
        }
    }

    fn draw_rgb565_editor(ui: &mut Ui, color: &mut (u8, u8, u8)) {
        let mut hex = format!("#{:04X}", combine_to_rgb565(color.0, color.1, color.2));

        ui.horizontal(|ui| {
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.add(Slider::new(&mut color.0, 0..=31).prefix("R: "));
                ui.add(Slider::new(&mut color.1, 0..=63).prefix("G: "));
                ui.add(Slider::new(&mut color.2, 0..=31).prefix("B: "));
            });

            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                let c2 = map_565_to_888(*color);

                Self::draw_rgb_preview(ui, Color32::from_rgb(c2.0, c2.1, c2.2), vec2(52.0, 38.0));

                let r = ui.add(TextEdit::singleline(&mut hex).desired_width(45.0));
                if r.changed() {
                    if let Some((r, g, b)) = Self::calculate_rgb565_from_hex_string(hex) {
                        color.0 = r;
                        color.1 = g;
                        color.2 = b;
                    }
                }
            });
        });
    }

    pub fn draw_edit_screen(&mut self, ui: &mut Ui) {
        ui.label(t!("screen.title"));
        let screen_defaults = [
            (
                ScreenProfile::Off,
                t!("screen.off.title"),
                t!("screen.off.description"),
            ),
            (
                ScreenProfile::default_display_keys(),
                t!("screen.display_keys.title"),
                t!("screen.display_keys.description"),
            ),
            (
                ScreenProfile::default_display_stats(),
                t!("screen.display_stats.title"),
                t!("screen.display_stats.description"),
            ),
        ];

        ui.horizontal(|ui| {
            ComboBox::from_id_salt("ScreenProfileSelect")
                .selected_text(
                    screen_defaults[self.editing_screen.get_type() as usize]
                        .1
                        .clone(),
                )
                .width(200.0)
                .truncate()
                .show_ui(ui, |ui| {
                    for (i, t, _) in &screen_defaults {
                        if ui
                            .selectable_label(
                                self.editing_screen.get_type() == i.get_type(),
                                t.clone(),
                            )
                            .clicked()
                        {
                            self.editing_screen = i.clone();
                        }
                    }
                })
                .response
                .on_hover_text_at_pointer(t!("help.screen.select"));

            ui.add_enabled_ui(self.is_screen_changed(), |ui| {
                if ui
                    .button(RichText::new(phos::FLOPPY_DISK))
                    .on_hover_text_at_pointer(t!("help.screen.save"))
                    .clicked()
                {
                    self.save_screen();
                }
            });
        });

        ui.label(
            screen_defaults[self.editing_screen.get_type() as usize]
                .2
                .clone(),
        );
        ui.label("");

        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.vertical(|ui| {
                ScrollArea::vertical().max_height(164.0).show(ui, |ui| {
                    ui.allocate_exact_size(vec2(250.0, 0.0), Sense::empty());
                    match self.editing_screen {
                        ScreenProfile::Off => {}
                        ScreenProfile::DisplayKeys {
                            mut brightness,
                            background_color,
                            text_color,
                        } => {
                            ui.label(t!("screen.brightness"));
                            ui.add(Slider::new(&mut brightness, 0..=100));

                            ui.label(t!("screen.select_background_color"));
                            let mut c = split_to_rgb565(background_color);
                            Self::draw_rgb565_editor(ui, &mut c);
                            let background_color = combine_to_rgb565(c.0, c.1, c.2);

                            ui.label(t!("screen.select_text_color"));
                            let mut c = split_to_rgb565(text_color);
                            Self::draw_rgb565_editor(ui, &mut c);
                            let text_color = combine_to_rgb565(c.0, c.1, c.2);

                            self.editing_screen = ScreenProfile::DisplayKeys {
                                brightness,
                                background_color,
                                text_color,
                            };
                        }
                        ScreenProfile::DisplayStats {
                            mut brightness,
                            background_color,
                            text_color,
                        } => {
                            ui.label(t!("screen.brightness"));
                            ui.add(Slider::new(&mut brightness, 0..=100));

                            ui.label(t!("screen.select_background_color"));
                            let mut c = split_to_rgb565(background_color);
                            Self::draw_rgb565_editor(ui, &mut c);
                            let background_color = combine_to_rgb565(c.0, c.1, c.2);

                            ui.label(t!("screen.select_text_color"));
                            let mut c = split_to_rgb565(text_color);
                            Self::draw_rgb565_editor(ui, &mut c);
                            let text_color = combine_to_rgb565(c.0, c.1, c.2);

                            self.editing_screen = ScreenProfile::DisplayStats {
                                brightness,
                                background_color,
                                text_color,
                            };
                        }
                    }
                });
            });
        });

        // TODO: preview?

        ui.allocate_space(ui.available_size_before_wrap());
    }

    pub fn set_device_screen(&mut self, device_uid: &String) {
        if self
            .devices
            .get(device_uid)
            .map(|d| d.device_info.device_type)
            .unwrap_or(DeviceType::Unknown)
            != DeviceType::KeyPad
        {
            return;
        }

        let screen_profile = {
            let c = self.config.blocking_lock();
            let p = c.current_profile.clone();
            c.profiles
                .get(&p)
                .and_then(|d| d.get(device_uid))
                .and_then(|p| p.screen_profile.clone())
                .unwrap_or(ScreenProfile::Off)
        };

        if self
            .devices
            .get(device_uid)
            .map(|d| d.connected)
            .unwrap_or(false)
        {
            let txs = self.scmd_txs.blocking_lock();
            if let Some(tx) = txs.get(device_uid) {
                let _ = tx.send(SerialCommand::SetScrMode(screen_profile));
            }
        }
    }

    pub fn is_screen_changed(&self) -> bool {
        let c = self.config.blocking_lock();
        let p = c.current_profile.clone();
        c.profiles
            .get(&p)
            .and_then(|p| p.get(&self.current_device))
            .and_then(|d| d.screen_profile.clone())
            .and_then(|screen| Some(screen != self.editing_screen))
            .unwrap_or(false)
    }

    pub fn save_screen(&mut self) {
        {
            let mut c = self.config.blocking_lock();
            let p = c.current_profile.clone();
            if let Some(profile) = c.profiles.get_mut(&p) {
                if let Some(device) = profile.get_mut(&self.current_device) {
                    device.screen_profile = Some(self.editing_screen.clone())
                }
            }
            c.save();
        }

        self.set_device_screen(&self.current_device.clone());
    }
}
