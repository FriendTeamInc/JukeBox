use serde::{Deserialize, Serialize};

use crate::input::InputKey;

use super::types::{Reaction, ReactionType};

#[derive(Serialize, Deserialize, Clone)]
pub struct ReactionMetaTest {}
#[typetag::serde]
impl Reaction for ReactionMetaTest {
    fn on_press(&self, key: InputKey) -> () {
        log::info!("Pressed {:?} !", key);
    }

    fn on_release(&self, key: InputKey) -> () {
        log::info!("Released {:?} !", key);
    }

    fn get_type(&self) -> ReactionType {
        ReactionType::MetaTest
    }
}
