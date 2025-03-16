// Defining actions to perform when actions happen (key pressed, knob turned, etc.)

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use futures::future::join_all;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

use crate::{
    config::JukeBoxConfig,
    input::InputKey,
    serial::{SerialCommand, SerialEvent},
};

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
            .unwrap_or((HashMap::new(), None));

        (profile, rgb, c.current_profile.clone()) // TODO: add hardware input info
    };

    while let Some(evnt) = s_evnt_rx.recv().await {
        match evnt {
            SerialEvent::Connected { device_info } => {
                let device_uid = &device_info.device_uid;

                clear_set(&mut prevkeys, device_uid).await;

                // TODO: set RGB here
                // TODO: set hardware inputs here
                let (_, current_rgb_profile, _) = get_profile_info(&config, device_uid).await;
                let rgb_profile =
                    current_rgb_profile.unwrap_or(jukebox_util::color::RgbProfile::Off);
                let txs = scmd_txs.lock().await;
                let _ = txs
                    .get(device_uid)
                    .and_then(|t| Some(t.send(SerialCommand::SetRGB(rgb_profile))));
            }
            SerialEvent::GetInputKeys { device_uid, keys } => {
                if !prevkeys.contains_key(&device_uid) {
                    prevkeys.insert(device_uid.clone(), Arc::new(Mutex::new(HashSet::new())));
                }
                let prevkeys = prevkeys.get(&device_uid).unwrap().clone();

                let config = config.clone();
                let scmd_txs = scmd_txs.clone();

                tokio::spawn(async move {
                    let (current_profile, _, current_profile_name) =
                        get_profile_info(&config, &device_uid).await;

                    let mut prevkeys = prevkeys.lock().await;

                    let pressed = keys.difference(&prevkeys);
                    let released = prevkeys.difference(&keys);

                    let mut futures = Vec::new();

                    for p in pressed {
                        if let Some(r) = current_profile.get(p) {
                            futures.push(r.on_press(&device_uid, *p, config.clone()));
                        }
                    }

                    for p in released {
                        if let Some(r) = current_profile.get(p) {
                            futures.push(r.on_release(&device_uid, *p, config.clone()));
                        }
                    }

                    for _res in join_all(futures).await {
                        // TODO: error signaling
                    }

                    *prevkeys = keys;

                    let (_, new_rgb_profile, new_profile_name) =
                        get_profile_info(&config, &device_uid).await;

                    if current_profile_name != new_profile_name {
                        let rgb_profile =
                            new_rgb_profile.unwrap_or(jukebox_util::color::RgbProfile::Off);
                        let txs = scmd_txs.lock().await;
                        let _ = txs
                            .get(&device_uid)
                            .and_then(|t| Some(t.send(SerialCommand::SetRGB(rgb_profile))));

                        // TODO: set hardware inputs here if profile changes
                    }
                });
            }
            SerialEvent::LostConnection { device_uid } => {
                clear_set(&mut prevkeys, &device_uid).await;
            }
            SerialEvent::Disconnected { device_uid } => {
                clear_set(&mut prevkeys, &device_uid).await;
            }
            _ => {}
        }
    }

    Ok(())
}
