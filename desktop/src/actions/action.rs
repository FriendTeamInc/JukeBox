// Defining actions to perform when actions happen (key pressed, knob turned, etc.)

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use jukebox_util::{
    input::InputEvent, peripheral::DeviceType, rgb::RgbProfile, screen::ScreenProfile,
};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

use crate::{
    actions::{
        input::{InputKeyboard, InputMouse},
        types::{get_icon_bytes, get_icon_cache_async, Action, ActionError, ActionModuleConfig},
    },
    config::{ActionConfig, JukeBoxConfig},
    input::InputKey,
    serial::{SerialCommand, SerialEvent},
};

async fn update_device_configs(
    tx: UnboundedSender<SerialCommand>,
    device_type: DeviceType,
    keys: HashMap<InputKey, ActionConfig>,
    profile_name: String,
    rgb_profile: RgbProfile,
    screen_profile: ScreenProfile,
) {
    if device_type == DeviceType::KeyPad {
        // send profile name
        let _ = tx.send(SerialCommand::SetProfileName(profile_name));

        // send rgb profile
        let _ = tx.send(SerialCommand::SetRgbMode(rgb_profile));

        // send screen profile
        let _ = tx.send(SerialCommand::SetScrMode(screen_profile));

        // set icons on screen
        for (k, a) in &keys {
            send_scr_icon(&tx, a, k).await;
        }
    }

    for (k, a) in keys {
        let slot = k.slot();
        send_input_event(&tx, slot, &a.action);
    }
}

async fn send_scr_icon(
    tx: &UnboundedSender<SerialCommand>,
    action_config: &ActionConfig,
    input_key: &InputKey,
) {
    let bytes = get_icon_bytes(action_config, &mut get_icon_cache_async().await);
    let _ = tx.send(SerialCommand::SetScrIcon(input_key.slot(), bytes));
}

pub fn send_input_event(tx: &UnboundedSender<SerialCommand>, slot: u8, action: &Action) {
    let _ = if action.is::<InputKeyboard>() {
        let a = action.downcast_ref::<InputKeyboard>().unwrap();
        tx.send(SerialCommand::SetInputEvent(slot, a.get_input_event()))
    } else if action.is::<InputMouse>() {
        let a = action.downcast_ref::<InputMouse>().unwrap();
        tx.send(SerialCommand::SetInputEvent(slot, a.get_input_event()))
    } else {
        tx.send(SerialCommand::SetInputEvent(slot, InputEvent::default()))
    };
}

async fn clear_set(p: &mut HashMap<String, Arc<Mutex<HashSet<InputKey>>>>, uid: &String) {
    if !p.contains_key(uid) {
        p.insert(uid.clone(), Arc::new(Mutex::new(HashSet::new())));
    }
    let p = p.get_mut(uid).unwrap();
    p.lock().await.clear();
}

async fn get_profile_info(
    config: &Arc<Mutex<JukeBoxConfig>>,
    device_uid: &String,
) -> (
    DeviceType,
    HashMap<InputKey, ActionConfig>,
    String,
    HashMap<String, ActionModuleConfig>,
    Option<RgbProfile>,
    Option<ScreenProfile>,
) {
    let c = config.lock().await;

    let (profile, rgb, scr) = c
        .profiles
        .get(&c.current_profile)
        .and_then(|p| p.device_configs.get(device_uid))
        .map(|p| {
            (
                p.key_map.clone(),
                p.rgb_profile.clone(),
                p.screen_profile.clone(),
            )
        })
        .unwrap_or((HashMap::new(), None, None));

    let module_configs = c
        .action_module_config
        .clone()
        .iter()
        .map(|(k, v)| (k.clone(), Arc::new(Mutex::new(v.clone()))))
        .collect();

    let device_type = c
        .devices
        .get(device_uid)
        .map(|d| d.device_type)
        .unwrap_or(DeviceType::Unknown)
        .clone();

    (
        device_type,
        profile,
        c.current_profile.clone(),
        module_configs,
        rgb,
        scr,
    )
}

pub async fn action_task(
    mut s_evnt_rx: UnboundedReceiver<SerialEvent>,
    config: Arc<Mutex<JukeBoxConfig>>,
    scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
    ae_tx: UnboundedSender<ActionError>,
) -> Result<()> {
    let mut prevkeys: HashMap<String, Arc<Mutex<HashSet<InputKey>>>> = HashMap::new();

    while let Some(evnt) = s_evnt_rx.recv().await {
        match evnt {
            SerialEvent::Connected {
                device_uid,
                firmware_version: _,
                device_type: _,
            } => {
                clear_set(&mut prevkeys, &device_uid).await;

                let scmd_tx = {
                    if let Some(tx) = scmd_txs.lock().await.get(&device_uid) {
                        tx.clone()
                    } else {
                        log::warn!("failed to find serial command sender for {}", device_uid);
                        continue;
                    }
                };

                let (device_type, keys, profile_uuid, _, rgb_profile, screen_profile) =
                    get_profile_info(&config, &device_uid).await;
                update_device_configs(
                    scmd_tx,
                    device_type,
                    keys,
                    profile_uuid,
                    rgb_profile.unwrap_or(RgbProfile::default_gui_profile()),
                    screen_profile.unwrap_or(ScreenProfile::default_profile()),
                )
                .await;
            }
            SerialEvent::GetInputKeys { device_uid, keys } => {
                if !prevkeys.contains_key(&device_uid) {
                    prevkeys.insert(device_uid.clone(), Arc::new(Mutex::new(HashSet::new())));
                }
                let prevkeys = prevkeys.get(&device_uid).unwrap().clone();

                let config = config.clone();
                let scmd_tx = {
                    if let Some(tx) = scmd_txs.lock().await.get(&device_uid) {
                        tx.clone()
                    } else {
                        log::warn!("failed to find serial command sender for {}", device_uid);
                        continue;
                    }
                };
                let ae_tx = ae_tx.clone();

                tokio::spawn(async move {
                    let (_, mut profile, profile_uuid, mut module_configs, _, _) =
                        get_profile_info(&config, &device_uid).await;

                    let mut prevkeys = prevkeys.lock().await;

                    let pressed = keys.difference(&prevkeys);
                    let released = prevkeys.difference(&keys);

                    let mut new_profile = None;

                    for k in pressed {
                        let Some(p) = profile.remove(k) else { continue };

                        let m = module_configs
                            .get(p.action.get_module())
                            .cloned()
                            .unwrap_or_default();
                        let _ = module_configs.insert(p.action.get_module().into(), m.clone());
                        let i = p.icons;
                        let mut a = p.action;

                        // TODO: restore join/join_all futures

                        match a.on_press(m, &device_uid, k).await {
                            Ok(o) => {
                                if o.save_action_config {
                                    let mut c = config.lock().await;
                                    let p = c.profiles.get_mut(&profile_uuid).unwrap();
                                    let d = p.device_configs.get_mut(&device_uid).unwrap();
                                    let k = d.key_map.get_mut(&k).unwrap();
                                    *k = ActionConfig {
                                        action: a,
                                        icons: i,
                                    };
                                    c.save();
                                }
                                if o.save_module_config {
                                    let a = profile.get(&k).unwrap();
                                    let mut c = config.lock().await;
                                    let amid = a.action.get_module().to_string();
                                    let m = module_configs.get(&amid).unwrap().lock().await;
                                    let _ = c.action_module_config.insert(amid, m.clone());
                                    c.save();
                                }
                                if let Some(p) = o.switch_to_profile {
                                    new_profile = Some(p);
                                }
                                if o.change_icon {
                                    if let Some(a) = profile.get(&k) {
                                        send_scr_icon(&scmd_tx, a, &k).await;
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = ae_tx.send(e);
                            }
                        }
                    }

                    for k in released {
                        let Some(r) = profile.remove(k) else { continue };

                        let m = module_configs
                            .get(r.action.get_module())
                            .cloned()
                            .unwrap_or_default();
                        let _ = module_configs.insert(r.action.get_module().into(), m.clone());
                        let i = r.icons;
                        let mut a = r.action;

                        match a.on_release(m, &device_uid, k).await {
                            Ok(o) => {
                                if o.save_action_config {
                                    let mut c = config.lock().await;
                                    let p = c.profiles.get_mut(&profile_uuid).unwrap();
                                    let d = p.device_configs.get_mut(&device_uid).unwrap();
                                    let k = d.key_map.get_mut(&k).unwrap();
                                    *k = ActionConfig {
                                        action: a,
                                        icons: i,
                                    };
                                    c.save();
                                }
                                if o.save_module_config {
                                    let a = profile.get(&k).unwrap();
                                    let mut c = config.lock().await;
                                    let amid = a.action.get_module().to_string();
                                    let m = module_configs.get(&amid).unwrap().lock().await;
                                    let _ = c.action_module_config.insert(amid, m.clone());
                                    c.save();
                                }
                                if let Some(p) = o.switch_to_profile {
                                    new_profile = Some(p);
                                }
                                if o.change_icon {
                                    if let Some(a) = profile.get(&k) {
                                        send_scr_icon(&scmd_tx, a, &k).await;
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = ae_tx.send(e);
                            }
                        }
                    }

                    *prevkeys = keys;

                    if let Some(p) = new_profile {
                        let found_profile = {
                            let mut c = config.lock().await;
                            if let Some(_) = c.profiles.get(&p) {
                                c.current_profile = p.clone();
                                c.save();
                                true
                            } else {
                                false
                            }
                        };

                        if found_profile {
                            let (
                                device_type,
                                new_keys,
                                new_profile_name,
                                _,
                                new_rgb_profile,
                                new_screen_profile,
                            ) = get_profile_info(&config, &device_uid).await;
                            update_device_configs(
                                scmd_tx,
                                device_type,
                                new_keys,
                                new_profile_name,
                                new_rgb_profile.unwrap_or(RgbProfile::default_gui_profile()),
                                new_screen_profile.unwrap_or(ScreenProfile::default_profile()),
                            )
                            .await;
                        } else {
                            let _ = ae_tx.send(ActionError::msg(t!(
                                "action.meta.switch_profile.err.profile_not_found",
                                profile = p
                            )));
                        }
                    }
                });
            }
            SerialEvent::LostConnection { device_uid } => {
                clear_set(&mut prevkeys, &device_uid).await;
            }
            SerialEvent::Disconnected { device_uid } => {
                clear_set(&mut prevkeys, &device_uid).await;
            }
        }
    }

    Ok(())
}
