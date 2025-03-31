use std::sync::Arc;

use anyhow::Result;
use eframe::egui::{include_image, ComboBox, ImageSource, Slider, Ui};
use egui_phosphor::regular as phos;
use jukebox_util::input::{KeyboardEvent, MouseEvent, KEYBOARD_SCAN_CODES};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::Action;

const ICON_KEYBOARD: ImageSource =
    include_image!("../../../assets/action-icons/input-keyboard.bmp");
const ICON_MOUSE: ImageSource = include_image!("../../../assets/action-icons/input-mouse.bmp");

#[rustfmt::skip]
pub fn input_action_list() -> (String, Vec<(String, Box<dyn Action>, String)>) {
    (
        t!("action.input.title", icon = phos::CURSOR_CLICK).into(),
        vec![
            ("InputKeyboard".into(), Box::new(InputKeyboard::default()), t!("action.input.keyboard.title").into()),
            // ("InputMouse".into(),    Box::new(InputMouse::default()),    t!("action.input.mouse.title").into()),
            // ("InputGamepad".into(),  Box::new(InputGamepad::default()),  t!("action.input.gamepad.title").into()),
        ],
    )
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct InputKeyboard {
    keys: Vec<u8>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for InputKeyboard {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    fn get_type(&self) -> String {
        "InputKeyboard".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.horizontal(|ui| {
            ui.label(t!("action.input.keyboard.add_keys"));
            ui.add_enabled_ui(self.keys.len() <= 6, |ui| {
                if ui.button("+").clicked() {
                    self.keys.push(0u8);
                }
            });
        });
        let mut delete = Vec::new();
        for (i, k) in self.keys.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui.button(phos::TRASH).clicked() {
                    delete.push(i);
                }
                ComboBox::from_id_salt(format!("InputKeyboardBox_{}", i))
                    .selected_text(format!("{:#04X}", k))
                    .width(196.0)
                    .show_ui(ui, |ui| {
                        for sc in KEYBOARD_SCAN_CODES {
                            ui.selectable_value(k, sc.1, format!("{:#04X} - \"{}\"", sc.1, sc.0));
                        }
                    });
            });
        }
        delete.reverse();
        for i in delete {
            self.keys.remove(i);
        }
    }

    fn help(&self) -> String {
        t!("action.input.keyboard.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_KEYBOARD
    }
}
impl InputKeyboard {
    pub fn get_keyboard_event(&self) -> KeyboardEvent {
        let mut keys = [0u8; 6];
        for (i, v) in self.keys.iter().enumerate() {
            keys[i] = *v;
        }

        KeyboardEvent { keys }
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct InputMouse {
    buttons: u8,
    x: i8,
    y: i8,
    scroll_y: i8,
    scroll_x: i8,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for InputMouse {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    fn get_type(&self) -> String {
        "InputMouse".into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.label(t!("action.input.mouse.buttons"));
        let mut bits = [
            (
                (self.buttons & (1 << 0)) > 0,
                t!("action.input.mouse.button.left"),
            ),
            (
                (self.buttons & (1 << 1)) > 0,
                t!("action.input.mouse.button.right"),
            ),
            (
                (self.buttons & (1 << 2)) > 0,
                t!("action.input.mouse.button.middle"),
            ),
            (
                (self.buttons & (1 << 3)) > 0,
                t!("action.input.mouse.button.unknown"),
            ),
            (
                (self.buttons & (1 << 4)) > 0,
                t!("action.input.mouse.button.unknown"),
            ),
            (
                (self.buttons & (1 << 5)) > 0,
                t!("action.input.mouse.button.unknown"),
            ),
            (
                (self.buttons & (1 << 6)) > 0,
                t!("action.input.mouse.button.unknown"),
            ),
            (
                (self.buttons & (1 << 7)) > 0,
                t!("action.input.mouse.button.unknown"),
            ),
        ];
        let mut n = 0;
        for (i, (bit, text)) in bits.iter_mut().enumerate().rev() {
            ui.checkbox(bit, text.clone());
            n |= (*bit as u8) << i;
        }
        self.buttons = n;

        ui.label("");

        ui.horizontal(|ui| {
            ui.label(t!("action.input.mouse.move_x"));
            ui.add(Slider::new(&mut self.x, i8::MIN..=i8::MAX));
        });
        ui.horizontal(|ui| {
            ui.label(t!("action.input.mouse.move_y"));
            ui.add(Slider::new(&mut self.y, i8::MIN..=i8::MAX));
        });

        ui.label("");

        ui.horizontal(|ui| {
            ui.label(t!("action.input.mouse.scroll_y"));
            ui.add(Slider::new(&mut self.scroll_y, i8::MIN..=i8::MAX));
        });
        ui.horizontal(|ui| {
            ui.label(t!("action.input.mouse.scroll_x"));
            ui.add(Slider::new(&mut self.scroll_x, i8::MIN..=i8::MAX));
        });
    }

    fn help(&self) -> String {
        t!("action.input.mouse.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_MOUSE
    }
}
impl InputMouse {
    pub fn get_mouse_event(&self) -> MouseEvent {
        MouseEvent {
            buttons: self.buttons,
            x: self.x,
            y: self.y,
            scroll_y: self.scroll_y,
            scroll_x: self.scroll_x,
        }
    }
}
