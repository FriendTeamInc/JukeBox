// Types of actions and their associations

use std::{
    any::Any,
    collections::HashMap,
    fmt,
    sync::{Arc, OnceLock},
};

use async_trait::async_trait;
use downcast_rs::{impl_downcast, DowncastSync};
use dyn_clone::{clone_trait_object, DynClone};
use eframe::egui::{
    load::Bytes, Image, ImageSource, TextureFilter, TextureOptions, TextureWrapMode, Ui,
};
use jukebox_util::peripheral::DeviceType;
use tokio::sync::{Mutex, MutexGuard};

use crate::{
    actions::{discord::*, input::*, meta::*, obs::*, system::*},
    config::{ActionConfig, ActionIcon, JukeBoxConfig},
    input::InputKey,
};

pub static ICON_CACHE: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ActionOk {
    pub save_action_config: bool,
    pub save_module_config: bool,
    pub switch_to_profile: Option<String>,
    pub change_icon: bool,
}
impl ActionOk {
    pub fn new() -> Self {
        Self {
            save_action_config: false,
            save_module_config: false,
            switch_to_profile: None,
            change_icon: false,
        }
    }

    pub fn save_action_config(mut self, save_action_config: bool) -> Self {
        self.save_action_config = save_action_config;
        self
    }

    pub fn save_module_config(mut self, save_module_config: bool) -> Self {
        self.save_module_config = save_module_config;
        self
    }

    pub fn switch_to_profile(mut self, switch_to_profile: impl Into<String>) -> Self {
        self.switch_to_profile = Some(switch_to_profile.into());
        self
    }

    pub fn change_icon(mut self, change_icon: bool) -> Self {
        self.change_icon = change_icon;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ActionError {
    pub device_uid: Option<String>,
    pub input_key: Option<InputKey>,
    pub msg: String,
}
impl ActionError {
    pub fn new(device_uid: impl Into<String>, input_key: InputKey, msg: impl Into<String>) -> Self {
        let s = Self {
            device_uid: Some(device_uid.into()),
            input_key: Some(input_key),
            msg: msg.into(),
        };
        log::debug!("{}", s);
        s
    }
    pub fn msg(msg: impl Into<String>) -> Self {
        let s = Self {
            device_uid: None,
            input_key: None,
            msg: msg.into(),
        };
        log::debug!("{}", s);
        s
    }
}
impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ActionError: ({:?}, {:?}) - `{}`",
            self.device_uid, self.input_key, self.msg
        )
    }
}
pub type ActionResult = Result<ActionOk, ActionError>;
pub type ActionModuleConfig = Arc<Mutex<HashMap<String, String>>>;

// https://quinedot.github.io/rust-learning/dyn-trait-eq.html
trait AsDynCompare: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_dyn_compare(&self) -> &dyn DynCompare;
}

// Sized types only
impl<T: Any + DynCompare> AsDynCompare for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_dyn_compare(&self) -> &dyn DynCompare {
        self
    }
}
trait DynCompare: AsDynCompare {
    fn dyn_eq(&self, other: &dyn DynCompare) -> bool;
}
impl<T: Any + PartialEq> DynCompare for T {
    fn dyn_eq(&self, other: &dyn DynCompare) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            self == other
        } else {
            false
        }
    }
}
impl PartialEq<dyn DynCompare> for dyn DynCompare {
    fn eq(&self, other: &dyn DynCompare) -> bool {
        self.dyn_eq(other)
    }
}
impl PartialEq<dyn ActionTrait> for dyn ActionTrait {
    fn eq(&self, other: &dyn ActionTrait) -> bool {
        (&*self as &dyn DynCompare) == (&*other as &dyn DynCompare)
    }
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait ActionTrait: DowncastSync + DynCompare + DynClone {
    fn get_type(&self) -> &'static str;
    fn get_module(&self) -> &'static str;
    fn get_title(&self) -> &'static str;
    fn get_description(&self) -> &'static str;

    async fn on_press(
        &mut self,
        _module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
    ) -> ActionResult {
        Ok(ActionOk::new())
    }
    async fn on_release(
        &mut self,
        _module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
    ) -> ActionResult {
        Ok(ActionOk::new())
    }
    fn edit_ui(
        &mut self,
        _profiles: &(String, Vec<(String, String)>), // (current_profile_uuid, [(uuid, name), ...]) // kind of a hack just for MetaSwitchProfile, revisit later
        _module_config: ActionModuleConfig,
        _device_uid: &String,
        _input_key: &InputKey,
        _ui: &mut Ui,
    ) {
    }

    fn icon_state(&self) -> u8 {
        0
    }
    fn icon_state_icons(&self) -> &[ImageSource<'_>];
    fn icon_state_count(&self) -> u8 {
        1
    }
    fn icon_state_descriptions(&self) -> &[&str] {
        &[""]
    }

    fn icon_source(&'_ self) -> ImageSource<'_> {
        self.icon_state_icons()[self.icon_state() as usize].clone()
    }
    fn icon(&'_ self) -> Image<'_> {
        Image::new(self.icon_source())
            .texture_options(TextureOptions {
                magnification: TextureFilter::Nearest,
                minification: TextureFilter::Nearest,
                wrap_mode: TextureWrapMode::ClampToEdge,
                mipmap_mode: None,
            })
            .corner_radius(2.0)
    }
}
impl_downcast!(sync ActionTrait);
clone_trait_object!(ActionTrait);
pub type Action = Box<dyn ActionTrait>;
// TODO: differentiate between built-in and external actions

pub struct ActionMap {
    ui_list: Vec<(String, Vec<(String, String)>)>,
    enum_map: HashMap<String, Action>,
}
impl ActionMap {
    pub fn new(config: Arc<Mutex<JukeBoxConfig>>) -> Self {
        // this function is only safe to call once!
        // TODO: we should probably fix that...
        let (
            meta_module_config,
            input_module_config,
            system_module_config,
            discord_module_config,
            obs_module_config,
        ) = {
            let c = config.blocking_lock();
            let meta_module_config = Arc::new(Mutex::new(HashMap::new()));
            let input_module_config = Arc::new(Mutex::new(HashMap::new()));
            let system_module_config = Arc::new(Mutex::new(HashMap::new()));
            let discord_module_config = Arc::new(Mutex::new(
                c.action_module_config
                    .get(AMID_DISCORD)
                    .cloned()
                    .unwrap_or_default(),
            ));
            let obs_module_config = Arc::new(Mutex::new(
                c.action_module_config
                    .get(AMID_OBS)
                    .cloned()
                    .unwrap_or_default(),
            ));
            (
                meta_module_config,
                input_module_config,
                system_module_config,
                discord_module_config,
                obs_module_config,
            )
        };

        let l = vec![
            init_actions_meta(meta_module_config.clone()),
            init_actions_input(input_module_config.clone()),
            init_actions_system(system_module_config.clone()),
            #[cfg(feature = "discord")]
            init_actions_discord(discord_module_config.clone()),
            init_actions_obs(obs_module_config.clone()),
        ];

        {
            let mut c = config.blocking_lock();
            let _ = c.action_module_config.insert(
                AMID_DISCORD.into(),
                discord_module_config.blocking_lock().clone(),
            );
            let _ = c
                .action_module_config
                .insert(AMID_OBS.into(), obs_module_config.blocking_lock().clone());
            c.save();
        }

        let ui_list = l
            .iter()
            .map(|(title, l)| {
                (
                    title.clone(),
                    l.iter()
                        .map(|a| (a.get_type().into(), a.get_title().into()))
                        .collect(),
                )
            })
            .collect();

        let enum_map = l
            .iter()
            .map(|(_, l)| l)
            .flatten()
            .map(|a| (a.get_type().into(), a.clone()))
            .collect();

        Self { ui_list, enum_map }
    }

    pub fn ui_list(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.ui_list.clone()
    }

    pub fn enum_new(&self, t: String) -> Action {
        (self.enum_map.get(&t).unwrap()).clone()
    }

    fn keyboard_key(key: u8) -> ActionConfig {
        ActionConfig {
            action: Box::new(InputKeyboard { keys: vec![key] }),
            icons: vec![ActionIcon::DefaultActionIcon],
        }
    }

    pub fn default_action_config(d: DeviceType) -> HashMap<InputKey, ActionConfig> {
        use InputKey as IK;

        match d {
            DeviceType::KeyPad => HashMap::from([
                (IK::KeySwitch1, Self::keyboard_key(0x68)),
                (IK::KeySwitch2, Self::keyboard_key(0x69)),
                (IK::KeySwitch3, Self::keyboard_key(0x6A)),
                (IK::KeySwitch4, Self::keyboard_key(0x6B)),
                (IK::KeySwitch5, Self::keyboard_key(0x6C)),
                (IK::KeySwitch6, Self::keyboard_key(0x6D)),
                (IK::KeySwitch7, Self::keyboard_key(0x6E)),
                (IK::KeySwitch8, Self::keyboard_key(0x6F)),
                (IK::KeySwitch9, Self::keyboard_key(0x70)),
                (IK::KeySwitch10, Self::keyboard_key(0x71)),
                (IK::KeySwitch11, Self::keyboard_key(0x72)),
                (IK::KeySwitch12, Self::keyboard_key(0x73)),
            ]),
            DeviceType::KnobPad => HashMap::from([
                (IK::KnobLeftSwitch, ActionConfig::default()),
                (IK::KnobLeftClockwise, ActionConfig::default()),
                (IK::KnobLeftCounterClockwise, ActionConfig::default()),
                (IK::KnobRightSwitch, ActionConfig::default()),
                (IK::KnobRightClockwise, ActionConfig::default()),
                (IK::KnobRightCounterClockwise, ActionConfig::default()),
            ]),
            DeviceType::PedalPad => HashMap::from([
                (IK::PedalLeft, ActionConfig::default()),
                (IK::PedalMiddle, ActionConfig::default()),
                (IK::PedalRight, ActionConfig::default()),
            ]),
            DeviceType::Unknown => HashMap::new(),
        }
    }
}

pub fn get_icon_cache<'a>() -> MutexGuard<'a, HashMap<String, Vec<u8>>> {
    ICON_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .blocking_lock()
}

pub async fn get_icon_cache_async<'a>() -> MutexGuard<'a, HashMap<String, Vec<u8>>> {
    ICON_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .await
}

pub fn get_icon_bytes<'a>(
    action_config: &ActionConfig,
    icon_cache: &mut MutexGuard<'a, HashMap<String, Vec<u8>>>,
) -> [u8; 32 * 32 * 2] {
    let icon_state = action_config.action.icon_state();
    let icon = &action_config.icons[icon_state as usize];

    let b = match icon {
        ActionIcon::ImageIcon(i) => {
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
