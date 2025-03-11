use anyhow::Result;
use eframe::egui::{ComboBox, Slider, Ui};
use egui_phosphor::regular as phos;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Action, ActionType as AT};

#[rustfmt::skip]
pub fn soundboard_action_list() -> (String, Vec<(AT, Box<dyn Action>, String)>) {
    (
        t!("action.soundboard.title", icon = phos::MUSIC_NOTES).to_string(),
        vec![(AT::SoundboardPlaySound, Box::new(SoundboardPlaySound::default()), t!("action.soundboard.play_sound.title").to_string())],
    )
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct SoundboardPlaySound {
    filepath: String,
    output_device: String,
    volume: u8,
}
#[async_trait::async_trait]
#[typetag::serde]
impl Action for SoundboardPlaySound {
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

    fn get_type(&self) -> AT {
        AT::SoundboardPlaySound
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: &mut JukeBoxConfig,
    ) {
        if ui
            .button(t!("action.soundboard.play_sound.choose_file"))
            .clicked()
        {
            if let Some(f) = FileDialog::new().pick_file() {
                self.filepath = f.to_str().unwrap().to_owned();
            }
        }
        ui.text_edit_singleline(&mut self.filepath);

        ui.label("");

        ui.label(t!("action.soundboard.play_sound.output_device"));
        ComboBox::from_id_salt("SoundboardPlaySoundDeviceSelect")
            .selected_text(self.output_device.clone())
            .width(228.0)
            .show_ui(ui, |_ui| {});

        ui.label("");

        ui.label(t!("action.soundboard.play_sound.volume"));
        ui.add(Slider::new(&mut self.volume, 0..=100));
    }

    fn help(&self) -> String {
        t!("action.soundboard.play_sound.help").to_string()
    }
}
