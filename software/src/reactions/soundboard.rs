use eframe::egui::{ComboBox, Slider, SliderClamping, Ui};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Reaction, ReactionType};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SoundboardPlaySound {
    filepath: String,
    output_device: String,
    volume: f64,
}
#[typetag::serde]
impl Reaction for SoundboardPlaySound {
    fn on_press(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {
        // TODO
    }

    fn on_release(&self, _device_uid: String, _key: InputKey, _config: &mut JukeBoxConfig) -> () {}

    fn get_type(&self) -> ReactionType {
        ReactionType::SoundboardPlaySound
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: String,
        _key: InputKey,
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
        ui.add(Slider::new(&mut self.volume, 0.0..=100.0).clamping(SliderClamping::Always));
    }

    fn help(&self) -> String {
        "Plays a sound file to an output audio device on press.".to_string()
    }
}
