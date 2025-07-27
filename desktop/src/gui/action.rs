use std::path::PathBuf;

use eframe::egui::{
    scroll_area::ScrollBarVisibility, vec2, Align, Button, CollapsingHeader, Grid, Image,
    ImageSource, Layout, RichText, ScrollArea, TextureFilter, TextureOptions, TextureWrapMode, Ui,
};
use egui_phosphor::regular as phos;
use image::EncodableLayout;
use jukebox_util::peripheral::DeviceType;
use rfd::FileDialog;
use tokio::runtime::Handle;

use crate::{
    actions::{
        action::send_input_event,
        meta::AID_META_NO_ACTION,
        types::{get_icon_bytes, get_icon_cache, ActionError},
    },
    config::{ActionConfig, ActionIcon, JukeBoxConfig},
    input::InputKey,
    serial::SerialCommand,
};

use super::gui::{GuiTab, JukeBoxGui};

const BMP_HEADER: &[u8] = &[
    66, 77, 122, 8, 0, 0, 0, 0, 0, 0, 122, 0, 0, 0, 108, 0, 0, 0, 32, 0, 0, 0, 32, 0, 0, 0, 1, 0,
    16, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 248, 0, 0,
    224, 7, 0, 0, 31, 0, 0, 0, 0, 0, 0, 0, 66, 71, 82, 115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0,
];

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

    pub fn set_device_action_icons(&mut self, device_uid: &String) {
        if self
            .devices
            .get(device_uid)
            .map(|d| d.device_info.device_type)
            .unwrap_or(DeviceType::Unknown)
            != DeviceType::KeyPad
            || !self
                .devices
                .get(device_uid)
                .map(|d| d.connected)
                .unwrap_or(false)
        {
            return;
        }

        let c = self.config.blocking_lock().clone();
        let p = c.profiles.clone();
        let p = p.get(&c.current_profile).and_then(|d| d.get(device_uid));

        if let Some(p) = p {
            for (k, a) in &p.key_map {
                let txs = self.scmd_txs.blocking_lock();
                if let Some(tx) = txs.get(device_uid) {
                    let slot = k.slot();
                    let icon = get_icon_bytes(&a, &mut get_icon_cache());
                    let _ = tx.send(SerialCommand::SetScrIcon(slot, icon));
                }
            }
        }
    }

    pub fn set_device_hardware_input(&mut self, device_uid: &String) {
        if self
            .devices
            .get(device_uid)
            .map(|d| d.device_info.device_type)
            .unwrap_or(DeviceType::Unknown)
            != DeviceType::KeyPad
            || !self
                .devices
                .get(device_uid)
                .map(|d| d.connected)
                .unwrap_or(false)
        {
            return;
        }

        let c = self.config.blocking_lock().clone();
        let p = c.profiles.clone();
        let p = p.get(&c.current_profile).and_then(|d| d.get(device_uid));

        if let Some(p) = p {
            for (k, a) in &p.key_map {
                let txs = self.scmd_txs.blocking_lock();
                if let Some(tx) = txs.get(device_uid) {
                    let slot = k.slot();
                    send_input_event(tx, slot, &a.action);
                }
            }
        }
    }

    fn set_device_edited_action_icon(&mut self, device_uid: &String) {
        if self
            .devices
            .get(device_uid)
            .map(|d| d.device_info.device_type)
            .unwrap_or(DeviceType::Unknown)
            != DeviceType::KeyPad
        {
            return;
        }

        let icon = if let Some(action_config) = {
            let c = self.config.blocking_lock().clone();
            c.profiles
                .clone()
                .get(&c.current_profile)
                .and_then(|d| d.get(device_uid))
                .and_then(|p| p.key_map.get(&self.editing_key))
        } {
            get_icon_bytes(action_config, &mut get_icon_cache())
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

    fn set_device_edited_hardware_input(&mut self, device_uid: &String) {
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
                send_input_event(tx, slot, &action);
            }
        }
    }

    pub fn is_action_changed(&self) -> bool {
        let c = self.config.blocking_lock();
        let current_profile = c.current_profile.clone();
        let profile = c.profiles.get(&current_profile).unwrap();
        let d = profile.get(&self.current_device).unwrap();

        if let Some(old_action) = d.key_map.get(&self.editing_key) {
            let new_action = ActionConfig {
                action: self.editing_action.clone(),
                icon: self.editing_action_icon.clone(),
            };
            new_action != *old_action
        } else {
            false
        }
    }

    pub fn save_action(&mut self) {
        // TODO: have config validate input?

        {
            let mut c = self.config.blocking_lock();
            let current_profile = c.current_profile.clone();
            let profile = c.profiles.get_mut(&current_profile).unwrap();
            let d = profile.get_mut(&self.current_device).unwrap();
            d.key_map.insert(
                self.editing_key.clone(),
                ActionConfig {
                    action: self.editing_action.clone(),
                    icon: self.editing_action_icon.clone(),
                },
            );
            c.save();
        }

        self.set_device_edited_action_icon(&self.current_device.clone());

        self.set_device_edited_hardware_input(&self.current_device.clone());
    }

    pub fn draw_edit_action(&mut self, ui: &mut Ui) {
        ui.columns_const(|[c1, c2]| {
            c1.horizontal(|ui| {
                let test_btn = match &self.editing_action_icon {
                    ActionIcon::ImageIcon(s) => {
                        let p = String::new() + "file://" + s;
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
                        let i = self.editing_action.icon();
                        Button::image(i)
                    }
                };
                let test_btn = ui
                    .add_sized([60.0, 60.0], test_btn)
                    .on_hover_text_at_pointer(t!("help.action.test_input"));
                if test_btn.clicked() {
                    let h = Handle::current();
                    if let Err(press_err) = h.block_on(async {
                        self.editing_action
                            .on_press(&self.current_device, self.editing_key, self.config.clone())
                            .await
                    }) {
                        self.action_errors.push_back(press_err);
                    }
                    if let Err(release_err) = h.block_on(async {
                        self.editing_action
                            .on_release(&self.current_device, self.editing_key, self.config.clone())
                            .await
                    }) {
                        self.action_errors.push_back(release_err);
                    }
                }
                ui.vertical(|ui| {
                    ui.add_enabled_ui(self.is_action_changed(), |ui| {
                        if ui
                            .button(RichText::new(phos::FLOPPY_DISK))
                            .on_hover_text_at_pointer(t!("help.action.save"))
                            .clicked()
                        {
                            self.save_action();
                        }
                    });
                    if ui
                        .button(RichText::new(phos::FOLDER_OPEN))
                        .on_hover_text_at_pointer(t!("help.action.image_icon"))
                        .clicked()
                    {
                        if let Some(f) = FileDialog::new()
                            .add_filter("PNG Image", &["png"])
                            .pick_file()
                        {
                            match self.load_custom_icon(f) {
                                Ok(i) => {
                                    self.editing_action_icon =
                                        ActionIcon::ImageIcon(i.to_string_lossy().to_string())
                                }
                                Err(e) => self.action_errors.push_back(e),
                            }
                        }
                    }
                    ui.add_enabled_ui(
                        self.editing_action_icon != ActionIcon::DefaultActionIcon,
                        |ui| {
                            if ui
                                .button(RichText::new(phos::ARROW_COUNTER_CLOCKWISE))
                                .on_hover_text_at_pointer(t!("help.action.reset_icon"))
                                .clicked()
                            {
                                self.editing_action_icon = ActionIcon::DefaultActionIcon;
                            }
                        },
                    );
                });
                ui.with_layout(
                    Layout::centered_and_justified(eframe::egui::Direction::TopDown)
                        .with_cross_justify(false),
                    |ui| {
                        ui.label(RichText::new(self.editing_action.help()).size(10.0));
                    },
                );
            });
            c1.separator();
            c1.allocate_space(vec2(0.0, 2.0));
            c1.allocate_ui(vec2(228.0, 162.0), |ui| {
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

    fn load_custom_icon(&mut self, f: PathBuf) -> Result<PathBuf, ActionError> {
        let image = image::open(f)
            .map_err(|_| {
                ActionError::new(
                    self.current_device.clone(),
                    self.editing_key,
                    t!("help.action.err.not_an_image"),
                )
            })?
            .into_rgb8();

        let image = image::imageops::resize(&image, 32, 32, image::imageops::FilterType::Nearest);

        let rgb = image.into_raw();
        let mut icon = [0u16; 32 * 32];
        for i in 0..(32 * 32) {
            let r = (((rgb[i * 3 + 0] as f64) / 255.0 * 31.0).round() as u16) & 0b00000000_00011111;
            let g = (((rgb[i * 3 + 1] as f64) / 255.0 * 63.0).round() as u16) & 0b00000000_00111111;
            let b = (((rgb[i * 3 + 2] as f64) / 255.0 * 31.0).round() as u16) & 0b00000000_00011111;

            let c = (r << 11) | (g << 5) | b;

            let x = i % 32;
            let y = 31 - i / 32;
            icon[y * 32 + x] = c;
        }

        let mut data = Vec::new();
        data.extend_from_slice(BMP_HEADER);
        data.extend_from_slice(icon.as_bytes());

        std::fs::create_dir_all(JukeBoxConfig::get_icon_dir()).map_err(|_| {
            ActionError::new(
                self.current_device.clone(),
                self.editing_key,
                t!("help.action.err.mkdir_fail"),
            )
        })?;

        let s = sha1_smol::Sha1::from(data.clone()).digest().to_string() + ".bmp";

        let mut p = JukeBoxConfig::get_icon_dir();
        p.push(s);

        if std::fs::metadata(p.clone()).is_err() {
            std::fs::write(p.clone(), data).map_err(|_| {
                ActionError::new(
                    self.current_device.clone(),
                    self.editing_key,
                    t!("help.action.err.write_fail"),
                )
            })?;
        }

        Ok(p)
    }
}
