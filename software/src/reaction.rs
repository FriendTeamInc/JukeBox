// Defining reactions to perform when actions happen (key pressed, knob turned, etc.)

use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::AtomicBool, mpsc::Receiver, Arc, Mutex},
    thread::yield_now,
    time::{Duration, Instant},
};

use anyhow::Result;

use crate::{
    config::JukeBoxConfig, input::InputKey, reactions::types::Reaction, serial::SerialEvent,
};

fn run_key(reaction_config: &Box<dyn Reaction>, key: InputKey, pressed: bool) {
    // we cannot allow any errors or panics to proceed past this point.
    // TODO: figure out how to *do* that

    match pressed {
        true => reaction_config.on_press(key),
        false => reaction_config.on_release(key),
    }
}

pub fn reaction_task(
    brkr: Arc<AtomicBool>,
    s_evnt_rx: Receiver<SerialEvent>,
    config: Arc<Mutex<JukeBoxConfig>>,
) -> Result<()> {
    let mut prevkeys: HashMap<String, HashSet<InputKey>> = HashMap::new();

    let mut timer = Instant::now();
    loop {
        if Instant::now() < timer {
            yield_now();
            continue;
        }
        timer = Instant::now() + Duration::from_millis(1);

        while let Ok(evnt) = s_evnt_rx.try_recv() {
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

                    let current_profile = {
                        let c = config.lock().unwrap();
                        c.profiles
                            .get(&device_uid)
                            .and_then(|p| p.get(&c.current_profile))
                            .and_then(|p| Some(p.clone()))
                            .unwrap_or(HashMap::new())
                    };

                    let pressed = keys.difference(&prevkeys);
                    let released = prevkeys.difference(&keys);

                    for p in pressed {
                        if let Some(r) = current_profile.get(&p) {
                            let _ = run_key(r, *p, true);
                        }
                    }

                    for p in released {
                        if let Some(r) = current_profile.get(&p) {
                            let _ = run_key(r, *p, false);
                        }
                    }

                    *prevkeys = keys;
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
                }
                _ => {}
            }
        }

        if brkr.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
    }

    Ok(())
}
