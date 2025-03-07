// Defining reactions to perform when actions happen (key pressed, knob turned, etc.)

use std::{
    collections::{HashMap, HashSet},
    sync::{mpsc::Receiver, Arc, Mutex},
};

use anyhow::Result;

use crate::{config::JukeBoxConfig, input::InputKey, serial::SerialEvent};

pub fn reaction_task(
    s_evnt_rx: Receiver<SerialEvent>,
    config: Arc<Mutex<JukeBoxConfig>>,
) -> Result<()> {
    let mut prevkeys: HashMap<String, HashSet<InputKey>> = HashMap::new();

    // TODO: have discord and obs clients set up here for specific reactions to trigger with

    while let Ok(evnt) = s_evnt_rx.recv() {
        match evnt {
            SerialEvent::Connected(device_info) => {
                if !prevkeys.contains_key(&device_info.device_uid) {
                    prevkeys.insert(device_info.device_uid.clone(), HashSet::new());
                }
                let prevkeys = prevkeys.get_mut(&device_info.device_uid).unwrap();
                prevkeys.clear();
            }
            SerialEvent::GetInputKeys((device_uid, keys)) => {
                if !prevkeys.contains_key(&device_uid) {
                    prevkeys.insert(device_uid.clone(), HashSet::new());
                }
                let prevkeys = prevkeys.get_mut(&device_uid).unwrap();

                let mut c = config.lock().unwrap().clone();

                let current_profile = c
                    .profiles
                    .get(&c.current_profile)
                    .and_then(|p| p.get(&device_uid))
                    .and_then(|p| Some(p.clone()))
                    .unwrap_or(HashMap::new());

                let pressed = keys.difference(&prevkeys);
                let released = prevkeys.difference(&keys);

                // TODO: error signaling

                for p in pressed {
                    if let Some(r) = current_profile.get(p) {
                        let _ = r.on_press(&device_uid, *p, &mut c);
                    }
                }

                for p in released {
                    if let Some(r) = current_profile.get(p) {
                        let _ = r.on_release(&device_uid, *p, &mut c);
                    }
                }

                *prevkeys = keys;

                // the current profile is the only field that actions can modify currently.
                let mut config = config.lock().unwrap();
                config.current_profile = c.current_profile.clone();
                config.save();
                // TODO: allow for editing of global configs for things like Discord and OBS
            }
            SerialEvent::LostConnection(device_uid) => {
                if !prevkeys.contains_key(&device_uid) {
                    prevkeys.insert(device_uid.clone(), HashSet::new());
                }
                let prevkeys = prevkeys.get_mut(&device_uid).unwrap();
                prevkeys.clear();
            }
            SerialEvent::Disconnected(device_uid) => {
                if !prevkeys.contains_key(&device_uid) {
                    prevkeys.insert(device_uid.clone(), HashSet::new());
                }
                let prevkeys = prevkeys.get_mut(&device_uid).unwrap();
                prevkeys.clear();
            } // _ => {}
        }
    }

    Ok(())
}
