//! Serial processing module

#[allow(unused_imports)]
use defmt::*;

use embedded_hal::timer::{Cancel as _, CountDown as _};
use itertools::Itertools;
use jukebox_util::{
    color::RgbProfile,
    peripheral::{
        Connection, IDENT_KEY_INPUT, IDENT_KNOB_INPUT, IDENT_PEDAL_INPUT, IDENT_UNKNOWN_INPUT,
    },
    protocol::{
        Command, CMD_END, RSP_ACK, RSP_DISCONNECTED, RSP_END, RSP_INPUT_HEADER, RSP_LINK_DELIMITER,
        RSP_LINK_HEADER, RSP_RGB_HEADER, RSP_UNKNOWN,
    },
};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use rp2040_hal::{fugit::ExtU32, timer::CountDown, usb::UsbBus};
use usbd_serial::SerialPort;

use crate::{
    peripheral::{inputs_default, inputs_write_report},
    IdentifyTrigger, PeripheralInputs, RgbControls, UpdateTrigger,
};

const BUFFER_SIZE: usize = 2048;

const KEEPALIVE: u32 = 250;

pub struct SerialMod {
    buffer: ConstGenericRingBuffer<u8, BUFFER_SIZE>,
    state: Connection,
    keepalive_timer: CountDown,
}

impl SerialMod {
    pub fn new(mut timer: CountDown) -> Self {
        timer.start(KEEPALIVE.millis());

        SerialMod {
            buffer: ConstGenericRingBuffer::new(),
            state: Connection::NotConnected(true),
            keepalive_timer: timer,
        }
    }

    fn check_cmd(&mut self) -> Option<usize> {
        // we measure out a command token by looking for the end-of-command string: "\r\n"
        // if one is not found, we do not have a valid command ready to be read
        // TODO: better match the characters in CMD_END
        for ((_, r), (i, n)) in self.buffer.iter().enumerate().tuple_windows() {
            if *r == CMD_END[0] && *n == CMD_END[1] {
                return Some(i + 1);
            }
        }

        None
    }

    fn send(serial: &mut SerialPort<UsbBus>, rsp: &[u8]) {
        // TODO: its possible for write to drop some characters, if we're not careful.
        // we should probably handle that before we take on larger communications.
        while let Err(_) = serial.write(rsp) {
            let _ = serial.flush();
            cortex_m::asm::nop();
        }
    }

    fn send_full_response(serial: &mut SerialPort<UsbBus>, rsp: &[u8]) {
        Self::send(serial, rsp);
        Self::send_end_response(serial);
    }

    fn send_end_response(serial: &mut SerialPort<UsbBus>) {
        Self::send(serial, RSP_END);
    }

    fn decode_cmd(&mut self, size: usize) -> (Command, [u8; 32]) {
        let mut data = [0u8; 32];

        // if size > 4 {
        //     return Command::Unknown;
        // }

        let w1 = self.buffer.get(0).unwrap_or(&b'\0');
        let w2 = self.buffer.get(size - 2).unwrap_or(&b'\0');
        let w3 = self.buffer.get(size - 1).unwrap_or(&b'\0');

        // debug!("cmd: {} {} {} (size:{})", w1, w2, w3, size);

        let cmd = Command::decode(*w1);

        if cmd != Command::Unknown && !(*w2 == CMD_END[0] && *w3 == CMD_END[1]) {
            for _ in 0..size {
                self.buffer.dequeue();
            }

            return (Command::Unknown, data);
        }

        self.buffer.dequeue();
        for i in 0..size - 3 {
            data[i] = self.buffer.dequeue().unwrap();
        }
        self.buffer.dequeue();
        self.buffer.dequeue();

        (cmd, data)
    }

    #[allow(dead_code)]
    pub fn get_connection_status(&self) -> Connection {
        self.state.clone()
    }

    fn start_update(&mut self, serial: &mut SerialPort<UsbBus>, update_trigger: &UpdateTrigger) {
        info!("Command Update");
        Self::send_full_response(serial, &[RSP_DISCONNECTED]);
        self.state = Connection::NotConnected(true);
        update_trigger.with_mut_lock(|u| *u = true);
    }

    fn start_identify(
        &mut self,
        serial: &mut SerialPort<UsbBus>,
        identify_trigger: &IdentifyTrigger,
    ) {
        info!("Command Identify");
        Self::send_full_response(serial, &[RSP_ACK]);
        identify_trigger.with_mut_lock(|u| *u = true);
    }

    pub fn update(
        &mut self,
        serial: &mut SerialPort<UsbBus>,
        firmware_version: &str,
        device_uid: &str,
        peripheral_inputs: &PeripheralInputs,
        update_trigger: &UpdateTrigger,
        identify_trigger: &IdentifyTrigger,
        rgb_controls: &RgbControls,
    ) {
        if self.state == Connection::Connected && self.keepalive_timer.wait().is_ok() {
            warn!("Keepalive triggered, disconnecting.");
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
        let size = self.check_cmd();
        if size.is_none() {
            return;
        }
        let (decode, data) = self.decode_cmd(size.unwrap());
        debug!("cmd:{} data:{}", decode as u8, data);

        // process command
        let mut unknown = || {
            Self::send_full_response(serial, &[RSP_UNKNOWN]);
            false
        };
        let valid = match self.state {
            Connection::NotConnected(_) => match decode {
                Command::Update => {
                    self.start_update(serial, update_trigger);
                    true
                }
                Command::Greeting => {
                    Self::send(serial, &[RSP_LINK_HEADER, RSP_LINK_DELIMITER]);
                    let dtype = if cfg!(feature = "keypad") {
                        IDENT_KEY_INPUT
                    } else if cfg!(feature = "knobpad") {
                        IDENT_KNOB_INPUT
                    } else if cfg!(feature = "pedalpad") {
                        IDENT_PEDAL_INPUT
                    } else {
                        IDENT_UNKNOWN_INPUT
                    };
                    Self::send(serial, &[dtype]);
                    Self::send(serial, &[RSP_LINK_DELIMITER]);
                    Self::send(serial, firmware_version.as_bytes());
                    Self::send(serial, &[RSP_LINK_DELIMITER]);
                    Self::send(serial, device_uid.as_bytes());
                    Self::send(serial, &[RSP_LINK_DELIMITER]);
                    Self::send_end_response(serial);

                    self.state = Connection::Connected;
                    info!("Serial Connected");
                    true
                }
                _ => unknown(),
            },
            Connection::Connected => match decode {
                Command::GetInputKeys => {
                    // copy peripherals and inputs out
                    let inputs = {
                        let mut inputs = inputs_default(); // JBInputs::default();
                        peripheral_inputs.with_lock(|i| {
                            inputs = *i;
                        });
                        inputs
                    };

                    // write all the inputs out
                    Self::send(serial, &[RSP_INPUT_HEADER]);
                    inputs_write_report(inputs, serial);
                    Self::send_end_response(serial);

                    true
                }
                Command::GetRGB => {
                    let rgb = {
                        let mut rgb = RgbProfile::Off;
                        rgb_controls.with_lock(|c| {
                            rgb = c.1.clone();
                        });
                        rgb
                    };

                    Self::send(serial, &[RSP_RGB_HEADER]);
                    let _ = serial.write(&rgb.encode());
                    Self::send_end_response(serial);

                    true
                }
                Command::SetRGB => {
                    let rgb = RgbProfile::decode(data);
                    rgb_controls.with_mut_lock(|c| {
                        c.0 = true;
                        c.1 = rgb;
                    });

                    Self::send(serial, &[RSP_ACK]);
                    Self::send_end_response(serial);

                    true
                }
                Command::GetScr => {
                    // TODO:
                    unknown()
                }
                Command::SetScr => {
                    // TODO:
                    unknown()
                }
                Command::Identify => {
                    // TODO: flash led for identifying
                    self.start_identify(serial, identify_trigger);
                    true
                }
                Command::Update => {
                    self.start_update(serial, update_trigger);
                    true
                }
                Command::Disconnect => {
                    Self::send_full_response(serial, &[RSP_DISCONNECTED]);
                    self.state = Connection::NotConnected(true);
                    info!("Serial Disconnected");
                    true
                }
                Command::NegativeAck => {
                    // we sent something in error, better bail
                    self.state = Connection::NotConnected(false);
                    info!("Serial NegativeAck'd");
                    false
                }
                _ => unknown(),
            },
        };

        if valid {
            // info!("restarting keepalive");
            let _ = self.keepalive_timer.cancel();
            self.keepalive_timer.start(KEEPALIVE.millis()); // restart keepalive timer with valid command
        }
    }
}
