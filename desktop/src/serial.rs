// Serial communication to JukeBox devices
// The main task launches new tasks for each device connected

use crate::input::InputKey;

use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use jukebox_util::protocol::{CMD_SET_PROFILE_NAME, CMD_SET_SCR_MODE, CMD_SET_SYSTEM_STATS};
use jukebox_util::screen::{ProfileName, ScreenProfile, PROFILE_NAME_CHAR_LEN};
use jukebox_util::stats::SystemStats;
use jukebox_util::{
    input::{KeyboardEvent, MouseEvent},
    peripheral::{
        KeyInputs, KnobInputs, PedalInputs, IDENT_KEY_INPUT, IDENT_KNOB_INPUT, IDENT_PEDAL_INPUT,
        IDENT_UNKNOWN_INPUT,
    },
    protocol::{
        decode_packet_size, encode_packet_size, CMD_DISCONNECT, CMD_GET_INPUT_KEYS, CMD_GREET,
        CMD_IDENTIFY, CMD_NEGATIVE_ACK, CMD_SET_KEYBOARD_INPUT, CMD_SET_MOUSE_INPUT,
        CMD_SET_RGB_MODE, CMD_SET_SCR_ICON, CMD_UPDATE, RSP_ACK, RSP_DISCONNECTED,
        RSP_INPUT_HEADER, RSP_LINK_DELIMITER, RSP_LINK_HEADER, RSP_UNKNOWN,
    },
    rgb::RgbProfile,
};
use serialport::SerialPort;
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    task::{block_in_place, spawn_blocking},
    time::sleep,
};

#[derive(Debug, PartialEq, Clone)]
pub struct SerialConnectionDetails {
    pub input_identifier: u8,
    pub firmware_version: String,
    pub device_uid: String,
}

#[allow(unused)]
pub enum SerialCommand {
    Identify,
    SetKeyboardInput(u8, KeyboardEvent),
    SetMouseInput(u8, MouseEvent),
    // SetGamepadInput(u8, [u8; 6]),
    SetRgbMode(RgbProfile),
    SetScrIcon(u8, [u8; 32 * 32 * 2]),
    SetScrMode(ScreenProfile),
    SetProfileName(String),
    Update,
    Disconnect,
}

#[derive(PartialEq, Clone)]
pub enum SerialEvent {
    Connected {
        device_info: SerialConnectionDetails,
    },
    GetInputKeys {
        device_uid: String,
        keys: HashSet<InputKey>,
    },
    LostConnection {
        device_uid: String,
    },
    Disconnected {
        device_uid: String,
    },
}

type Serial = Box<dyn SerialPort>;

async fn get_serial_string(f: &mut Serial) -> Result<Vec<u8>> {
    block_in_place(|| {
        let timeout = Instant::now() + Duration::from_secs(3);
        let mut buf = Vec::new();
        let mut size = 0usize;

        loop {
            if Instant::now() >= timeout {
                bail!("serial read timed out (current buffer: {:?})", buf);
            }

            let mut b = [0u8; 1];
            let res = f.read(&mut b);

            match res {
                Err(e) => match e.kind() {
                    std::io::ErrorKind::BrokenPipe => bail!("broken serial pipe"),
                    _ => continue,
                },
                _ => (),
            }
            buf.push(b[0]);

            if buf.len() >= 3 {
                if size == 0 {
                    let w1 = buf[0];
                    let w2 = buf[1];
                    let w3 = buf[2];
                    size = decode_packet_size(w1, w2, w3).map_err(|_| {
                        anyhow!("failed to decode packet size {} {} {}", w1, w2, w3)
                    })?;
                } else {
                    size -= 1;
                    if size == 0 {
                        buf.remove(0);
                        buf.remove(0);
                        buf.remove(0);
                        return Ok(buf);
                    }
                }
            }
        }
    })
}

async fn send_cmd(f: &mut Serial, c: u8) -> Result<()> {
    send_bytes(f, &[c])
        .await
        .with_context(|| format!("failed to send cmd {}", c))
}

async fn send_bytes(f: &mut Serial, bytes: &[u8]) -> Result<()> {
    block_in_place(|| {
        let size = &encode_packet_size(bytes.len())
            .map_err(|_| anyhow!("failed to encode packet size {}", bytes.len()))?;

        // log::trace!(
        //     "send_bytes: {} ({:?}) {:?}",
        //     decode_packet_size(size[0], size[1], size[2]),
        //     size,
        //     bytes
        // );

        f.write_all(size)
            .with_context(|| format!("failed to write message size for {:?}", bytes))?;
        f.write_all(bytes)
            .with_context(|| format!("failed to write message {:?}", bytes))?;
        f.flush().context("failed to flush message")?;

        Ok(())
    })
}

async fn expect_string(f: &mut Serial, expect: &[u8]) -> Result<()> {
    let s = get_serial_string(f)
        .await
        .with_context(|| format!("expected string: {:?}", expect))?;

    let matching = s.iter().zip(expect).filter(|&(a, b)| a == b).count() == s.len();

    if !matching {
        let matches_unknown = s
            .iter()
            .zip([RSP_UNKNOWN])
            .filter(|&(a, b)| *a == b)
            .count()
            == s.len();
        if matches_unknown {
            send_negative_ack(f).await?;
            bail!("device did not understand command");
        }

        bail!("expect mismatch (expected {:?}, got {:?}", expect, s);
    }

    Ok(())
}

async fn send_expect(f: &mut Serial, send: &[u8], expect: &[u8]) -> Result<()> {
    send_bytes(f, send)
        .await
        .with_context(|| format!("failed to send bytes in send/expect"))?;
    expect_string(f, expect)
        .await
        .with_context(|| format!("failed to get expected bytes in send/expect"))?;
    Ok(())
}

// Tasks

async fn send_negative_ack(f: &mut Serial) -> Result<()> {
    send_cmd(f, CMD_NEGATIVE_ACK)
        .await
        .context("failed to send nack")?;
    Ok(())
}

async fn greet_host(f: &mut Serial) -> Result<SerialConnectionDetails> {
    // Host confirms protocol is good, recieves "link established" with some info about the device
    send_cmd(f, CMD_GREET)
        .await
        .context("failed to send greet")?;
    let resp = get_serial_string(f).await.context("expected greeting")?;

    if *resp.iter().nth(0).unwrap_or(&0) != RSP_LINK_HEADER {
        send_negative_ack(f).await?;
        bail!("failed to parse device info (command character mismatch)");
    }

    let mut input_identifier = None;
    let mut firmware_version = None;
    let mut device_uid = None;
    for (i, s) in resp.split(|c| *c == RSP_LINK_DELIMITER).enumerate() {
        if i == 1 {
            input_identifier = Some(s.get(0).unwrap_or(&IDENT_UNKNOWN_INPUT));
        } else if i == 2 {
            firmware_version = Some(s);
        } else if i == 3 {
            device_uid = Some(s);
        }
    }

    if input_identifier.is_none() || firmware_version.is_none() || device_uid.is_none() {
        send_negative_ack(f).await?;
        bail!("failed to parse device info (missing input identifier, firmware version, or device uid)");
    }

    let firmware_version = match String::from_utf8(firmware_version.unwrap().to_vec()) {
        Ok(s) => s,
        Err(_) => {
            send_negative_ack(f).await?;
            bail!("failed to parse device info (failed to convert firmware version to utf-8)");
        }
    };
    let device_uid = match String::from_utf8(device_uid.unwrap().to_vec()) {
        Ok(s) => s,
        Err(_) => {
            send_negative_ack(f).await?;
            bail!("failed to parse device info (failed to convert device uid to utf-8)");
        }
    };

    Ok(SerialConnectionDetails {
        input_identifier: *input_identifier.unwrap(),
        firmware_version: firmware_version,
        device_uid: device_uid,
    })
}

async fn transmit_get_input_keys(f: &mut Serial) -> Result<HashSet<InputKey>> {
    send_cmd(f, CMD_GET_INPUT_KEYS)
        .await
        .context("failed to send get input keys")?;
    let resp = get_serial_string(f).await.context("expected input keys")?;

    if *resp.get(0).unwrap_or(&0) != RSP_INPUT_HEADER {
        log::info!("rsp input: {:?}", resp);
        send_negative_ack(f).await?;
        bail!("failed to parse input keys (command character mismatch)");
    }

    let mut result = HashSet::new();
    let mut i = resp.iter();
    loop {
        match i.next() {
            Some(c) => match *c {
                IDENT_KEY_INPUT => {
                    let w2 = i.next();
                    let w1 = i.next();
                    if w2.is_none() || w1.is_none() {
                        bail!("failed to parse input keys (missing keyboard words)");
                    }
                    let keypad = KeyInputs::decode(&[*c, *w2.unwrap(), *w1.unwrap()])
                        .map_err(|_| anyhow!("failed to decode key inputs"))?;
                    result.extend(InputKey::trans_keys(keypad));
                }
                IDENT_KNOB_INPUT => {
                    let w = i.next();
                    if w.is_none() {
                        bail!("failed to parse input keys (missing knob 1 word)");
                    }
                    let knobpad = KnobInputs::decode(&[*c, *w.unwrap()])
                        .map_err(|_| anyhow!("failed to decode knob inputs"))?;
                    result.extend(InputKey::trans_knob(knobpad));
                }
                IDENT_PEDAL_INPUT => {
                    let w = i.next();
                    if w.is_none() {
                        bail!("failed to parse input keys (missing pedal 1 word)");
                    }
                    let pedalpad = PedalInputs::decode(&[*c, *w.unwrap()])
                        .map_err(|_| anyhow!("failed to decode pedal inputs"))?;
                    result.extend(InputKey::trans_pedals(pedalpad));
                }
                _ => {}
            },
            None => break,
        }
    }

    Ok(result)
}

async fn transmit_set_keyboard_input(f: &mut Serial, slot: u8, event: KeyboardEvent) -> Result<()> {
    let mut cmd = vec![CMD_SET_KEYBOARD_INPUT, slot];
    cmd.extend_from_slice(&event.encode());

    send_expect(f, &cmd, &[RSP_ACK]).await
}

async fn transmit_set_mouse_input(f: &mut Serial, slot: u8, event: MouseEvent) -> Result<()> {
    let mut cmd = vec![CMD_SET_MOUSE_INPUT, slot];
    cmd.extend_from_slice(&event.encode());

    send_expect(f, &cmd, &[RSP_ACK]).await
}

async fn transmit_set_rgb_mode(f: &mut Serial, rgb_profile: RgbProfile) -> Result<()> {
    let mut cmd = vec![CMD_SET_RGB_MODE];
    cmd.extend_from_slice(&rgb_profile.encode());

    send_expect(f, &cmd, &[RSP_ACK]).await
}

async fn transmit_set_scr_icon(
    f: &mut Serial,
    slot: u8,
    icon_data: [u8; 32 * 32 * 2],
) -> Result<()> {
    let mut cmd = vec![CMD_SET_SCR_ICON, slot];
    cmd.extend_from_slice(&icon_data);

    send_expect(f, &cmd, &[RSP_ACK]).await
}

async fn transmit_set_screen_mode(f: &mut Serial, screen_profile: ScreenProfile) -> Result<()> {
    let mut cmd = vec![CMD_SET_SCR_MODE];
    cmd.extend_from_slice(&screen_profile.encode());

    send_expect(f, &cmd, &[RSP_ACK]).await
}

async fn transmit_set_profile_name(f: &mut Serial, profile_name: String) -> Result<()> {
    let profile_name =
        ProfileName::from_str(&profile_name[..min(profile_name.len(), PROFILE_NAME_CHAR_LEN)]);

    let mut cmd = vec![CMD_SET_PROFILE_NAME];
    cmd.extend_from_slice(&profile_name.encode());

    send_expect(f, &cmd, &[RSP_ACK]).await
}

async fn transmit_set_system_stats(f: &mut Serial, system_stats: SystemStats) -> Result<()> {
    let mut cmd = vec![CMD_SET_SYSTEM_STATS];
    cmd.extend_from_slice(&system_stats.encode());

    send_expect(f, &cmd, &[RSP_ACK]).await
}

async fn transmit_identify_signal(f: &mut Serial) -> Result<()> {
    send_expect(f, &[CMD_IDENTIFY], &[RSP_ACK]).await
}

async fn transmit_update_signal(f: &mut Serial) -> Result<()> {
    send_expect(f, &[CMD_UPDATE], &[RSP_DISCONNECTED]).await
}

async fn transmit_disconnect_signal(f: &mut Serial) -> Result<()> {
    send_expect(f, &[CMD_DISCONNECT], &[RSP_DISCONNECTED]).await
}

pub fn serial_get_device(connected_uids: &HashSet<String>) -> Result<Serial> {
    let ports = serialport::available_ports().context("failed to scan serial ports")?;
    let ports: Vec<_> = ports
        .iter()
        .filter(|p| match &p.port_type {
            serialport::SerialPortType::UsbPort(p) => {
                p.vid == 0x1209
                    && (p.pid == 0xF209 || p.pid == 0xF20A || p.pid == 0xF20B || p.pid == 0xF20C)
                    && !connected_uids.contains(&p.serial_number.clone().unwrap_or("".into()))
            }
            _ => false,
        })
        .collect();

    log::debug!("serial: {:?} / {:?}", ports, connected_uids);

    if ports.len() == 0 {
        bail!("failed to find any jukebox serial ports");
    }

    let port = ports.get(0).unwrap();

    Ok(serialport::new(port.port_name.clone(), 115200)
        .timeout(Duration::from_millis(250))
        .open()
        .context("failed to open serial port")?)
}

pub async fn serial_loop(
    f: &mut Serial,
    sg_tx: UnboundedSender<SerialEvent>,
    sr_tx: UnboundedSender<SerialEvent>,
    device_uid: String,
    mut s_cmd_rx: UnboundedReceiver<SerialCommand>,
    system_stats: Arc<Mutex<SystemStats>>,
) -> Result<()> {
    let mut tick = Instant::now().checked_add(Duration::from_secs(1)).unwrap();

    'forv: loop {
        let keys = transmit_get_input_keys(f).await?;
        sr_tx
            .send(SerialEvent::GetInputKeys {
                device_uid: device_uid.clone(),
                keys: keys.clone(),
            })
            .context("failed to send input info to action thread")?;
        sg_tx
            .send(SerialEvent::GetInputKeys {
                device_uid: device_uid.clone(),
                keys: keys.clone(),
            })
            .context("failed to send input info to gui thread")?;

        // TODO: only send system stats when device has a screen
        if Instant::now() >= tick {
            tick = Instant::now().checked_add(Duration::from_secs(1)).unwrap();
            let stats = {
                let locked = system_stats.lock().await;
                locked.clone()
            };
            transmit_set_system_stats(f, stats).await?;
        }

        while let Ok(cmd) = s_cmd_rx.try_recv() {
            match cmd {
                SerialCommand::Identify => {
                    transmit_identify_signal(f).await?;
                }
                SerialCommand::SetKeyboardInput(slot, keyboard_event) => {
                    transmit_set_keyboard_input(f, slot, keyboard_event).await?;
                }
                SerialCommand::SetMouseInput(slot, mouse_event) => {
                    transmit_set_mouse_input(f, slot, mouse_event).await?;
                }
                SerialCommand::SetRgbMode(rgb_profile) => {
                    // TODO: only send when device has rgb
                    transmit_set_rgb_mode(f, rgb_profile).await?;
                }
                SerialCommand::SetScrIcon(slot, icon_data) => {
                    // TODO: only send when device has a screen
                    transmit_set_scr_icon(f, slot, icon_data).await?;
                }
                SerialCommand::SetScrMode(screen_profile) => {
                    // TODO: only send when device has a screen
                    transmit_set_screen_mode(f, screen_profile).await?;
                }
                SerialCommand::SetProfileName(profile_name) => {
                    // TODO: only send when device has a screen
                    transmit_set_profile_name(f, profile_name).await?;
                }
                SerialCommand::Update => {
                    transmit_update_signal(f).await?;
                    sr_tx
                        .send(SerialEvent::Disconnected {
                            device_uid: device_uid.clone(),
                        })
                        .context("failed to send disconnect (for update) info to react")?;
                    sg_tx
                        .send(SerialEvent::Disconnected {
                            device_uid: device_uid.clone(),
                        })
                        .context("failed to send disconnect (for update) info to gui")?;
                    break 'forv; // The device has disconnected, we should too.
                }
                SerialCommand::Disconnect => {
                    transmit_disconnect_signal(f).await?;
                    sr_tx
                        .send(SerialEvent::Disconnected {
                            device_uid: device_uid.clone(),
                        })
                        .context("failed to send disconnect info to react")?;
                    sg_tx
                        .send(SerialEvent::Disconnected {
                            device_uid: device_uid.clone(),
                        })
                        .context("failed to send disconnect info to gui")?;
                    break 'forv; // The device has disconnected, we should too.
                }
            }
        }
    }

    log::info!("exiting device thread {}", device_uid);

    Ok(())
}

pub async fn serial_task(
    brkr: Arc<AtomicBool>,
    scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
    sg_tx: UnboundedSender<SerialEvent>,
    sr_tx: UnboundedSender<SerialEvent>,
    system_stats: Arc<Mutex<SystemStats>>,
) -> Result<()> {
    log::debug!("starting serial thread...");

    let connected_uids = Arc::new(Mutex::new(HashSet::new()));

    while !brkr.load(std::sync::atomic::Ordering::Relaxed) {
        let mut f = {
            let uids = connected_uids.lock().await.clone();
            let r = spawn_blocking(move || serial_get_device(&uids))
                .await
                .unwrap();
            match r {
                Err(_e) => {
                    // log::debug!("get_serial_device() failure: {:#}", _e);
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                Ok(f) => f,
            }
        };

        // Greet and link up
        let device_info = greet_host(&mut f).await?;
        let device_uid = device_info.device_uid.clone();
        log::info!("device connected: {:?}", device_info);
        // TODO: check that firmware version is ok
        let sg_tx = sg_tx.clone();
        let sr_tx = sr_tx.clone();

        let (s_cmd_tx, s_cmd_rx) = unbounded_channel::<SerialCommand>();

        scmd_txs.lock().await.insert(device_uid.clone(), s_cmd_tx);
        connected_uids.lock().await.insert(device_uid.clone());

        let connected_uids = connected_uids.clone();
        let scmd_txs = scmd_txs.clone();
        let system_stats = system_stats.clone();

        tokio::spawn(async move {
            let _ = sg_tx
                .send(SerialEvent::Connected {
                    device_info: device_info.clone(),
                })
                .context("failed to send device info to gui");
            let _ = sr_tx
                .send(SerialEvent::Connected {
                    device_info: device_info.clone(),
                })
                .context("failed to send device info to react");

            match serial_loop(
                &mut f,
                sg_tx.clone(),
                sr_tx.clone(),
                device_uid.clone(),
                s_cmd_rx,
                system_stats,
            )
            .await
            {
                Err(e) => {
                    log::warn!("Serial device {} error: {:#}", device_uid, e);
                    if let Err(e) = sg_tx.send(SerialEvent::LostConnection {
                        device_uid: device_uid.clone(),
                    }) {
                        log::warn!(
                            "failed to send lost connection for {} to gui ({})",
                            device_uid,
                            e
                        );
                    }
                    if let Err(e) = sr_tx.send(SerialEvent::LostConnection {
                        device_uid: device_uid.clone(),
                    }) {
                        log::warn!(
                            "failed to send lost connection for {} to react ({})",
                            device_uid,
                            e
                        );
                    }
                }
                _ => log::info!(
                    "Serial device {} successfully disconnected. Looping...",
                    device_uid
                ),
            };

            connected_uids.lock().await.remove(&device_uid);
            scmd_txs.lock().await.remove(&device_uid);
        });
    }

    Ok(())
}
