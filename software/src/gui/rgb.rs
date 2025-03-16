use eframe::egui::{
    color_picker::show_color, vec2, Align, Color32, ComboBox, Layout, ScrollArea, Sense, Slider,
    TextEdit, Ui,
};
use jukebox_util::color::RgbProfile;

use super::gui::JukeBoxGui;

impl JukeBoxGui {
    fn calculate_color_from_hex_string(s: String) -> Option<(u8, u8, u8)> {
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

    fn draw_color_editor(ui: &mut Ui, color: &mut (u8, u8, u8)) {
        let mut hex = format!("{:02X}{:02X}{:02X}", color.0, color.1, color.2);

        ui.horizontal(|ui| {
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.horizontal(|ui| {
                    ui.label("R: ");
                    ui.add(Slider::new(&mut color.0, 0..=255));
                });
                ui.horizontal(|ui| {
                    ui.label("G: ");
                    ui.add(Slider::new(&mut color.1, 0..=255));
                });
                ui.horizontal(|ui| {
                    ui.label("B: ");
                    ui.add(Slider::new(&mut color.2, 0..=255));
                });
            });

            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                show_color(
                    ui,
                    Color32::from_rgb(color.0, color.1, color.2),
                    vec2(59.0, 38.0),
                );

                let r = ui.add(TextEdit::singleline(&mut hex).desired_width(50.0));
                if r.changed() {
                    if let Some((r, g, b)) = Self::calculate_color_from_hex_string(hex) {
                        color.0 = r;
                        color.1 = g;
                        color.2 = b;
                    }
                }
            });
        });
    }

    pub fn draw_edit_rgb(&mut self, ui: &mut Ui) {
        ui.label("RGB Mode:");
        let map = [
            (
                RgbProfile::Off,
                t!("rgb.profile.off.title"),
                t!("rgb.profile.off.description"),
            ),
            (
                RgbProfile::Static {
                    brightness: 25,
                    color: (204, 153, 51),
                },
                t!("rgb.profile.static.title"),
                t!("rgb.profile.static.description"),
            ),
            (
                RgbProfile::Wave {
                    brightness: 25,
                    speed_x: 50,
                    speed_y: 0,
                    color_count: 3,
                    colors: [(200, 0, 0), (0, 200, 0), (0, 0, 200), (0, 0, 0)],
                },
                t!("rgb.profile.wave.title"),
                t!("rgb.profile.wave.description"),
            ),
            (
                RgbProfile::Breathe {
                    brightness: 25,
                    hold_time: 20,
                    trans_time: 5,
                    color_count: 3,
                    colors: [(200, 0, 0), (0, 200, 0), (0, 0, 200), (0, 0, 0)],
                },
                t!("rgb.profile.breathe.title"),
                t!("rgb.profile.breathe.description"),
            ),
            (
                RgbProfile::RainbowSolid {
                    brightness: 25,
                    speed: 30,
                    saturation: 100,
                    value: 100,
                },
                t!("rgb.profile.rainbow_solid.title"),
                t!("rgb.profile.rainbow_solid.description"),
            ),
            (
                RgbProfile::RainbowWave {
                    brightness: 25,
                    speed: 100,
                    speed_x: 0,
                    speed_y: 30,
                    saturation: 100,
                    value: 100,
                },
                t!("rgb.profile.rainbow_wave.title"),
                t!("rgb.profile.rainbow_wave.description"),
            ),
        ];

        ComboBox::from_id_salt("RGBSelect")
            .selected_text(map[self.config_editing_rgb.get_type() as usize].1.clone())
            .width(200.0)
            .truncate()
            .show_ui(ui, |ui| {
                for (i, t, _) in &map {
                    let u = ui.selectable_label(
                        self.config_editing_rgb.get_type() == i.get_type(),
                        t.clone(),
                    );
                    if u.clicked() {
                        self.config_editing_rgb = i.clone();
                    }
                }
            })
            .response
            .on_hover_text_at_pointer(t!("help.device.select"));

        ui.label(map[self.config_editing_rgb.get_type() as usize].2.clone());
        ui.label("");

        ScrollArea::vertical()
            // .max_width(200.0)
            .max_height(164.0)
            .show(ui, |ui| {
                ui.allocate_exact_size(vec2(275.0, 0.0), Sense::empty());
                match self.config_editing_rgb {
                    RgbProfile::Off => {}
                    RgbProfile::Static {
                        mut brightness,
                        mut color,
                    } => {
                        ui.label(t!("rgb.brightness"));
                        ui.add(Slider::new(&mut brightness, 0..=100));

                        ui.label(t!("rgb.profile.static.select_color"));
                        Self::draw_color_editor(ui, &mut color);

                        self.config_editing_rgb = RgbProfile::Static { brightness, color };
                    }
                    RgbProfile::Wave {
                        mut brightness,
                        speed_x,
                        speed_y,
                        color_count,
                        colors,
                    } => {
                        ui.label(t!("rgb.brightness"));
                        ui.add(Slider::new(&mut brightness, 0..=100));

                        ui.label("todo!");

                        self.config_editing_rgb = RgbProfile::Wave {
                            brightness,
                            speed_x,
                            speed_y,
                            color_count,
                            colors,
                        };
                    }
                    RgbProfile::Breathe {
                        mut brightness,
                        hold_time,
                        trans_time,
                        color_count,
                        colors,
                    } => {
                        ui.label(t!("rgb.brightness"));
                        ui.add(Slider::new(&mut brightness, 0..=100));

                        ui.label("todo!");

                        self.config_editing_rgb = RgbProfile::Breathe {
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

                        ui.label(t!("rgb.profile.rainbow_solid.speed"));
                        ui.add(Slider::new(&mut speed, -100..=100));

                        ui.label(t!("rgb.saturation"));
                        ui.add(Slider::new(&mut saturation, 0..=100));

                        ui.label(t!("rgb.value"));
                        ui.add(Slider::new(&mut value, 0..=100));

                        // TODO: add preview?

                        self.config_editing_rgb = RgbProfile::RainbowSolid {
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

                        ui.label(t!("rgb.profile.rainbow_wave.speed"));
                        ui.add(Slider::new(&mut speed, -100..=100));

                        ui.label(t!("rgb.profile.rainbow_wave.speed_x"));
                        ui.add(Slider::new(&mut speed_x, -100..=100));

                        ui.label(t!("rgb.profile.rainbow_wave.speed_y"));
                        ui.add(Slider::new(&mut speed_y, -100..=100));

                        ui.label(t!("rgb.saturation"));
                        ui.add(Slider::new(&mut saturation, 0..=100));

                        ui.label(t!("rgb.value"));
                        ui.add(Slider::new(&mut value, 0..=100));

                        // TODO: add preview?

                        self.config_editing_rgb = RgbProfile::RainbowWave {
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

        ui.allocate_space(ui.available_size_before_wrap());
    }

    pub fn save_rgb_and_exit(&mut self) {
        todo!()
    }
}
