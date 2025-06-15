use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use eframe::egui::{include_image, ComboBox, ImageSource, Slider, Ui};
use egui_phosphor::regular as phos;
use jukebox_util::input::{KeyboardEvent, MouseEvent, KEYBOARD_SCAN_CODES};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Action, ActionError};

pub const AID_INPUT_KEYBOARD: &str = "InputKeyboard";
pub const AID_INPUT_MOUSE: &str = "InputMouse";
// pub const AID_INPUT_GAMEPAD: &str = "InputGamepad";

const ICON_KEYBOARD: ImageSource =
    include_image!("../../../assets/action-icons/input-keyboard.bmp");
const ICON_MOUSE: ImageSource = include_image!("../../../assets/action-icons/input-mouse.bmp");

static KEY_MAP: OnceLock<HashMap<u8, &str>> = OnceLock::new();

#[rustfmt::skip]
pub fn input_action_list() -> (String, Vec<(String, Action, String)>) {
    (
        t!("action.input.title", icon = phos::CURSOR_CLICK).into(),
        vec![
            (AID_INPUT_KEYBOARD.into(), Action::InputKeyboard(InputKeyboard::default()), t!("action.input.keyboard.title").into()),
            (AID_INPUT_MOUSE.into(),    Action::InputMouse(InputMouse::default()),       t!("action.input.mouse.title").into()),
            // (AID_INPUT_GAMEPAD.into(),  Action::InputGamepad(InputGamepad::default()),  t!("action.input.gamepad.title").into()),
        ],
    )
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct InputKeyboard {
    keys: Vec<u8>,
}
impl InputKeyboard {
    pub async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        // TODO: trigger on test input from gui?
        Ok(())
    }

    pub async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        // TODO: trigger on test input from gui?
        Ok(())
    }

    pub fn get_type(&self) -> String {
        AID_INPUT_KEYBOARD.into()
    }

    pub fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) {
        ui.horizontal(|ui| {
            ui.label(t!("action.input.keyboard.add_keys"));
            ui.add_enabled_ui(self.keys.len() < 6, |ui| {
                if ui.button("+").clicked() {
                    self.keys.push(0x04);
                }
            });
        });
        let mut delete = Vec::new();
        let key_map = KEY_MAP.get_or_init(|| HashMap::from(KEYBOARD_SCAN_CODES));
        for (i, k) in self.keys.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                if ui.button(phos::TRASH).clicked() {
                    delete.push(i);
                }
                ComboBox::from_id_salt(format!("InputKeyboardBox_{}", i))
                    .selected_text(format!("{:#04X} - \"{}\"", k, key_map.get(k).unwrap()))
                    .width(196.0)
                    .show_ui(ui, |ui| {
                        for sc in KEYBOARD_SCAN_CODES {
                            ui.selectable_value(k, sc.0, format!("{:#04X} - \"{}\"", sc.0, sc.1));
                        }
                    });
            });
        }
        delete.reverse();
        for i in delete {
            self.keys.remove(i);
        }
    }

    pub fn help(&self) -> String {
        t!("action.input.keyboard.help").into()
    }

    pub fn icon_source(&self) -> ImageSource {
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

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct InputMouse {
    buttons: u8,
    x: i8,
    y: i8,
    scroll_y: i8,
    scroll_x: i8,
}
impl InputMouse {
    pub async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        // TODO: trigger on test input from gui?
        Ok(())
    }

    pub async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        // TODO: trigger on test input from gui?
        Ok(())
    }

    pub fn get_type(&self) -> String {
        AID_INPUT_MOUSE.into()
    }

    pub fn edit_ui(
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
                t!("action.input.mouse.button.button_4"),
            ),
            (
                (self.buttons & (1 << 4)) > 0,
                t!("action.input.mouse.button.button_5"),
            ),
            (
                (self.buttons & (1 << 5)) > 0,
                t!("action.input.mouse.button.button_6"),
            ),
            (
                (self.buttons & (1 << 6)) > 0,
                t!("action.input.mouse.button.button_7"),
            ),
            (
                (self.buttons & (1 << 7)) > 0,
                t!("action.input.mouse.button.button_8"),
            ),
        ];
        let mut n = 0;
        for (i, (bit, text)) in bits.iter_mut().enumerate() {
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

    pub fn help(&self) -> String {
        t!("action.input.mouse.help").into()
    }

    pub fn icon_source(&self) -> ImageSource {
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
