use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

use crate::input::InputKey;

use super::types::{Reaction, ReactionType};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct MetaNoAction {}
#[typetag::serde]
impl Reaction for MetaNoAction {
    fn on_press(&self, key: InputKey) -> () {
        log::info!("META NO ACTION: Pressed {:?} !", key);
    }

    fn on_release(&self, key: InputKey) -> () {
        log::info!("META NO ACTION: Released {:?} !", key);
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::MetaNoAction
    }

    fn edit_ui(&mut self, _ui: &mut Ui) {}

    fn help(&self) -> String {
        "Does nothing!".to_string()
    }
}
