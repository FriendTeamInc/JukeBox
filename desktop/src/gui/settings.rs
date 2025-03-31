use eframe::egui::{Align, Color32, Layout, RichText, Ui};
use egui_phosphor::regular as phos;
use egui_theme_switch::global_theme_switch;

use super::gui::{GuiTab, JukeBoxGui};

const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");

impl JukeBoxGui {
    pub fn draw_settings_toggle(&mut self, ui: &mut Ui) {
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

    pub fn draw_settings_page(&mut self, ui: &mut Ui) {
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

        if ui
            .checkbox(
                &mut self.config_always_save_on_exit,
                t!("help.settings.exit_on_save"),
            )
            .changed()
        {
            let mut conf = self.config.blocking_lock();
            conf.always_save_on_exit = self.config_always_save_on_exit;
            conf.save();
        }

        ui.with_layout(Layout::bottom_up(Align::RIGHT), |ui| {
            ui.columns_const(|[c1, c2]| {
                c1.with_layout(Layout::left_to_right(Align::Max), |ui| {
                    ui.label(t!("settings.copyright"));
                });
                c2.with_layout(Layout::right_to_left(Align::Max), |ui| {
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
            });
        });
    }
}
