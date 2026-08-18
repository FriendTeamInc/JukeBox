use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use eframe::egui::{include_image, ComboBox, ImageSource, Slider, Ui};
use egui_phosphor::regular as phos;
use jukebox_util::input::{InputEvent, KeyboardEvent, MouseEvent, KEYBOARD_SCAN_CODES};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    actions::types::{ActionModuleConfig, ActionTrait},
    config::JukeBoxConfig,
};

use super::types::Action;

pub const AID_INPUT_KEYBOARD: &str = "InputKeyboard";
pub const AID_INPUT_MOUSE: &str = "InputMouse";
// pub const AID_INPUT_GAMEPAD: &str = "InputGamepad";

const ICON_KEYBOARD: ImageSource =
    include_image!("../../../assets/action-icons/input-keyboard.bmp");
const ICON_MOUSE: ImageSource = include_image!("../../../assets/action-icons/input-mouse.bmp");

static KEY_MAP: OnceLock<HashMap<u8, &str>> = OnceLock::new();

pub fn init_actions_input(_config: ActionModuleConfig) -> (String, Vec<Action>) {
    (
        t!("action.input.title", icon = phos::CURSOR_CLICK).into(),
        vec![
            Arc::new(InputKeyboard::default()),
            Arc::new(InputMouse::default()),
            // Arc::new(InputGamepad::default()),
        ],
    )
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct InputKeyboard {
    pub keys: Vec<u8>,
}
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for InputKeyboard {
    fn get_type(&self) -> &'static str {
        AID_INPUT_KEYBOARD
    }
    fn get_title(&self) -> &'static str {
        "action.input.keyboard.title"
    }
    fn get_description(&self) -> &'static str {
        "action.input.keyboard.help"
    }

    fn edit_ui(&mut self, _module_config: &mut ActionModuleConfig, ui: &mut Ui) {
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

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_KEYBOARD]
    }
}
impl InputKeyboard {
    pub fn get_input_event(&self) -> InputEvent {
        let mut keys = [0u8; 6];
        for (i, v) in self.keys.iter().enumerate() {
            keys[i] = *v;
        }

        InputEvent::Keyboard(KeyboardEvent { keys })
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
#[async_trait::async_trait]
#[typetag::serde]
impl ActionTrait for InputMouse {
    fn get_type(&self) -> &'static str {
        AID_INPUT_MOUSE
    }
    fn get_title(&self) -> &'static str {
        "action.input.mouse.title"
    }
    fn get_description(&self) -> &'static str {
        "action.input.mouse.help"
    }

    fn edit_ui(&mut self, _module_config: &mut ActionModuleConfig, ui: &mut Ui) {
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

    fn icon_state_icons(&'_ self) -> &[ImageSource<'_>] {
        &[ICON_MOUSE]
    }
}
impl InputMouse {
    pub fn get_input_event(&self) -> InputEvent {
        InputEvent::Mouse(MouseEvent {
            buttons: self.buttons,
            x: self.x,
            y: self.y,
            scroll_y: self.scroll_y,
            scroll_x: self.scroll_x,
        })
    }
}
