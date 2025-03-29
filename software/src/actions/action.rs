// Defining actions to perform when actions happen (key pressed, knob turned, etc.)

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use futures::future::join_all;
use jukebox_util::{color::RgbProfile, peripheral::DeviceType};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

use crate::{
    config::{ActionConfig, JukeBoxConfig},
    input::InputKey,
    serial::{SerialCommand, SerialEvent},
};

use super::types::get_icon_bytes;

pub async fn action_task(
    mut s_evnt_rx: UnboundedReceiver<SerialEvent>,
    config: Arc<Mutex<JukeBoxConfig>>,
    scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
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
            .and_then(|p| Some(p.clone()))
            .and_then(|p| Some((p.key_map, p.rgb_profile)))
            .unwrap_or((HashMap::new(), None));

        let device_type = c
            .devices
            .get(device_uid)
            .and_then(|d| Some(d.0))
            .unwrap()
            .clone();

        (device_type, profile, rgb, c.current_profile.clone()) // TODO: add hardware input info
    };

    let update_device_configs =
        async |scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
               device_uid: &String,
               device_type: DeviceType,
               keys: HashMap<InputKey, ActionConfig>,
               rgb_profile: RgbProfile| {
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
                let slots = [
                    InputKey::KeySwitch1,
                    InputKey::KeySwitch2,
                    InputKey::KeySwitch3,
                    InputKey::KeySwitch4,
                    InputKey::KeySwitch5,
                    InputKey::KeySwitch6,
                    InputKey::KeySwitch7,
                    InputKey::KeySwitch8,
                    InputKey::KeySwitch9,
                    InputKey::KeySwitch10,
                    InputKey::KeySwitch11,
                    InputKey::KeySwitch12,
                ];

                for (i, k) in slots.iter().enumerate() {
                    if let Some(a) = keys.get(k) {
                        let bytes = get_icon_bytes(a.action.icon_source());
                        let _ = tx.send(SerialCommand::SetScrIcon(i as u8, bytes));
                    }
                }
            }

            // TODO: set hardware inputs here
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

                    for _res in join_all(futures).await {
                        // TODO: error signaling
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
