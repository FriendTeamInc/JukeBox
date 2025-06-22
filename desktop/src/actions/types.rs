// Types of actions and their associations

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, OnceLock},
};

use eframe::egui::{
    load::Bytes, vec2, Image, ImageSource, TextureFilter, TextureOptions, TextureWrapMode, Ui,
};
use jukebox_util::peripheral::DeviceType;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    actions::{discord::*, input::*, meta::*, obs::*, soundboard::*, system::*},
    config::{ActionConfig, ActionIcon, JukeBoxConfig},
    input::InputKey,
};

static ICON_CACHE: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ActionError {
    pub device_uid: String,
    pub input_key: InputKey,
    pub msg: String,
}
impl ActionError {
    pub fn new(device_uid: impl Into<String>, input_key: InputKey, msg: impl Into<String>) -> Self {
        Self {
            device_uid: device_uid.into(),
            input_key: input_key,
            msg: msg.into(),
        }
    }
}
impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

macro_rules! create_actions {
    ( $( $item:ident ),* ) => {
        #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
        pub enum Action {
            $($item($item),)*
        }

        impl Action {
            pub async fn on_press(
                &self,
                device_uid: &String,
                input_key: InputKey,
                config: Arc<Mutex<JukeBoxConfig>>,
            ) -> Result<(), ActionError> {
                match self {
                    $(Self::$item(x) => x.on_press(device_uid, input_key, config).await,)*
                }
            }

            pub async fn on_release(
                &self,
                device_uid: &String,
                input_key: InputKey,
                config: Arc<Mutex<JukeBoxConfig>>,
            ) -> Result<(), ActionError> {
                match self {
                    $(Self::$item(x) => x.on_release(device_uid, input_key, config).await,)*
                }
            }

            pub fn edit_ui(
                &mut self,
                ui: &mut Ui,
                device_uid: &String,
                input_key: InputKey,
                config: Arc<Mutex<JukeBoxConfig>>,
            ) {
                match self {
                    $(Self::$item(x) => x.edit_ui(ui, device_uid, input_key, config),)*
                }
            }

            pub fn get_type(&self) -> String {
                match self {
                    $(Self::$item(x) => x.get_type(),)*
                }
            }

            pub fn help(&self) -> String {
                match self {
                    $(Self::$item(x) => x.help(),)*
                }
            }

            pub fn icon_source(&self) -> ImageSource {
                match self {
                    $(Self::$item(x) => x.icon_source(),)*
                }
            }

            pub fn icon(&self) -> Image {
                Image::new(self.icon_source())
                    .texture_options(TextureOptions {
                        magnification: TextureFilter::Nearest,
                        minification: TextureFilter::Nearest,
                        wrap_mode: TextureWrapMode::ClampToEdge,
                        mipmap_mode: None,
                    })
                    .corner_radius(2.0)
                    .max_size(vec2(64.0, 64.0))
            }
        }
    };
}

create_actions! {
    MetaNoAction,
    MetaSwitchProfile,
    // MetaCopyFromProfile,

    SystemOpenApp,
    SystemOpenWeb,
    SystemSndInCtrl,
    SystemSndOutCtrl,

    SoundboardPlaySound,

    InputKeyboard,
    InputMouse,
    // InputGamepad,

    ObsStream,
    ObsRecord,
    ObsPauseRecord,
    ObsReplayBuffer,
    ObsSaveReplay,
    ObsSource,
    ObsMute,
    ObsSceneSwitch,
    ObsPreviewSceneSwitch,
    ObsPreviewScenePush,
    ObsSceneCollectionSwitch,
    // ObsFilter,
    // ObsTransition,
    ObsChapterMarker,

    DiscordToggleMute,
    DiscordToggleDeafen,
    DiscordPushToTalk,
    DiscordPushToMute,
    DiscordPushToDeafen
}

// #[async_trait::async_trait]
// #[typetag::serde(tag = "type")]
// pub trait Action: Sync + Send + DynClone + Downcast + DowncastSync + DowncastSend {
//     pub async fn on_press(
//         &self,
//         device_uid: &String,
//         input_key: InputKey,
//         config: Arc<Mutex<JukeBoxConfig>>,
//     ) -> Result<(), ActionError>;
//     pub async fn on_release(
//         &self,
//         device_uid: &String,
//         input_key: InputKey,
//         config: Arc<Mutex<JukeBoxConfig>>,
//     ) -> Result<(), ActionError>;
//     fn get_type(&self) -> String;
//     pub fn edit_ui(
//         &mut self,
//         ui: &mut Ui,
//         device_uid: &String,
//         input_key: InputKey,
//         config: Arc<Mutex<JukeBoxConfig>>,
//     );
//     pub fn help(&self) -> String;
//     pub fn icon_source(&self) -> ImageSource;
//     fn icon(&self) -> Image {
//         Image::new(self.icon_source())
//             .texture_options(TextureOptions {
//                 magnification: TextureFilter::Nearest,
//                 minification: TextureFilter::Nearest,
//                 wrap_mode: TextureWrapMode::ClampToEdge,
//                 mipmap_mode: None,
//             })
//             .corner_radius(2.0)
//             .max_size(vec2(64.0, 64.0))
//     }
// }
// clone_trait_object!(Action);
// impl_downcast!(Action);

pub struct ActionMap {
    ui_list: Vec<(String, Vec<(String, String)>)>,
    enum_map: HashMap<String, Action>,
}
impl ActionMap {
    pub fn new() -> Self {
        // this function is only safe to call once!
        // TODO: we should probably fix that...

        let l = vec![
            meta_action_list(),
            input_action_list(),
            system_action_list(),
            // soundboard_action_list(),
            #[cfg(feature = "discord")]
            discord_action_list(),
            obs_action_list(),
        ];

        let ui_list = l
            .iter()
            .map(|(title, l)| {
                (
                    title.clone(),
                    l.iter().map(|(at, _, s)| (at.clone(), s.clone())).collect(),
                )
            })
            .collect();

        let enum_map = l
            .iter()
            .map(|(_, l)| l)
            .flatten()
            .map(|(at, a, _)| (at.clone(), a.clone()))
            .collect();

        Self { ui_list, enum_map }
    }

    pub fn ui_list(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.ui_list.clone()
    }

    pub fn enum_new(&self, t: String) -> Action {
        self.enum_map.get(&t).unwrap().clone()
    }

    pub fn default_action_config(d: DeviceType) -> HashMap<InputKey, ActionConfig> {
        use InputKey as IK;
        let keys = match d {
            DeviceType::Unknown => &[][..],
            DeviceType::KeyPad => &[
                IK::KeySwitch1,
                IK::KeySwitch2,
                IK::KeySwitch3,
                IK::KeySwitch4,
                IK::KeySwitch5,
                IK::KeySwitch6,
                IK::KeySwitch7,
                IK::KeySwitch8,
                IK::KeySwitch9,
                IK::KeySwitch10,
                IK::KeySwitch11,
                IK::KeySwitch12,
            ][..],
            DeviceType::KnobPad => &[
                IK::KnobLeftSwitch,
                IK::KnobLeftClockwise,
                IK::KnobLeftCounterClockwise,
                IK::KnobRightSwitch,
                IK::KnobRightClockwise,
                IK::KnobRightCounterClockwise,
            ][..],
            DeviceType::PedalPad => &[IK::PedalLeft, IK::PedalMiddle, IK::PedalRight][..],
        };

        let mut c = HashMap::new();
        for k in keys {
            c.insert(
                *k,
                ActionConfig {
                    action: Action::MetaNoAction(MetaNoAction {}),
                    icon: ActionIcon::DefaultActionIcon,
                },
            );
        }

        c
    }
}

// todo: resolve redundancy here
pub async fn get_icon_bytes_async(action_config: &ActionConfig) -> [u8; 32 * 32 * 2] {
    let b = match &action_config.icon {
        ActionIcon::ImageIcon(i) => {
            let mut icon_cache = ICON_CACHE
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .await;

            if !icon_cache.contains_key(i) {
                // TODO: use fallback in cases where we can't read icon?
                icon_cache.insert(
                    i.into(),
                    std::fs::read(i).expect("failed to read icon data"),
                );
            }

            icon_cache.get(i).unwrap().clone()
        }
        ActionIcon::DefaultActionIcon => match action_config.action.icon_source() {
            ImageSource::Uri(_) => panic!(),
            ImageSource::Texture(_) => panic!(),
            ImageSource::Bytes { uri: _, bytes } => match bytes {
                Bytes::Static(items) => items.to_vec(),
                Bytes::Shared(items) => items.to_vec(),
            },
        },
    };

    get_icon_bytes_internal(b)
}

pub fn get_icon_bytes(action_config: &ActionConfig) -> [u8; 32 * 32 * 2] {
    let b = match &action_config.icon {
        ActionIcon::ImageIcon(i) => {
            let mut icon_cache = ICON_CACHE
                .get_or_init(|| Mutex::new(HashMap::new()))
                .blocking_lock();

            if !icon_cache.contains_key(i) {
                // TODO: use fallback in cases where we can't read icon?
                icon_cache.insert(
                    i.into(),
                    std::fs::read(i).expect("failed to read icon data"),
                );
            }

            icon_cache.get(i).unwrap().clone()
        }
        ActionIcon::DefaultActionIcon => match action_config.action.icon_source() {
            ImageSource::Uri(_) => panic!(),
            ImageSource::Texture(_) => panic!(),
            ImageSource::Bytes { uri: _, bytes } => match bytes {
                Bytes::Static(items) => items.to_vec(),
                Bytes::Shared(items) => items.to_vec(),
            },
        },
    };

    get_icon_bytes_internal(b)
}

fn get_icon_bytes_internal(b: Vec<u8>) -> [u8; 32 * 32 * 2] {
    let (_, b) = b.split_at(0x7A);

    if b.len() != (32 * 32 * 2) {
        panic!();
    }

    let mut bytes = [0u8; 32 * 32 * 2];
    bytes.copy_from_slice(b);

    bytes
}

#[macro_export]
macro_rules! single_fire {
    ($eval:expr, $call:expr) => {{
        static LATCH: std::sync::OnceLock<std::sync::atomic::AtomicBool> =
            std::sync::OnceLock::new();
        let expr = $eval;
        if expr {
            if LATCH
                .get_or_init(|| false.into())
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                $call
            }
            let _ = LATCH.set(false.into());
        } else {
            let _ = LATCH.set(true.into());
        }
    }};
}
