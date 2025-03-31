//! Serial processing module

#[allow(unused_imports)]
use defmt::*;

use embedded_hal::timer::{Cancel as _, CountDown as _};
use jukebox_util::{
    color::RgbProfile,
    input::{KeyboardEvent, MouseEvent},
    peripheral::{
        Connection, JBInputs, IDENT_KEY_INPUT, IDENT_KNOB_INPUT, IDENT_PEDAL_INPUT,
        IDENT_UNKNOWN_INPUT,
    },
    protocol::{
        decode_packet_size, Command, MAX_PACKET_SIZE, RSP_ACK, RSP_DISCONNECTED, RSP_INPUT_HEADER,
        RSP_LINK_DELIMITER, RSP_LINK_HEADER, RSP_UNKNOWN,
    },
};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use rp2040_hal::{fugit::ExtU32, timer::CountDown, usb::UsbBus};
use usbd_serial::SerialPort;

use crate::{
    modules::rgb::DEFAULT_RGB,
    reset_icons,
    util::{
        inputs_default, reset_peripherals, CONNECTION_STATUS, ICONS, IDENTIFY_TRIGGER,
        KEYBOARD_EVENTS, MOUSE_EVENTS, PERIPHERAL_INPUTS, RGB_CONTROLS, UPDATE_TRIGGER,
    },
};

const BUFFER_SIZE: usize = 4096;

const KEEPALIVE: u32 = 500;

pub struct SerialMod {
    buffer: ConstGenericRingBuffer<u8, BUFFER_SIZE>,
    state: Connection,
    keepalive_timer: CountDown,
}

impl SerialMod {
    pub fn new(mut keepalive_timer: CountDown) -> Self {
        keepalive_timer.start(KEEPALIVE.millis());

        SerialMod {
            buffer: ConstGenericRingBuffer::new(),
            state: Connection::NotConnected(true),
            keepalive_timer,
        }
    }

    fn send(serial: &mut SerialPort<UsbBus>, rsp: &[u8]) {
        // TODO: its possible for write to drop some characters, if we're not careful.
        // we should probably handle that before we take on larger communications.
        while let Err(_) = serial.write(rsp) {
            let _ = serial.flush();
            cortex_m::asm::nop();
        }
    }

    fn check_cmd(&mut self) -> Option<(Command, [u8; MAX_PACKET_SIZE])> {
        if self.buffer.len() >= 3 {
            let w1 = *self.buffer.get(0).unwrap();
            let w2 = *self.buffer.get(1).unwrap();
            let w3 = *self.buffer.get(2).unwrap();

            let size = decode_packet_size(w1, w2, w3);

            if self.buffer.len() >= size + 3 {
                // dequeue packet size
                self.buffer.dequeue();
                self.buffer.dequeue();
                self.buffer.dequeue();

                // dequeue command type
                let c = self.buffer.dequeue().unwrap();
                let cmd = Command::decode(c);

                let mut data = [0u8; MAX_PACKET_SIZE];
                for i in 0..size - 1 {
                    data[i] = self.buffer.dequeue().unwrap();
                }

                Some((cmd, data))
            } else {
                None
            }
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn get_connection_status(&self) -> Connection {
        self.state.clone()
    }

    fn start_update(&mut self, serial: &mut SerialPort<UsbBus>) {
        info!("Command Update");
        Self::send(serial, &[b'0', b'0', b'1', RSP_DISCONNECTED]);
        CONNECTION_STATUS.with_mut_lock(|c| *c = Connection::NotConnected(true));
        self.state = Connection::NotConnected(true);
        UPDATE_TRIGGER.with_mut_lock(|u| *u = true);
    }

    fn start_identify(&mut self, serial: &mut SerialPort<UsbBus>) {
        info!("Command Identify");
        Self::send(serial, &[b'0', b'0', b'1', RSP_ACK]);
        IDENTIFY_TRIGGER.with_mut_lock(|u| *u = true);
    }

    pub fn update(
        &mut self,
        serial: &mut SerialPort<UsbBus>,
        firmware_version: &str,
        device_uid: &str,
    ) {
        if self.state == Connection::Connected && self.keepalive_timer.wait().is_ok() {
            warn!("Keepalive triggered, disconnecting.");
            CONNECTION_STATUS.with_mut_lock(|c| *c = Connection::NotConnected(false));
            RGB_CONTROLS.with_mut_lock(|c| {
                c.0 = true;
                c.1 = DEFAULT_RGB;
            });
            reset_icons();
            self.state = Connection::NotConnected(false);
        }

        let mut buf = [0u8; 128];
        match serial.read(&mut buf) {
            Err(_) => {}
            Ok(s) => {
                // copy read data to internal buffer
                for b in 0..s {
                    self.buffer.push(buf[b]);
                }
            }
        }

        // load and decode command if available
        let (decode, data) = if let Some(d) = self.check_cmd() {
            d
        } else {
            return;
        };

        // process command
        let mut unknown = || {
            Self::send(serial, &[b'0', b'0', b'1', RSP_UNKNOWN]);
            false
        };
        let valid = match self.state {
            Connection::NotConnected(_) => match decode {
                Command::Update => {
                    self.start_update(serial);
                    true
                }
                Command::Greeting => {
                    let dtype = if cfg!(feature = "keypad") {
                        IDENT_KEY_INPUT
                    } else if cfg!(feature = "knobpad") {
                        IDENT_KNOB_INPUT
                    } else if cfg!(feature = "pedalpad") {
                        IDENT_PEDAL_INPUT
                    } else {
                        IDENT_UNKNOWN_INPUT
                    };
                    Self::send(
                        serial,
                        &[
                            b'0',
                            b'1',
                            b'B',
                            RSP_LINK_HEADER,
                            RSP_LINK_DELIMITER,
                            dtype,
                            RSP_LINK_DELIMITER,
                        ],
                    );
                    Self::send(serial, firmware_version.as_bytes());
                    Self::send(serial, &[RSP_LINK_DELIMITER]);
                    Self::send(serial, device_uid.as_bytes());
                    Self::send(serial, &[RSP_LINK_DELIMITER]);

                    self.state = Connection::Connected;
                    CONNECTION_STATUS.with_mut_lock(|c| *c = Connection::Connected);
                    info!("Serial Connected");
                    true
                }
                _ => unknown(),
            },
            Connection::Connected => match decode {
                Command::GetInputKeys => {
                    // copy peripherals and inputs out
                    let inputs = {
                        let mut inputs = inputs_default();
                        PERIPHERAL_INPUTS.with_lock(|i| {
                            inputs = *i;
                        });
                        inputs
                    };

                    // write all the inputs out
                    let _ = match inputs {
                        JBInputs::KeyPad(i) => {
                            Self::send(serial, b"004");
                            Self::send(serial, &[RSP_INPUT_HEADER]);
                            Self::send(serial, &i.encode());
                        }
                        JBInputs::KnobPad(i) => {
                            Self::send(serial, b"003");
                            Self::send(serial, &[RSP_INPUT_HEADER]);
                            Self::send(serial, &i.encode());
                        }
                        JBInputs::PedalPad(i) => {
                            Self::send(serial, b"003");
                            Self::send(serial, &[RSP_INPUT_HEADER]);
                            Self::send(serial, &i.encode());
                        }
                    };

                    true
                }
                Command::SetKeyboardInput => {
                    let slot = data[0];
                    let new_input = &data[1..6 + 1];
                    let new_input = KeyboardEvent::decode(new_input);

                    KEYBOARD_EVENTS.with_mut_lock(|e| {
                        e[slot as usize] = new_input;
                    });

                    Self::send(serial, b"001");
                    Self::send(serial, &[RSP_ACK]);

                    true
                }
                Command::SetMouseInput => {
                    let slot = data[0];
                    let new_input = &data[1..5 + 1];
                    let new_input = MouseEvent::decode(new_input);

                    MOUSE_EVENTS.with_mut_lock(|e| {
                        e[slot as usize] = new_input;
                    });

                    Self::send(serial, b"001");
                    Self::send(serial, &[RSP_ACK]);

                    true
                }
                Command::SetGamepadInput => {
                    // TODO:
                    unknown()
                }
                Command::SetRgbMode => {
                    let rgb = RgbProfile::decode(&data);
                    RGB_CONTROLS.with_mut_lock(|c| {
                        c.0 = true;
                        c.1 = rgb;
                    });

                    Self::send(serial, b"001");
                    Self::send(serial, &[RSP_ACK]);

                    true
                }
                Command::SetScrMode => {
                    // TODO:
                    unknown()
                }
                Command::SetScrIcon => {
                    let slot = data[0];
                    let new_icon = &data[1..32 * 32 * 2 + 1];

                    ICONS.with_mut_lock(|icons| {
                        let scr_icon = &mut icons[slot as usize];
                        let mut i = 0;
                        while i < 32 * 32 {
                            scr_icon.1[i] =
                                ((new_icon[i * 2 + 1] as u16) << 8) | (new_icon[i * 2] as u16);
                            i += 1;
                        }
                        scr_icon.0 = true;
                    });

                    Self::send(serial, b"001");
                    Self::send(serial, &[RSP_ACK]);

                    true
                }
                Command::Identify => {
                    // TODO: flash led for identifying
                    self.start_identify(serial);
                    true
                }
                Command::Update => {
                    self.start_update(serial);
                    true
                }
                Command::Disconnect => {
                    Self::send(serial, &[b'0', b'0', b'1', RSP_DISCONNECTED]);
                    self.state = Connection::NotConnected(true);
                    reset_peripherals(true);

                    info!("Serial Disconnected");
                    true
                }
                Command::NegativeAck => {
                    // we sent something in error, better bail
                    self.state = Connection::NotConnected(false);
                    reset_peripherals(false);

                    info!("Serial NegativeAck'd");
                    false
                }
                _ => unknown(),
            },
        };

        if valid {
            // info!("restarting keepalive");
            let _ = self.keepalive_timer.cancel();
            self.keepalive_timer.start(KEEPALIVE.millis());
            // restart keepalive timer with valid command
        }
    }
}
