use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{
    vec2, Align, Color32, ComboBox, Layout, Response, RichText, ScrollArea, Sense, Slider,
    TextEdit, Ui, Vec2,
};
use egui_phosphor::regular as phos;
use jukebox_util::{
    peripheral::DeviceType,
    rgb::{RgbProfile, RGB_BREATHE_COLOR_COUNT_MAX, RGB_WAVE_COLOR_COUNT_MAX},
};

use crate::serial::SerialCommand;

use super::gui::JukeBoxGui;

impl JukeBoxGui {
    fn calculate_rgb888_from_hex_string(s: String) -> Option<(u8, u8, u8)> {
        let s = s.trim().trim_start_matches('#');
        if s.len() != 6 {
            None
        } else {
            u32::from_str_radix(&s, 16)
                .ok()
                .map(u32::to_be_bytes)
                .map(|[_, r, g, b]| (r, g, b))
        }
    }

    fn draw_rgb888_editor(ui: &mut Ui, color: &mut (u8, u8, u8)) {
        let mut hex = format!("#{:02X}{:02X}{:02X}", color.0, color.1, color.2);

        ui.horizontal(|ui| {
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.add(Slider::new(&mut color.0, 0..=255).prefix("R: "));
                ui.add(Slider::new(&mut color.1, 0..=255).prefix("G: "));
                ui.add(Slider::new(&mut color.2, 0..=255).prefix("B: "));
            });

            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                Self::draw_rgb_preview(
                    ui,
                    Color32::from_rgb(color.0, color.1, color.2),
                    vec2(58.0, 38.0),
                );

                let r = ui.add(TextEdit::singleline(&mut hex).desired_width(50.0));
                if r.changed() {
                    if let Some((r, g, b)) = Self::calculate_rgb888_from_hex_string(hex) {
                        color.0 = r;
                        color.1 = g;
                        color.2 = b;
                    }
                }
            });
        });
    }

    pub fn draw_rgb_preview(ui: &mut Ui, color: Color32, size: Vec2) -> Response {
        let (rect, response) = ui.allocate_exact_size(size, Sense::empty());

        if ui.is_rect_visible(rect) {
            let visuals = ui.visuals().widgets.noninteractive;
            let rect = rect.expand(visuals.expansion);

            let corner_radius = visuals.corner_radius.at_least(8);
            ui.painter().rect_filled(rect, corner_radius, color);
        }

        response
    }

    pub fn draw_edit_rgb(&mut self, ui: &mut Ui) {
        ui.label(t!("rgb.title"));
        let rgb_defaults = [
            (
                RgbProfile::Off,
                t!("rgb.off.title"),
                t!("rgb.off.description"),
            ),
            (
                RgbProfile::default_static_solid(),
                t!("rgb.static_solid.title"),
                t!("rgb.static_solid.description"),
            ),
            (
                RgbProfile::default_static_per_key(),
                t!("rgb.static_per_key.title"),
                t!("rgb.static_per_key.description"),
            ),
            (
                RgbProfile::default_wave(),
                t!("rgb.wave.title"),
                t!("rgb.wave.description"),
            ),
            (
                RgbProfile::default_breathe(),
                t!("rgb.breathe.title"),
                t!("rgb.breathe.description"),
            ),
            (
                RgbProfile::default_rainbow_solid(),
                t!("rgb.rainbow_solid.title"),
                t!("rgb.rainbow_solid.description"),
            ),
            (
                RgbProfile::default_rainbow_wave(),
                t!("rgb.rainbow_wave.title"),
                t!("rgb.rainbow_wave.description"),
            ),
        ];

        ui.horizontal(|ui| {
            ComboBox::from_id_salt("RGBSelect")
                .selected_text(rgb_defaults[self.editing_rgb.get_type() as usize].1.clone())
                .width(200.0)
                .truncate()
                .show_ui(ui, |ui| {
                    for (i, t, _) in &rgb_defaults {
                        if ui
                            .selectable_label(
                                self.editing_rgb.get_type() == i.get_type(),
                                t.clone(),
                            )
                            .clicked()
                        {
                            self.editing_rgb = i.clone();
                        }
                    }
                })
                .response
                .on_hover_text_at_pointer(t!("help.rgb.select"));

            ui.add_enabled_ui(self.is_rgb_changed(), |ui| {
                if ui
                    .button(RichText::new(phos::FLOPPY_DISK))
                    .on_hover_text_at_pointer(t!("help.rgb.save"))
                    .clicked()
                {
                    self.save_rgb();
                }
            });
        });

        ui.label(rgb_defaults[self.editing_rgb.get_type() as usize].2.clone());
        ui.label("");

        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.vertical(|ui| {
                ScrollArea::vertical().max_height(164.0).show(ui, |ui| {
                    ui.allocate_exact_size(vec2(250.0, 0.0), Sense::empty());
                    match self.editing_rgb {
                        RgbProfile::Off => {}
                        RgbProfile::StaticSolid {
                            mut brightness,
                            mut color,
                        } => {
                            ui.label(t!("rgb.brightness"));
                            ui.add(Slider::new(&mut brightness, 0..=100));

                            ui.label(t!("rgb.static_solid.select_color"));
                            Self::draw_rgb888_editor(ui, &mut color);

                            self.editing_rgb = RgbProfile::StaticSolid { brightness, color };
                        }
                        RgbProfile::StaticPerKey {
                            mut brightness,
                            mut colors,
                        } => {
                            ui.label(t!("rgb.brightness"));
                            ui.add(Slider::new(&mut brightness, 0..=100));
                            ui.label("");

                            ui.horizontal(|ui| {
                                ui.label(t!("rgb.static_per_key.select_color"));
                                ComboBox::from_id_salt("RGBSelectKey")
                                    .selected_text(format!(
                                        "Key {}",
                                        self.editing_rgb_key_index + 1
                                    ))
                                    .width(75.0)
                                    .truncate()
                                    .show_ui(ui, |ui| {
                                        for (i, _) in colors.iter().enumerate() {
                                            if ui
                                                .selectable_label(
                                                    self.editing_rgb_key_index == i,
                                                    format!("Key {}", i + 1),
                                                )
                                                .clicked()
                                            {
                                                self.editing_rgb_key_index = i;
                                            }
                                        }
                                    })
                                    .response
                                    .on_hover_text_at_pointer(t!("help.rgb.select_key"));
                            });

                            ui.label("");

                            Self::draw_rgb888_editor(ui, &mut colors[self.editing_rgb_key_index]);

                            self.editing_rgb = RgbProfile::StaticPerKey { brightness, colors }
                        }
                        RgbProfile::Wave {
                            mut brightness,
                            mut speed,
                            mut speed_x,
                            mut speed_y,
                            mut color_count,
                            mut colors,
                        } => {
                            ui.label(t!("rgb.brightness"));
                            ui.add(Slider::new(&mut brightness, 0..=100));

                            ui.label(t!("rgb.wave.speed"));
                            ui.add(Slider::new(&mut speed, -100..=100));

                            ui.label(t!("rgb.wave.speed_x"));
                            ui.add(Slider::new(&mut speed_x, -100..=100));

                            ui.label(t!("rgb.wave.speed_y"));
                            ui.add(Slider::new(&mut speed_y, -100..=100));

                            ui.label(t!("rgb.wave.select_color"));
                            ui.label("");
                            let mut delete_idx = None;
                            for i in 0..color_count {
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}.", i + 1));
                                    ui.add_enabled_ui(color_count > 1, |ui| {
                                        if ui
                                            .button(phos::TRASH)
                                            .on_hover_text_at_pointer(t!("rgb.wave.delete_color"))
                                            .clicked()
                                        {
                                            delete_idx = Some(i);
                                        }
                                    });
                                });
                                Self::draw_rgb888_editor(ui, &mut colors[i as usize]);
                                ui.label("");
                            }
                            if let Some(x) = delete_idx {
                                for i in x..color_count - 1 {
                                    colors[i as usize] = colors[(i + 1) as usize];
                                }
                                color_count -= 1;
                            }

                            ui.add_enabled_ui(color_count < RGB_WAVE_COLOR_COUNT_MAX as u8, |ui| {
                                if ui
                                    .button("+")
                                    .on_hover_text_at_pointer(t!("rgb.wave.add_color"))
                                    .clicked()
                                {
                                    color_count += 1;
                                    colors[(color_count - 1) as usize] = (0, 0, 0);
                                }
                            });

                            self.editing_rgb = RgbProfile::Wave {
                                brightness,
                                speed,
                                speed_x,
                                speed_y,
                                color_count,
                                colors,
                            };
                        }
                        RgbProfile::Breathe {
                            mut brightness,
                            mut hold_time,
                            mut trans_time,
                            mut color_count,
                            mut colors,
                        } => {
                            ui.label(t!("rgb.brightness"));
                            ui.add(Slider::new(&mut brightness, 0..=100));

                            ui.label(t!("rgb.breathe.hold_time"));
                            ui.add(Slider::new(&mut hold_time, 0..=255));

                            ui.label(t!("rgb.breathe.trans_time"));
                            ui.add(Slider::new(&mut trans_time, 0..=255));

                            ui.label(t!("rgb.breathe.select_color"));
                            ui.label("");
                            let mut delete_idx = None;
                            for i in 0..color_count {
                                ui.horizontal(|ui| {
                                    ui.label(format!("{}.", i + 1));
                                    ui.add_enabled_ui(color_count > 1, |ui| {
                                        if ui
                                            .button(phos::TRASH)
                                            .on_hover_text_at_pointer(t!(
                                                "rgb.breathe.delete_color"
                                            ))
                                            .clicked()
                                        {
                                            delete_idx = Some(i);
                                        }
                                    });
                                });
                                Self::draw_rgb888_editor(ui, &mut colors[i as usize]);
                                ui.label("");
                            }
                            if let Some(x) = delete_idx {
                                for i in x..color_count - 1 {
                                    colors[i as usize] = colors[(i + 1) as usize];
                                }
                                color_count -= 1;
                            }

                            ui.add_enabled_ui(
                                color_count < RGB_BREATHE_COLOR_COUNT_MAX as u8,
                                |ui| {
                                    if ui
                                        .button("+")
                                        .on_hover_text_at_pointer(t!("rgb.breathe.add_color"))
                                        .clicked()
                                    {
                                        color_count += 1;
                                        colors[(color_count - 1) as usize] = (0, 0, 0);
                                    }
                                },
                            );

                            self.editing_rgb = RgbProfile::Breathe {
                                brightness,
                                hold_time,
                                trans_time,
                                color_count,
                                colors,
                            };
                        }
                        RgbProfile::RainbowSolid {
                            mut brightness,
                            mut speed,
                            mut saturation,
                            mut value,
                        } => {
                            ui.label(t!("rgb.brightness"));
                            ui.add(Slider::new(&mut brightness, 0..=100));

                            ui.label(t!("rgb.rainbow_solid.speed"));
                            ui.add(Slider::new(&mut speed, -100..=100));

                            ui.label(t!("rgb.saturation"));
                            ui.add(Slider::new(&mut saturation, 0..=100));

                            ui.label(t!("rgb.value"));
                            ui.add(Slider::new(&mut value, 0..=100));

                            self.editing_rgb = RgbProfile::RainbowSolid {
                                brightness,
                                speed,
                                saturation,
                                value,
                            };
                        }
                        RgbProfile::RainbowWave {
                            mut brightness,
                            mut speed,
                            mut speed_x,
                            mut speed_y,
                            mut saturation,
                            mut value,
                        } => {
                            ui.label(t!("rgb.brightness"));
                            ui.add(Slider::new(&mut brightness, 0..=100));

                            ui.label(t!("rgb.rainbow_wave.speed"));
                            ui.add(Slider::new(&mut speed, -100..=100));

                            ui.label(t!("rgb.rainbow_wave.speed_x"));
                            ui.add(Slider::new(&mut speed_x, -100..=100));

                            ui.label(t!("rgb.rainbow_wave.speed_y"));
                            ui.add(Slider::new(&mut speed_y, -100..=100));

                            ui.label(t!("rgb.saturation"));
                            ui.add(Slider::new(&mut saturation, 0..=100));

                            ui.label(t!("rgb.value"));
                            ui.add(Slider::new(&mut value, 0..=100));

                            self.editing_rgb = RgbProfile::RainbowWave {
                                brightness,
                                speed,
                                speed_x,
                                speed_y,
                                saturation,
                                value,
                            };
                        }
                    }
                });
            });

            ui.centered_and_justified(|ui| {
                let t = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_micros()
                    % 1_000_000_000;
                let buf = self.editing_rgb.calculate_matrix(t as u64);
                ui.vertical(|ui| {
                    ui.allocate_exact_size(vec2(0.0, 10.0), Sense::empty());
                    for y in 0..3 {
                        ui.horizontal(|ui| {
                            ui.allocate_exact_size(vec2(3.0, 45.0), Sense::empty());
                            for x in 0..4 {
                                let i = x + y * 4;
                                let c = buf[i];

                                Self::draw_rgb_preview(
                                    ui,
                                    Color32::from_rgb(c.0, c.1, c.2),
                                    vec2(40.0, 40.0),
                                );
                            }
                        });
                    }
                });
            });
        });

        ui.allocate_space(ui.available_size_before_wrap());
    }

    pub fn set_device_rgb(&mut self, device_uid: &String) {
        if self
            .devices
            .get(device_uid)
            .map(|d| d.device_info.device_type)
            .unwrap_or(DeviceType::Unknown)
            != DeviceType::KeyPad
        {
            return;
        }

        let rgb_profile = {
            let c = self.config.blocking_lock();
            let p = c.current_profile.clone();
            c.profiles
                .get(&p)
                .and_then(|d| d.get(device_uid))
                .and_then(|p| p.rgb_profile.clone())
                .unwrap_or(RgbProfile::Off)
        };

        if self
            .devices
            .get(device_uid)
            .map(|d| d.connected)
            .unwrap_or(false)
        {
            let txs = self.scmd_txs.blocking_lock();
            if let Some(tx) = txs.get(device_uid) {
                let _ = tx.send(SerialCommand::SetRgbMode(rgb_profile));
            }
        }
    }

    pub fn is_rgb_changed(&self) -> bool {
        let c = self.config.blocking_lock();
        let p = c.current_profile.clone();
        c.profiles
            .get(&p)
            .and_then(|p| p.get(&self.current_device))
            .and_then(|d| d.rgb_profile.clone())
            .and_then(|rgb| Some(rgb != self.editing_rgb))
            .unwrap_or(false)
    }

    pub fn save_rgb(&mut self) {
        {
            let mut c = self.config.blocking_lock();
            let p = c.current_profile.clone();
            if let Some(profile) = c.profiles.get_mut(&p) {
                if let Some(device) = profile.get_mut(&self.current_device) {
                    device.rgb_profile = Some(self.editing_rgb.clone())
                }
            }
            c.save();
        }

        self.set_device_rgb(&self.current_device.clone());
    }
}
