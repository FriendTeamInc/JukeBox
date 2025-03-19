// Serial communication

use crate::input::InputKey;

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use jukebox_util::color::RgbProfile;
use jukebox_util::peripheral::{
    KeyInputs, KnobInputs, PedalInputs, IDENT_KEY_INPUT, IDENT_KNOB_INPUT, IDENT_PEDAL_INPUT,
    IDENT_UNKNOWN_INPUT,
};
use jukebox_util::protocol::{
    CMD_DISCONNECT, CMD_END, CMD_GET_INPUT_KEYS, CMD_GET_RGB, CMD_GREET, CMD_IDENTIFY,
    CMD_NEGATIVE_ACK, CMD_SET_RGB, CMD_UPDATE, RSP_ACK, RSP_DISCONNECTED, RSP_END,
    RSP_INPUT_HEADER, RSP_LINK_DELIMITER, RSP_LINK_HEADER, RSP_RGB_HEADER, RSP_UNKNOWN,
};
use serialport::SerialPort;
use tokio::task::block_in_place;
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    task::spawn_blocking,
    time::sleep,
};

#[derive(PartialEq, Clone)]
pub struct SerialConnectionDetails {
    pub input_identifier: u8,
    pub firmware_version: String,
    pub device_uid: String,
}

#[allow(unused)]
pub enum SerialCommand {
    Identify,
    GetRGB,
    SetRGB(RgbProfile),
    GetScr,
    SetScr,
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
    GetRGB {
        device_uid: String,
        rgb_control: RgbProfile,
    },
    LostConnection {
        device_uid: String,
    },
    Disconnected {
        device_uid: String,
    },
}

async fn get_serial_string(f: &mut Box<dyn SerialPort>) -> Result<Vec<u8>> {
    block_in_place(|| {
        let timeout = Instant::now() + Duration::from_secs(3);
        let mut buf = Vec::new();

        loop {
            if Instant::now() >= timeout {
                bail!("serial read timed out");
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

            if buf.len() > RSP_END.len() {
                let s = &buf[(buf.len() - RSP_END.len())..buf.len()];
                let c = s.iter().zip(RSP_END).all(|(a, b)| a == b);
                if c {
                    return Ok(buf);
                }
            }
        }
    })
}

async fn send_cmd(f: &mut Box<dyn SerialPort>, c: u8) -> Result<()> {
    let mut cmd = vec![c];
    cmd.extend_from_slice(CMD_END);
    send_bytes(f, cmd.as_slice())
        .await
        .with_context(|| format!("failed to send cmd {}", c))
}

async fn send_bytes(f: &mut Box<dyn SerialPort>, bytes: &[u8]) -> Result<()> {
    block_in_place(|| {
        f.write_all(bytes)
            .with_context(|| format!("failed to write message {:?}", bytes))?;
        f.flush().context("failed to flush message")?;

        Ok(())
    })
}

async fn expect_string(f: &mut Box<dyn SerialPort>, expect: &[u8]) -> Result<()> {
    let s = get_serial_string(f).await?;

    let matching = s.iter().zip(expect).filter(|&(a, b)| a == b).count() == s.len();

    if !matching {
        let matches_unknown = s
            .iter()
            .zip([RSP_UNKNOWN].iter().chain(RSP_END).collect::<Vec<_>>())
            .filter(|&(a, b)| a == b)
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

async fn send_expect(f: &mut Box<dyn SerialPort>, send: &[u8], expect: &[u8]) -> Result<()> {
    send_bytes(f, send)
        .await
        .with_context(|| format!("failed to send bytes {:?}", send))?;
    expect_string(f, expect)
        .await
        .with_context(|| format!("failed to get bytes {:?}", expect))?;
    Ok(())
}

// Tasks

async fn send_negative_ack(f: &mut Box<dyn SerialPort>) -> Result<()> {
    send_cmd(f, CMD_NEGATIVE_ACK)
        .await
        .context("failed to send nack")?;
    Ok(())
}

async fn greet_host(f: &mut Box<dyn SerialPort>) -> Result<SerialConnectionDetails> {
    // Host confirms protocol is good, recieves "link established" with some info about the device
    send_cmd(f, CMD_GREET)
        .await
        .context("failed to send greet")?;
    let resp = get_serial_string(f).await?;

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

async fn transmit_get_input_keys(f: &mut Box<dyn SerialPort>) -> Result<HashSet<InputKey>> {
    send_cmd(f, CMD_GET_INPUT_KEYS)
        .await
        .context("failed to send get input keys")?;
    let resp = get_serial_string(f).await?;

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

async fn transmit_get_rgb(f: &mut Box<dyn SerialPort>) -> Result<RgbProfile> {
    send_cmd(f, CMD_GET_RGB)
        .await
        .context("failed to set get rgb")?;
    let resp = get_serial_string(f).await?;

    if *resp.get(0).unwrap_or(&0) != RSP_RGB_HEADER {
        log::info!("rsp rgb: {:?}", resp);
        send_negative_ack(f).await?;
        bail!("failed to parse rgb (command character mismatch)");
    }

    if resp.len() != (32 + 1 + RSP_END.len()) {
        bail!("failed to parse rgb (wrong amount of data)");
    }

    let mut data = [0u8; 60];
    data.clone_from_slice(&resp[1..=60]);

    Ok(RgbProfile::decode(data))
}

async fn transmit_set_rgb(f: &mut Box<dyn SerialPort>, rgb_profile: RgbProfile) -> Result<()> {
    let mut cmd = vec![CMD_SET_RGB];
    cmd.extend_from_slice(&rgb_profile.encode());
    cmd.extend_from_slice(CMD_END);
    let mut rsp = vec![RSP_ACK];
    rsp.extend_from_slice(RSP_END);

    send_expect(f, &cmd, &rsp).await
}

async fn transmit_identify_signal(f: &mut Box<dyn SerialPort>) -> Result<()> {
    let mut cmd = vec![CMD_IDENTIFY];
    cmd.extend_from_slice(CMD_END);
    let mut rsp = vec![RSP_ACK];
    rsp.extend_from_slice(RSP_END);

    send_expect(f, &cmd, &rsp).await
}

async fn transmit_update_signal(f: &mut Box<dyn SerialPort>) -> Result<()> {
    let mut cmd = vec![CMD_UPDATE];
    cmd.extend_from_slice(CMD_END);
    let mut rsp = vec![RSP_DISCONNECTED];
    rsp.extend_from_slice(RSP_END);

    send_expect(f, &cmd, &rsp).await
}

async fn transmit_disconnect_signal(f: &mut Box<dyn SerialPort>) -> Result<()> {
    // tell the device to disconnect cleanly
    let mut cmd = vec![CMD_DISCONNECT];
    cmd.extend_from_slice(CMD_END);
    let mut rsp = vec![RSP_DISCONNECTED];
    rsp.extend_from_slice(RSP_END);

    send_expect(f, &cmd, &rsp).await
}

pub fn serial_get_device(connected_uids: &HashSet<String>) -> Result<Box<dyn SerialPort>> {
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

    log::debug!("serial ports found: {:?}", ports);
    log::debug!("serial ports connected: {:?}", connected_uids);

    if ports.len() == 0 {
        bail!("failed to find any jukebox serial ports");
    }

    let port = ports.get(0).unwrap();

    Ok(serialport::new(port.port_name.clone(), 115200)
        .timeout(std::time::Duration::from_millis(10))
        .open()
        .context("failed to open serial port")?)
}

pub async fn serial_loop(
    f: &mut Box<dyn SerialPort>,
    sg_tx: UnboundedSender<SerialEvent>,
    sr_tx: UnboundedSender<SerialEvent>,
    device_uid: String,
    mut s_cmd_rx: UnboundedReceiver<SerialCommand>,
) -> Result<()> {
    'forv: loop {
        let keys = transmit_get_input_keys(f).await?;
        sr_tx
            .send(SerialEvent::GetInputKeys {
                device_uid: device_uid.clone(),
                keys: keys.clone(),
            })
            .context("failed to send input info to react")?;
        sg_tx
            .send(SerialEvent::GetInputKeys {
                device_uid: device_uid.clone(),
                keys: keys.clone(),
            })
            .context("failed to send input info to gui")?;

        while let Ok(cmd) = s_cmd_rx.try_recv() {
            match cmd {
                SerialCommand::Identify => {
                    transmit_identify_signal(f).await?;
                }
                SerialCommand::GetRGB => {
                    let rgb_control = transmit_get_rgb(f).await?;
                    sg_tx
                        .send(SerialEvent::GetRGB {
                            device_uid: device_uid.clone(),
                            rgb_control: rgb_control,
                        })
                        .context("failed to send rgb info to gui")?;
                }
                SerialCommand::SetRGB(rgb_profile) => {
                    transmit_set_rgb(f, rgb_profile).await?;
                }
                SerialCommand::GetScr => {
                    todo!()
                }
                SerialCommand::SetScr => {
                    todo!()
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

        sleep(Duration::from_millis(25)).await;
    }

    log::info!("exiting device thread {}", device_uid);

    Ok(())
}

pub async fn serial_task(
    brkr: Arc<AtomicBool>,
    scmd_txs: Arc<Mutex<HashMap<String, UnboundedSender<SerialCommand>>>>,
    sg_tx: UnboundedSender<SerialEvent>,
    sr_tx: UnboundedSender<SerialEvent>,
) -> Result<()> {
    let connected_uids = Arc::new(Mutex::new(HashSet::new()));

    while !brkr.load(std::sync::atomic::Ordering::Relaxed) {
        let mut f = {
            let uids = connected_uids.lock().await.clone();
            let r = spawn_blocking(move || serial_get_device(&uids))
                .await
                .unwrap();
            match r {
                Err(e) => {
                    log::debug!("get_serial_device() failure: {:#}", e);
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                Ok(f) => f,
            }
        };

        // Greet and link up
        let device_info = greet_host(&mut f).await?;
        let device_uid = device_info.device_uid.clone();
        // TODO: check that firmware version is ok
        let sg_tx2 = sg_tx.clone();
        let sr_tx2 = sr_tx.clone();

        let (s_cmd_tx, s_cmd_rx) = unbounded_channel::<SerialCommand>();

        scmd_txs.lock().await.insert(device_uid.clone(), s_cmd_tx);
        connected_uids.lock().await.insert(device_uid.clone());

        let connected_uids2 = connected_uids.clone();
        let scmd_txs2 = scmd_txs.clone();

        tokio::spawn(async move {
            let connected_uids = connected_uids2;
            let scmd_txs = scmd_txs2;

            let _ = sg_tx2
                .clone()
                .send(SerialEvent::Connected {
                    device_info: device_info.clone(),
                })
                .context("failed to send device info to gui");
            let _ = sr_tx2
                .clone()
                .send(SerialEvent::Connected {
                    device_info: device_info.clone(),
                })
                .context("failed to send device info to react");

            match serial_loop(
                &mut f,
                sg_tx2.clone(),
                sr_tx2.clone(),
                device_uid.clone(),
                s_cmd_rx,
            )
            .await
            {
                Err(e) => {
                    log::warn!("Serial device {} error: {:#}", device_uid, e);
                    if let Err(e) = sg_tx2.send(SerialEvent::LostConnection {
                        device_uid: device_uid.clone(),
                    }) {
                        log::warn!(
                            "failed to send lost connection for {} to gui ({})",
                            device_uid,
                            e
                        );
                    }
                    if let Err(e) = sr_tx2.send(SerialEvent::LostConnection {
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
