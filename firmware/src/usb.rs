//! USB Comms
//!
//! Defines and handles all the USB communication through interrupts and async.

use core::sync::atomic::AtomicBool;

use crate::uid;

use defmt::*;

use embassy_executor::Spawner;
use embassy_rp::{
    Peri,
    peripherals::USB,
    usb::{Driver, InterruptHandler},
};
use embassy_time::Timer;
use embassy_usb::{
    UsbDevice,
    class::{
        cdc_acm::{CdcAcmClass, State as SerialState},
        hid::{
            Config as HidConfig, HidReader, HidReaderWriter, HidWriter, ReportId, RequestHandler,
            State as HidState,
        },
    },
    control::OutResponse,
    driver::EndpointError,
};
use packed_struct::PackedStruct;
use static_cell::StaticCell;
use usbd_human_interface_device::device::{
    keyboard::{NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR, NKROBootKeyboardReport},
    mouse::{WHEEL_MOUSE_REPORT_DESCRIPTOR, WheelMouseReport},
};

embassy_rp::bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

const KEYBOARD_READ_N: usize = 64;
const KEYBOARD_WRITE_N: usize = 64;
const MOUSE_READ_N: usize = 64;
const MOUSE_WRITE_N: usize = 64;

type UsbDriver = Driver<'static, USB>;
type UsbDev = UsbDevice<'static, UsbDriver>;
type UsbSerial = CdcAcmClass<'static, UsbDriver>;
type UsbKeyboard = HidReaderWriter<'static, UsbDriver, KEYBOARD_READ_N, KEYBOARD_WRITE_N>;
type UsbMouse = HidReaderWriter<'static, UsbDriver, MOUSE_READ_N, MOUSE_WRITE_N>;

pub struct UsbMod {
    usb_dev: UsbDev,
    serial: UsbSerial,
    keyboard: UsbKeyboard,
    mouse: UsbMouse,
}
impl UsbMod {
    pub fn new(p_usb: Peri<'static, USB>) -> Self {
        // Create the driver, from the HAL.
        let driver = Driver::new(p_usb, Irqs);

        // Create embassy-usb Config
        let config = {
            let usb_pid = if cfg!(feature = "keypad") {
                0xF20A
            } else if cfg!(feature = "knobpad") {
                0xF20B
            } else if cfg!(feature = "pedalpad") {
                0xF20C
            } else {
                0xF209
            };
            let usb_product = if cfg!(feature = "keypad") {
                "JukeBox KeyPad"
            } else if cfg!(feature = "knobpad") {
                "JukeBox KnobPad"
            } else if cfg!(feature = "pedalpad") {
                "JukeBox PedalPad"
            } else {
                "JukeBox Unknown Device"
            };

            let mut config = embassy_usb::Config::new(0x1209, usb_pid);
            config.manufacturer = Some("Friend Team Inc.");
            config.product = Some(usb_product);
            config.serial_number = Some(uid::get_uid());
            config.device_release = 0x0500;
            config.max_power = 500;
            config.supports_remote_wakeup = true;
            config.max_packet_size_0 = 64;
            config.composite_with_iads = true;
            config
        };

        // Create embassy-usb DeviceBuilder using the driver and config.
        // It needs some buffers for building the descriptors.
        let mut builder = {
            static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
            static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
            static CONTROL_BUF: StaticCell<[u8; 256]> = StaticCell::new();

            let builder = embassy_usb::Builder::new(
                driver,
                config,
                CONFIG_DESCRIPTOR.init([0; 256]),
                BOS_DESCRIPTOR.init([0; 256]),
                &mut [], // no msos descriptors
                CONTROL_BUF.init([0; 256]),
            );
            builder
        };

        // Create classes on the builder.
        let serial = {
            static STATE: StaticCell<SerialState> = StaticCell::new();
            CdcAcmClass::new(&mut builder, STATE.init(SerialState::new()), 64)
        };
        let keyboard = {
            static STATE: StaticCell<HidState> = StaticCell::new();
            let config = HidConfig {
                report_descriptor: NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR,
                request_handler: None,
                poll_ms: 10,
                max_packet_size: 64,
            };
            HidReaderWriter::<_, KEYBOARD_READ_N, KEYBOARD_WRITE_N>::new(
                &mut builder,
                STATE.init(HidState::new()),
                config,
            )
        };
        let mouse = {
            static STATE: StaticCell<HidState> = StaticCell::new();
            let config = HidConfig {
                report_descriptor: WHEEL_MOUSE_REPORT_DESCRIPTOR,
                request_handler: None,
                poll_ms: 10,
                max_packet_size: 64,
            };
            HidReaderWriter::<_, MOUSE_READ_N, MOUSE_WRITE_N>::new(
                &mut builder,
                STATE.init(HidState::new()),
                config,
            )
        };

        // Build the builder.
        Self {
            usb_dev: builder.build(),
            serial,
            keyboard,
            mouse,
        }
    }

    pub fn run(self, spawner: &Spawner) {
        // start USB control loop
        unwrap!(spawner.spawn(usb_run(self.usb_dev)));

        // start USB serial loop
        unwrap!(spawner.spawn(usb_serial_run(self.serial)));
        // start USB HID loops
        let (keyboard_reader, keyboard_writer) = self.keyboard.split();
        let (mouse_reader, mouse_writer) = self.mouse.split();
        unwrap!(spawner.spawn(usb_keyboard_in_run(keyboard_writer)));
        unwrap!(spawner.spawn(usb_keyboard_out_run(keyboard_reader)));
        unwrap!(spawner.spawn(usb_mouse_in_run(mouse_writer)));
        unwrap!(spawner.spawn(usb_mouse_out_run(mouse_reader)));
    }
}

static USB_SUSPENDED: AtomicBool = AtomicBool::new(false);
pub fn usb_suspended() -> bool {
    USB_SUSPENDED.load(core::sync::atomic::Ordering::Relaxed)
}

#[embassy_executor::task]
async fn usb_run(mut usb_dev: UsbDevice<'static, Driver<'static, USB>>) -> ! {
    loop {
        usb_dev.run_until_suspend().await;
        USB_SUSPENDED.store(true, core::sync::atomic::Ordering::Relaxed);
        info!("USB Suspended");
        usb_dev.wait_resume().await;
        USB_SUSPENDED.store(false, core::sync::atomic::Ordering::Relaxed);
        info!("USB Resumed");
    }
}

#[embassy_executor::task]
async fn usb_serial_run(mut usb_serial: UsbSerial) -> ! {
    // TODO: better control flow for serial processing. This just echoes right now.
    loop {
        usb_serial.wait_connection().await;
        let mut buf = [0; 64];
        loop {
            let n = match usb_serial.read_packet(&mut buf).await {
                Ok(n) => n,
                Err(e) => match e {
                    EndpointError::BufferOverflow => {
                        defmt::panic!("Buffer overflow from serial read!")
                    }
                    EndpointError::Disabled => break,
                },
            };
            let data = &buf[..n];
            info!("USB Serial Data: {:x}", data);
            match usb_serial.write_packet(data).await {
                Ok(_) => (),
                Err(e) => match e {
                    EndpointError::BufferOverflow => {
                        defmt::panic!("Buffer overflow from serial write!")
                    }
                    EndpointError::Disabled => break,
                },
            };
        }
    }
}

#[embassy_executor::task]
async fn usb_keyboard_out_run(
    keyboard_reader: HidReader<'static, UsbDriver, KEYBOARD_READ_N>,
) -> ! {
    let mut keyboard_hid_handler = HidHandler {};
    keyboard_reader.run(false, &mut keyboard_hid_handler).await;
}
#[embassy_executor::task]
async fn usb_keyboard_in_run(
    mut keyboard_writer: HidWriter<'static, UsbDriver, KEYBOARD_WRITE_N>,
) -> ! {
    loop {
        Timer::after_millis(10).await;
        // TODO: build full report with NKROBootKeyboardReport::new(keys)
        let report = NKROBootKeyboardReport::default().pack().unwrap();
        match keyboard_writer.write(&report).await {
            Ok(_) => (),
            Err(e) => warn!("failed to send keyboard report: {:?}", e),
        }
    }
}

#[embassy_executor::task]
async fn usb_mouse_out_run(mouse_reader: HidReader<'static, UsbDriver, MOUSE_READ_N>) -> ! {
    let mut mouse_hid_handler = HidHandler {};
    mouse_reader.run(false, &mut mouse_hid_handler).await;
}
#[embassy_executor::task]
async fn usb_mouse_in_run(mut mouse_writer: HidWriter<'static, UsbDriver, MOUSE_WRITE_N>) -> ! {
    loop {
        Timer::after_millis(10).await;
        let report = WheelMouseReport::default().pack().unwrap();
        match mouse_writer.write(&report).await {
            Ok(_) => (),
            Err(e) => warn!("failed to send mouse report: {:?}", e),
        }
    }
}

struct HidHandler;
impl RequestHandler for HidHandler {
    fn get_report(&mut self, _id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        None
    }

    fn set_report(&mut self, _id: ReportId, _data: &[u8]) -> OutResponse {
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, _id: Option<ReportId>, _dur: u32) {}

    fn get_idle_ms(&mut self, _id: Option<ReportId>) -> Option<u32> {
        None
    }
}
