use crate::uid;

use embassy_rp::{
    Peri,
    peripherals::USB,
    usb::{Driver, InterruptHandler},
};
use embassy_usb::{
    UsbDevice,
    class::{
        cdc_acm::{CdcAcmClass, State as SerialState},
        hid::{Config as HidConfig, HidReaderWriter, State as HidState},
    },
};
use static_cell::StaticCell;
use usbd_human_interface_device::device::{
    keyboard::NKRO_BOOT_KEYBOARD_REPORT_DESCRIPTOR, mouse::WHEEL_MOUSE_REPORT_DESCRIPTOR,
};

embassy_rp::bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

const KEYBOARD_READ_N: usize = 1;
const KEYBOARD_WRITE_N: usize = 8;
const MOUSE_READ_N: usize = 1;
const MOUSE_WRITE_N: usize = 8;

pub struct UsbDev {
    usb_dev: UsbDevice<'static, Driver<'static, USB>>,
    serial: CdcAcmClass<'static, Driver<'static, USB>>,
    keyboard: HidReaderWriter<'static, Driver<'static, USB>, KEYBOARD_READ_N, KEYBOARD_WRITE_N>,
    mouse: HidReaderWriter<'static, Driver<'static, USB>, MOUSE_READ_N, MOUSE_WRITE_N>,
}

pub fn build_usb(p_usb: Peri<'static, USB>) -> UsbDev {
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
        config
    };

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        let builder = embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
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
    UsbDev {
        usb_dev: builder.build(),
        serial,
        keyboard,
        mouse,
    }
}
