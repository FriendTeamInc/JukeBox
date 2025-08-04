//! Serial processing module

#[allow(unused_imports)]
use defmt::*;

use embedded_graphics::pixelcolor::Bgr565;
use embedded_hal::timer::CountDown as _;
use jukebox_util::{
    color::split_to_rgb565,
    input::InputEvent,
    peripheral::{
        Connection, JBInputs, IDENT_KEY_INPUT, IDENT_KNOB_INPUT, IDENT_PEDAL_INPUT,
        IDENT_UNKNOWN_INPUT,
    },
    protocol::{
        decode_packet_size, Command, MAX_PACKET_SIZE, RSP_ACK, RSP_DISCONNECTED, RSP_FULL_ACK,
        RSP_FULL_DISCONNECTED, RSP_FULL_UNKNOWN, RSP_INPUT_HEADER, RSP_LINK_DELIMITER,
        RSP_LINK_HEADER,
    },
    rgb::RgbProfile,
    screen::{ProfileName, ScreenProfile},
    smallstr::SmallStr,
    stats::SystemStats,
};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use rp2040_hal::{fugit::ExtU32, timer::CountDown, usb::UsbBus};
use usbd_serial::SerialPort as FullSerialPort;

use crate::util::{
    reset_peripherals, CONNECTION_STATUS, ICONS, IDENTIFY_TRIGGER, INPUT_EVENTS, PERIPHERAL_INPUTS,
    PROFILE_NAME, RGB_CONTROLS, SCREEN_CONTROLS, SCREEN_SYSTEM_STATS, UPDATE_TRIGGER,
};

type SerialPort<'a> = FullSerialPort<'a, UsbBus, [u8; SERIAL_READ_SIZE], [u8; SERIAL_WRITE_SIZE]>;

pub const SERIAL_WRITE_SIZE: usize = 1024;
pub const SERIAL_READ_SIZE: usize = 1024;
const BUFFER_SIZE: usize = 4096;
const KEEPALIVE: u32 = 1000;

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

    fn send(serial: &mut SerialPort, rsp: &[u8]) {
        // TODO: its possible for write to drop some characters, if we're not careful.
        // we should probably handle that before we take on larger communications.
        while let Err(_) = serial.write(rsp) {
            match serial.flush() {
                Ok(_) => {}
                Err(usbd_serial::UsbError::WouldBlock) => {}
                Err(e) => {
                    defmt::error!("Failed to flush serial: {:?}", e);
                }
            };
        }
    }

    fn check_cmd(&mut self) -> Option<(Command, [u8; MAX_PACKET_SIZE])> {
        if self.buffer.len() >= 3 {
            let w1 = *self.buffer.get(0).unwrap();
            let w2 = *self.buffer.get(1).unwrap();
            let w3 = *self.buffer.get(2).unwrap();

            match decode_packet_size(w1, w2, w3) {
                Ok(size) => {
                    if self.buffer.len() >= size + 3 {
                        // dequeue packet size
                        self.buffer.dequeue();
                        self.buffer.dequeue();
                        self.buffer.dequeue();

                        // dequeue command type
                        let c = self.buffer.dequeue().unwrap();
                        let cmd = c.into();

                        let mut data = [0u8; MAX_PACKET_SIZE];
                        for i in 0..size - 1 {
                            data[i] = self.buffer.dequeue().unwrap();
                        }

                        Some((cmd, data))
                    } else {
                        None
                    }
                }
                Err(()) => {
                    error!("failed to decode packet size: {} {} {}", w1, w2, w3);
                    self.buffer.clear();
                    None
                }
            }
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn get_connection_status(&self) -> Connection {
        self.state.clone()
    }

    fn start_update(&mut self, serial: &mut SerialPort) {
        info!("Command Update");
        Self::send(serial, &[b'0', b'0', b'1', RSP_DISCONNECTED]);
        CONNECTION_STATUS.with_mut_lock(|c| *c = Connection::NotConnected(true));
        self.state = Connection::NotConnected(true);
        UPDATE_TRIGGER.with_mut_lock(|u| *u = true);
    }

    fn start_identify(&mut self, serial: &mut SerialPort) {
        info!("Command Identify");
        Self::send(serial, &[b'0', b'0', b'1', RSP_ACK]);
        IDENTIFY_TRIGGER.with_mut_lock(|u| *u = true);
    }

    pub fn update(&mut self, serial: &mut SerialPort, firmware_version: &str, device_uid: &str) {
        if self.state == Connection::Connected && self.keepalive_timer.wait().is_ok() {
            warn!("Keepalive triggered, disconnecting.");
            reset_peripherals(false);
            self.state = Connection::NotConnected(false);
            self.buffer.clear();
            warn!("Peripherals reset.");
        }

        let mut buf = [0u8; 128];
        match serial.read(&mut buf) {
            Err(_) => {}
            Ok(s) => {
                // copy read data to internal buffer
                for b in 0..s {
                    let _ = self.buffer.enqueue(buf[b]);
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
            error!("unknown command: {}", data);
            Self::send(serial, RSP_FULL_UNKNOWN);
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
                    let inputs = PERIPHERAL_INPUTS.with_lock(|i| *i);

                    // write all the inputs out
                    match inputs {
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
                Command::SetInputEvent => {
                    let slot = data[0];
                    let new_input = InputEvent::decode(&data[1..7 + 1]);
                    INPUT_EVENTS.with_mut_lock(|e| e[slot as usize] = new_input);

                    Self::send(serial, RSP_FULL_ACK);

                    true
                }
                Command::SetRgbMode => {
                    let rgb = RgbProfile::decode(&data);
                    RGB_CONTROLS.with_mut_lock(|c| *c = (true, rgb));

                    Self::send(serial, RSP_FULL_ACK);

                    true
                }
                Command::SetScrIcon => {
                    let slot = data[0];
                    let new_icon = &data[1..32 * 32 * 2 + 1];

                    ICONS.with_mut_lock(|icons| {
                        let scr_icon = &mut icons[slot as usize];
                        let mut i = 0;
                        while i < 32 * 32 {
                            let (r, g, b) = split_to_rgb565(
                                ((new_icon[i * 2 + 1] as u16) << 8) | (new_icon[i * 2] as u16),
                            );
                            let c = Bgr565::new(b, g, r);
                            scr_icon.1[i] = c;
                            i += 1;
                        }
                        scr_icon.0 = true;
                    });

                    Self::send(serial, RSP_FULL_ACK);

                    true
                }
                Command::SetScrMode => {
                    let profile = ScreenProfile::decode(&data);

                    SCREEN_CONTROLS.with_mut_lock(|p| *p = (true, profile));

                    Self::send(serial, RSP_FULL_ACK);

                    true
                }
                Command::SetSystemStats => {
                    let stats = SystemStats::decode(&data);

                    SCREEN_SYSTEM_STATS.with_mut_lock(|s| *s = (true, stats));

                    Self::send(serial, RSP_FULL_ACK);

                    true
                }
                Command::SetProfileName => {
                    let new_profile_name: ProfileName = SmallStr::decode(&data);

                    PROFILE_NAME.with_mut_lock(|p| *p = (true, new_profile_name));

                    Self::send(serial, RSP_FULL_ACK);

                    true
                }
                Command::Identify => {
                    self.start_identify(serial);
                    true
                }
                Command::Update => {
                    self.start_update(serial);
                    true
                }
                Command::Disconnect => {
                    Self::send(serial, RSP_FULL_DISCONNECTED);
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
            // info!("restarting keepalive for command: {}", decode);
            self.keepalive_timer.start(KEEPALIVE.millis());
            // restart keepalive timer with valid command
        }
    }
}
