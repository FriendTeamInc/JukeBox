// Defining reactions to perform when actions happen (key pressed, knob turned, etc.)

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};

use crate::{config::JukeBoxConfig, input::InputKey, serial::SerialEvent};

pub async fn reaction_task(
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

    // TODO: have discord and obs clients set up here for specific reactions to trigger with

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
                    let mut c = config.lock().await.clone(); // Lock drops immediately
                    let current_profile = c
                        .profiles
                        .get(&c.current_profile)
                        .and_then(|p| p.get(&device_uid))
                        .and_then(|p| Some(p.clone()))
                        .unwrap_or(HashMap::new());

                    let mut prevkeys = prevkeys.lock().await;

                    let pressed = keys.difference(&prevkeys);
                    let released = prevkeys.difference(&keys);

                    // TODO: error signaling
                    // TODO: batching futures?

                    for p in pressed {
                        if let Some(r) = current_profile.get(p) {
                            let _ = r.on_press(&device_uid, *p, &mut c).await;
                        }
                    }

                    for p in released {
                        if let Some(r) = current_profile.get(p) {
                            let _ = r.on_release(&device_uid, *p, &mut c).await;
                        }
                    }

                    *prevkeys = keys;

                    // the current profile is the only field that actions can modify currently.
                    let mut config = config.lock().await;
                    config.current_profile = c.current_profile.clone();
                    config.save();
                    // TODO: allow for editing of global configs for things like Discord and OBS
                });
            }
            SerialEvent::LostConnection { device_uid } => {
                clear_set(&mut prevkeys, &device_uid).await;
            }
            SerialEvent::Disconnected { device_uid } => {
                clear_set(&mut prevkeys, &device_uid).await;
            } // _ => {}
        }
    }

    Ok(())
}
