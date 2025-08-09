//! Serial Comms
//!
//! The protocol processor for JukeBox.

use defmt::*;

use embassy_futures::yield_now;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, pipe::Pipe};
use embassy_time::{Duration, Instant};
use jukebox_util::{
    peripheral::{IDENT_KEY_INPUT, IDENT_KNOB_INPUT, IDENT_PEDAL_INPUT, IDENT_UNKNOWN_INPUT},
    protocol::{
        Command, MAX_PACKET_SIZE, RSP_FULL_ACK, RSP_FULL_DISCONNECTED, RSP_FULL_UNKNOWN,
        RSP_LINK_DELIMITER, RSP_LINK_HEADER, decode_packet_size,
    },
};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

use crate::{identify::start_identify, uid::get_uid, util::bootsel};

type InternalBuf = ConstGenericRingBuffer<u8, 4096>;

static USB_TO_SERIAL: Pipe<ThreadModeRawMutex, 2048> = Pipe::new();
static SERIAL_TO_USB: Pipe<ThreadModeRawMutex, 512> = Pipe::new();

const KEEPALIVE_TIME: Duration = Duration::from_secs(1);

struct SerialMod {
    buf: InternalBuf,
    connected: bool,
    keep_alive_end: Instant,
}
impl SerialMod {
    fn new() -> Self {
        Self {
            buf: ConstGenericRingBuffer::new(),
            connected: false,
            keep_alive_end: unwrap!(Instant::now().checked_add(KEEPALIVE_TIME)),
        }
    }

    fn check_pipe(&mut self) {
        let mut read_buf = [0u8; 128];
        while !USB_TO_SERIAL.is_empty() {
            match USB_TO_SERIAL.try_read(&mut read_buf) {
                Ok(n) => {
                    for b in &read_buf[..n] {
                        self.buf.enqueue(*b);
                    }
                }
                Err(_) => (),
            }
        }
    }

    async fn check_cmd(&mut self) -> Option<(Command, [u8; MAX_PACKET_SIZE])> {
        self.check_pipe();

        if self.buf.len() < 3 {
            return None;
        }

        let w1 = *unwrap!(self.buf.get(0));
        let w2 = *unwrap!(self.buf.get(1));
        let w3 = *unwrap!(self.buf.get(2));

        match decode_packet_size(w1, w2, w3) {
            Ok(size) => {
                if self.buf.len() >= size + 3 {
                    // we have all the data necessary to decode the packet
                    // dequeue the packet size, we've already used it
                    self.buf.dequeue();
                    self.buf.dequeue();
                    self.buf.dequeue();

                    let cmd = self.buf.dequeue().unwrap().into();

                    let mut data = [0u8; MAX_PACKET_SIZE];
                    for i in 0..size - 1 {
                        data[i] = self.buf.dequeue().unwrap();
                    }

                    Some((cmd, data))
                } else {
                    // we're still waiting on some data, so we exit early
                    None
                }
            }
            Err(()) => {
                error!("failed to decode packet size: {} {} {}", w1, w2, w3);
                self.buf.clear();
                None
            }
        }
    }

    async fn reset_peripherals(&mut self) {
        // TODO
    }

    async fn start_update(&mut self) -> bool {
        info!("Command Update");
        SERIAL_TO_USB.write_all(RSP_FULL_DISCONNECTED).await;
        bootsel();

        true
    }

    async fn task(&mut self) -> ! {
        loop {
            if self.connected && self.keep_alive_end <= Instant::now() {
                warn!("Keepalive triggered, disconnecting.");
                self.connected = false;
                self.buf.clear();
                self.reset_peripherals().await;
            }

            let (cmd, data) = match self.check_cmd().await {
                Some(cmd_data) => cmd_data,
                None => {
                    yield_now().await;
                    continue;
                }
            };

            let unknown = async || {
                error!("unknown command: {}", data);
                SERIAL_TO_USB.write_all(RSP_FULL_UNKNOWN).await;
                false
            };
            let keep_connection_alive = match self.connected {
                false => match cmd {
                    Command::Update => self.start_update().await,
                    Command::Greeting => {
                        let device_type = if cfg!(feature = "keypad") {
                            IDENT_KEY_INPUT
                        } else if cfg!(feature = "knobpad") {
                            IDENT_KNOB_INPUT
                        } else if cfg!(feature = "pedalpad") {
                            IDENT_PEDAL_INPUT
                        } else {
                            IDENT_UNKNOWN_INPUT
                        };

                        SERIAL_TO_USB
                            .write_all(&[
                                b'0',
                                b'1',
                                b'B',
                                RSP_LINK_HEADER,
                                RSP_LINK_DELIMITER,
                                device_type,
                                RSP_LINK_DELIMITER,
                            ])
                            .await;
                        SERIAL_TO_USB
                            .write_all(env!("CARGO_PKG_VERSION").as_bytes())
                            .await;
                        SERIAL_TO_USB.write_all(&[RSP_LINK_DELIMITER]).await;
                        SERIAL_TO_USB.write_all(get_uid().as_bytes()).await;
                        SERIAL_TO_USB.write_all(&[RSP_LINK_DELIMITER]).await;

                        self.connected = true;

                        true
                    }
                    _ => unknown().await,
                },
                true => match cmd {
                    Command::GetInputKeys => {
                        // // copy peripherals and inputs out
                        // let inputs = PERIPHERAL_INPUTS.with_lock(|i| *i);
                        // // write all the inputs out
                        // match inputs {
                        //     JBInputs::KeyPad(i) => {
                        //         Self::send(serial, b"004");
                        //         Self::send(serial, &[RSP_INPUT_HEADER]);
                        //         Self::send(serial, &i.encode());
                        //     }
                        //     JBInputs::KnobPad(i) => {
                        //         Self::send(serial, b"003");
                        //         Self::send(serial, &[RSP_INPUT_HEADER]);
                        //         Self::send(serial, &i.encode());
                        //     }
                        //     JBInputs::PedalPad(i) => {
                        //         Self::send(serial, b"003");
                        //         Self::send(serial, &[RSP_INPUT_HEADER]);
                        //         Self::send(serial, &i.encode());
                        //     }
                        // };
                        defmt::todo!();
                        true
                    }

                    Command::SetInputEvent => {
                        // let slot = data[0];
                        // let new_input = InputEvent::decode(&data[1..7 + 1]);
                        // INPUT_EVENTS.with_mut_lock(|e| e[slot as usize] = new_input);
                        // Self::send(serial, RSP_FULL_ACK);
                        defmt::todo!();
                        true
                    }
                    Command::SetRgbMode => {
                        // let rgb = RgbProfile::decode(&data);
                        // RGB_CONTROLS.with_mut_lock(|c| *c = (true, rgb));
                        // Self::send(serial, RSP_FULL_ACK);
                        defmt::todo!();
                        true
                    }
                    Command::SetScrIcon => {
                        // let slot = data[0];
                        // let new_icon = &data[1..32 * 32 * 2 + 1];
                        // ICONS.with_mut_lock(|icons| {
                        //     let scr_icon = &mut icons[slot as usize];
                        //     let mut i = 0;
                        //     while i < 32 * 32 {
                        //         let (r, g, b) = split_to_rgb565(
                        //             ((new_icon[i * 2 + 1] as u16) << 8) | (new_icon[i * 2] as u16),
                        //         );
                        //         let c = Bgr565::new(b, g, r);
                        //         scr_icon.1[i] = c;
                        //         i += 1;
                        //     }
                        //     scr_icon.0 = true;
                        // });
                        // Self::send(serial, RSP_FULL_ACK);
                        defmt::todo!();
                        true
                    }
                    Command::SetScrMode => {
                        // let profile = ScreenProfile::decode(&data);
                        // SCREEN_CONTROLS.with_mut_lock(|p| *p = (true, profile));
                        // Self::send(serial, RSP_FULL_ACK);
                        defmt::todo!();
                        true
                    }
                    Command::SetSystemStats => {
                        // let stats = SystemStats::decode(&data);
                        // SCREEN_SYSTEM_STATS.with_mut_lock(|s| *s = (true, stats));
                        // Self::send(serial, RSP_FULL_ACK);
                        defmt::todo!();
                        true
                    }
                    Command::SetProfileName => {
                        // let new_profile_name: ProfileName = SmallStr::decode(&data);
                        // PROFILE_NAME.with_mut_lock(|p| *p = (true, new_profile_name));
                        // Self::send(serial, RSP_FULL_ACK);
                        defmt::todo!();
                        true
                    }
                    Command::Identify => {
                        start_identify().await;
                        SERIAL_TO_USB.write_all(RSP_FULL_ACK).await;
                        true
                    }
                    Command::Update => self.start_update().await,
                    Command::Disconnect => {
                        info!("Serial Disconnected");
                        SERIAL_TO_USB.write_all(RSP_FULL_DISCONNECTED).await;
                        self.connected = false;
                        self.reset_peripherals().await;
                        true
                    }
                    _ => unknown().await,
                },
            };

            if keep_connection_alive {
                self.keep_alive_end = unwrap!(Instant::now().checked_add(KEEPALIVE_TIME));
            }

            yield_now().await;
        }
    }
}

#[embassy_executor::task]
pub async fn serial_task() -> ! {
    SerialMod::new().task().await;
}
