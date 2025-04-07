// Defining actions to perform when actions happen (key pressed, knob turned, etc.)

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use futures::future::join_all;
use jukebox_util::{input::KeyboardEvent, peripheral::DeviceType, rgb::RgbProfile};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

use crate::{
    config::{ActionConfig, JukeBoxConfig},
    input::InputKey,
    serial::{SerialCommand, SerialEvent},
};

use super::{
    input::{InputKeyboard, InputMouse},
    types::{get_icon_bytes, ActionError},
};

async fn update_device_configs(
    scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
    device_uid: &String,
    device_type: DeviceType,
    keys: HashMap<InputKey, ActionConfig>,
    rgb_profile: RgbProfile,
) {
    let txs = scmd_txs.lock().await;
    let tx = if let Some(tx) = txs.get(device_uid) {
        tx
    } else {
        return;
    };

    if device_type == DeviceType::KeyPad {
        // send rgb profile
        let _ = tx.send(SerialCommand::SetRgbMode(rgb_profile));

        // set icons on screen
        for (k, a) in &keys {
            let bytes = get_icon_bytes(a);
            let _ = tx.send(SerialCommand::SetScrIcon(k.slot(), bytes));
        }

        for (k, a) in keys {
            let slot = k.slot();
            let action = a.action.as_any();
            let _ = if let Some(kb) = action.downcast_ref::<InputKeyboard>() {
                tx.send(SerialCommand::SetKeyboardInput(
                    slot,
                    kb.get_keyboard_event(),
                ))
            } else if let Some(mouse) = action.downcast_ref::<InputMouse>() {
                tx.send(SerialCommand::SetMouseInput(slot, mouse.get_mouse_event()))
            } else {
                tx.send(SerialCommand::SetKeyboardInput(
                    slot,
                    KeyboardEvent::empty_event(),
                ))
            };
        }
    }
}

pub async fn action_task(
    mut s_evnt_rx: UnboundedReceiver<SerialEvent>,
    config: Arc<Mutex<JukeBoxConfig>>,
    scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
    ae_tx: UnboundedSender<ActionError>,
) -> Result<()> {
    let mut prevkeys: HashMap<String, Arc<Mutex<HashSet<InputKey>>>> = HashMap::new();

    let clear_set = async |p: &mut HashMap<String, Arc<Mutex<HashSet<InputKey>>>>, uid: &String| {
        if !p.contains_key(uid) {
            p.insert(uid.clone(), Arc::new(Mutex::new(HashSet::new())));
        }
        let p = p.get_mut(uid).unwrap();
        p.lock().await.clear();
    };

    let get_profile_info = async |config: &Arc<Mutex<JukeBoxConfig>>, device_uid: &String| {
        let c = config.lock().await; // Lock drops immediately

        let (profile, rgb) = c
            .profiles
            .get(&c.current_profile)
            .and_then(|p| p.get(device_uid))
            .map(|p| (p.key_map.clone(), p.rgb_profile.clone()))
            .unwrap_or((HashMap::new(), None));

        let device_type = c
            .devices
            .get(device_uid)
            .map(|d| d.device_type)
            .unwrap()
            .clone();

        (device_type, profile, rgb, c.current_profile.clone()) // TODO: add hardware input info
    };

    while let Some(evnt) = s_evnt_rx.recv().await {
        match evnt {
            SerialEvent::Connected { device_info } => {
                let device_uid = &device_info.device_uid;

                clear_set(&mut prevkeys, device_uid).await;

                // TODO: set hardware inputs here
                let (device_type, keys, rgb_profile, _) =
                    get_profile_info(&config, device_uid).await;
                update_device_configs(
                    scmd_txs.clone(),
                    device_uid,
                    device_type,
                    keys,
                    rgb_profile.unwrap_or(RgbProfile::default_gui_profile()),
                )
                .await;
            }
            SerialEvent::GetInputKeys { device_uid, keys } => {
                if !prevkeys.contains_key(&device_uid) {
                    prevkeys.insert(device_uid.clone(), Arc::new(Mutex::new(HashSet::new())));
                }
                let prevkeys = prevkeys.get(&device_uid).unwrap().clone();

                let config = config.clone();
                let scmd_txs = scmd_txs.clone();
                let ae_tx = ae_tx.clone();

                tokio::spawn(async move {
                    let (_, current_profile, _, current_profile_name) =
                        get_profile_info(&config, &device_uid).await;

                    let mut prevkeys = prevkeys.lock().await;

                    let pressed = keys.difference(&prevkeys);
                    let released = prevkeys.difference(&keys);

                    let mut futures = Vec::new();

                    for p in pressed {
                        if let Some(r) = current_profile.get(p) {
                            futures.push(r.action.on_press(&device_uid, *p, config.clone()));
                        }
                    }

                    for p in released {
                        if let Some(r) = current_profile.get(p) {
                            futures.push(r.action.on_release(&device_uid, *p, config.clone()));
                        }
                    }

                    for res in join_all(futures).await {
                        match res {
                            Ok(_) => {}
                            Err(e) => {
                                let _ = ae_tx.send(e);
                            }
                        }
                    }

                    *prevkeys = keys;

                    let (device_type, new_keys, new_rgb_profile, new_profile_name) =
                        get_profile_info(&config, &device_uid).await;

                    if current_profile_name != new_profile_name {
                        update_device_configs(
                            scmd_txs.clone(),
                            &device_uid,
                            device_type,
                            new_keys,
                            new_rgb_profile.unwrap_or(RgbProfile::default_gui_profile()),
                        )
                        .await;
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
