use std::sync::Arc;

use eframe::egui::{include_image, ComboBox, ImageSource, Slider, TextWrapMode, Ui};
use egui_phosphor::regular as phos;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{config::JukeBoxConfig, input::InputKey};

use super::types::{Action, ActionError};

pub const AID_SOUNDBOARD_PLAY_SOUND: &str = "SoundboardPlaySound";

const ICON_PLAY_SOUND: ImageSource =
    include_image!("../../../assets/action-icons/soundboard-play.bmp");

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
enum PlayMethod {
    #[default]
    PlayStop,
    PlayOverlap,
    PlayRestart,
    LoopStop,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
enum FadeMethod {
    #[default]
    NoFade,
    FadeIn,
    FadeOut,
    FadeInAndOut,
}

#[rustfmt::skip]
pub fn soundboard_action_list() -> (String, Vec<(String, Action, String)>) {
    (
        t!("action.soundboard.title", icon = phos::MUSIC_NOTES).into(),
        vec![(AID_SOUNDBOARD_PLAY_SOUND.into(), Action::SoundboardPlaySound(SoundboardPlaySound::default()), t!("action.soundboard.play_sound.title").into())],
    )
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SoundboardPlaySound {
    filepath: String,
    volume: u8,
    play_method: PlayMethod,
    fade_method: FadeMethod,
    fade_time: u8,
    output_device: String,
}
impl SoundboardPlaySound {
    pub async fn on_press(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        // TODO
        Ok(())
    }

    pub async fn on_release(
        &self,
        _device_uid: &String,
        _input_key: InputKey,
        _config: Arc<Mutex<JukeBoxConfig>>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    pub fn get_type(&self) -> String {
        AID_SOUNDBOARD_PLAY_SOUND.into()
    }

    pub fn edit_ui(
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

        ui.label(t!("action.soundboard.play_sound.volume"));
        ui.add(Slider::new(&mut self.volume, 0..=100));

        ui.label("");

        let combo = &[
            (
                PlayMethod::PlayStop,
                t!("action.soundboard.play_sound.play_method.play_stop"),
            ),
            (
                PlayMethod::PlayOverlap,
                t!("action.soundboard.play_sound.play_method.play_overlap"),
            ),
            (
                PlayMethod::PlayRestart,
                t!("action.soundboard.play_sound.play_method.play_restart"),
            ),
            (
                PlayMethod::LoopStop,
                t!("action.soundboard.play_sound.play_method.loop_stop"),
            ),
        ];

        let current_combo = combo.iter().position(|c| c.0 == self.play_method).unwrap();

        ui.label(t!("action.soundboard.play_sound.play_method.title"));
        ComboBox::from_id_salt("SoundboardPlaySoundPlayMethod")
            .selected_text(combo[current_combo].1.clone())
            .width(200.0)
            .wrap_mode(TextWrapMode::Truncate)
            .show_ui(ui, |ui| {
                for c in combo {
                    ui.selectable_value(&mut self.play_method, c.0, c.1.clone());
                }
            });

        ui.label("");

        let combo = &[
            (
                FadeMethod::NoFade,
                t!("action.soundboard.play_sound.fade_method.no_fade"),
            ),
            (
                FadeMethod::FadeIn,
                t!("action.soundboard.play_sound.fade_method.fade_in"),
            ),
            (
                FadeMethod::FadeOut,
                t!("action.soundboard.play_sound.fade_method.fade_out"),
            ),
            (
                FadeMethod::FadeInAndOut,
                t!("action.soundboard.play_sound.fade_method.fade_in_and_out"),
            ),
        ];

        let current_combo = combo.iter().position(|c| c.0 == self.fade_method).unwrap();

        ui.label(t!("action.soundboard.play_sound.fade_method.title"));
        ComboBox::from_id_salt("SoundboardPlaySoundFadeMethod")
            .selected_text(combo[current_combo].1.clone())
            .width(200.0)
            .wrap_mode(TextWrapMode::Truncate)
            .show_ui(ui, |ui| {
                for c in combo {
                    ui.selectable_value(&mut self.fade_method, c.0, c.1.clone());
                }
            });

        if self.fade_method != FadeMethod::NoFade {
            ui.label("");

            ui.label(t!("action.soundboard.play_sound.fade_time"));
            ui.add(Slider::new(&mut self.fade_time, 0..=5));
        }

        ui.label("");

        ui.label(t!("action.soundboard.play_sound.output_device"));
        ComboBox::from_id_salt("SoundboardPlaySoundDeviceSelect")
            .selected_text(self.output_device.clone())
            .width(200.0)
            .wrap_mode(TextWrapMode::Truncate)
            .show_ui(ui, |_ui| {
                // TODO
            });
    }

    pub fn help(&self) -> String {
        t!("action.soundboard.play_sound.help").into()
    }

    pub fn icon_source(&self) -> ImageSource {
        ICON_PLAY_SOUND
    }
}
