use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

use crate::input::InputKey;

use super::types::{Reaction, ReactionType};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ReactionSystemOpenWebsite {
    url: String,
}
#[typetag::serde]
impl Reaction for ReactionSystemOpenWebsite {
    fn on_press(&self, _key: InputKey) -> () {}

    fn on_release(&self, _key: InputKey) -> () {
        let _ = open::that(self.url.clone());
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::SystemOpenWebsite
    }

    fn edit_ui(&mut self, ui: &mut Ui) {
        ui.label("URL:");
        ui.text_edit_singleline(&mut self.url);
    }
}
