use std::sync::Arc;

use anyhow::Result;
use eframe::egui::{include_image, ComboBox, ImageSource, Slider, Ui};
use egui_phosphor::regular as phos;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::Action;

pub const AID_SOUNDBOARD_PLAY_SOUND: &str = "SoundboardPlaySound";

const ICON_PLAY_SOUND: ImageSource =
    include_image!("../../../assets/action-icons/soundboard-play.bmp");

#[rustfmt::skip]
pub fn soundboard_action_list() -> (String, Vec<(String, Box<dyn Action>, String)>) {
    (
        t!("action.soundboard.title", icon = phos::MUSIC_NOTES).into(),
        vec![(AID_SOUNDBOARD_PLAY_SOUND.into(), Box::new(SoundboardPlaySound::default()), t!("action.soundboard.play_sound.title").into())],
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
        Ok(())
    }

    fn get_type(&self) -> String {
        AID_SOUNDBOARD_PLAY_SOUND.into()
    }

    fn edit_ui(
        &mut self,
        ui: &mut Ui,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
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
        t!("action.soundboard.play_sound.help").into()
    }

    fn icon_source(&self) -> ImageSource {
        ICON_PLAY_SOUND
    }
}
