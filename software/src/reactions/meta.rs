use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

use crate::input::InputKey;

use super::types::{Reaction, ReactionType};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ActMetaTest {}
#[typetag::serde]
impl Reaction for ActMetaTest {
    fn on_press(&self, key: InputKey) -> () {
        log::info!("METATEST: Pressed {:?} !", key);
    }

    fn on_release(&self, key: InputKey) -> () {
        log::info!("METATEST: Released {:?} !", key);
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::MetaTest
    }

    fn edit_ui(&mut self, _ui: &mut Ui) {}
}
