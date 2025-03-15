// Defining actions to perform when actions happen (key pressed, knob turned, etc.)

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use futures::future::join_all;
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};

use crate::{config::JukeBoxConfig, input::InputKey, serial::SerialEvent};

pub async fn action_task(
    mut s_evnt_rx: UnboundedReceiver<SerialEvent>,
    config: Arc<Mutex<JukeBoxConfig>>,
) -> Result<()> {
    let mut prevkeys: HashMap<String, Arc<Mutex<HashSet<InputKey>>>> = HashMap::new();

    let clear_set = async |p: &mut HashMap<String, Arc<Mutex<HashSet<InputKey>>>>, uid: &String| {
        if !p.contains_key(uid) {
            p.insert(uid.clone(), Arc::new(Mutex::new(HashSet::new())));
        }
        let p = p.get_mut(uid).unwrap();
        p.lock().await.clear();
    };

    while let Some(evnt) = s_evnt_rx.recv().await {
        match evnt {
            SerialEvent::Connected { device_info } => {
                clear_set(&mut prevkeys, &device_info.device_uid).await;
            }
            SerialEvent::GetInputKeys { device_uid, keys } => {
                if !prevkeys.contains_key(&device_uid) {
                    prevkeys.insert(device_uid.clone(), Arc::new(Mutex::new(HashSet::new())));
                }
                let prevkeys = prevkeys.get(&device_uid).unwrap().clone();

                let config = config.clone();

                tokio::spawn(async move {
                    let current_profile = {
                        let c = config.lock().await; // Lock drops immediately

                        c.profiles
                            .get(&c.current_profile)
                            .and_then(|p| p.get(&device_uid))
                            .and_then(|p| Some(p.clone()))
                            .unwrap_or(HashMap::new())
                    };

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
