use std::collections::HashMap;

use anyhow::Result;
use eframe::egui::{ComboBox, Slider, Ui};
use egui_phosphor::regular as phos;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Reaction, ReactionType as RT};

#[rustfmt::skip]
pub fn soundboard_reaction_list() -> (String, Vec<(RT, String)>) {
    (
        format!("{} Soundboard", phos::MUSIC_NOTES),
        vec![(RT::SoundboardPlaySound, "Play Sound".to_string())],
    )
}

#[rustfmt::skip]
pub fn soundboard_enum_map() -> HashMap<RT, Box<dyn Reaction>> {
    let mut h: HashMap<RT, Box<dyn Reaction>> = HashMap::new();

    h.insert(RT::SoundboardPlaySound, Box::new(SoundboardPlaySound::default()));

    h
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SoundboardPlaySound {
    filepath: String,
    output_device: String,
    volume: u8,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Reaction for SoundboardPlaySound {
    async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        // TODO
        Ok(())
    }

    async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) -> Result<()> {
        Ok(())
    }

    fn get_type(&self) -> RT {
        RT::SoundboardPlaySound
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        if ui.button("Choose File").clicked() {
            if let Some(f) = FileDialog::new().pick_file() {
                self.filepath = f.to_str().unwrap().to_owned();
            }
        }
        ui.text_edit_singleline(&mut self.filepath);

        ui.label("");

        ui.label("Output device:");
        ComboBox::from_id_salt("SoundboardPlaySoundDeviceSelect")
            .selected_text(self.output_device.clone())
            .width(228.0)
            .show_ui(ui, |_ui| {});

        ui.label("");

        ui.label("Volume:");
        ui.add(Slider::new(&mut self.volume, 0..=100));
    }

    fn help(&self) -> String {
        "Plays a sound file to an output audio device on press.".to_string()
    }
}
