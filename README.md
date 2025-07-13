# <img src="assets/textlogo.svg" alt="JukeBox!" width="400"/>

An affordable, adorable, powerful hotkey device to run macros, play funny sounds, managing your audio settings, change paint brush settings, configuring items in OBS, and so much more!

(TODO: physical image here)

# Disclaimer
While this project is public, it is not ready for any sort of prime time. Any releases prior to 1.0 are considered beta.

## Docs
If you are the proud owner of a new JukeBox, or are building your own, take a look at the [getting started doc](docs/getting-started.md).

## Desktop Software
Made with Rust and egui for Windows and Linux, curates the user experience with the JukeBox.

### Building
It's as simple as running `cargo build --release`.

### Discord support
Discord, currently, will not provide support to new projects using their RPC protocol. Because of this, JukeBox Desktop will not support Discord out of the box, despite having functionality for it built in. To use the Discord functionality in JukeBox Desktop, you must do the following:
1. Go to https://discord.com/developers/applications/ and log in with your account.
2. Create a new application and name it JukeBoxDesktop.
3. Go to the OAuth2 settings tab.
4. Copy the client ID and client secret down to a safe location.
5. Set the redirect to `https://localhost:61961`. Save your changes.
6. Go to the App Testers settings tab.
7. Add your Discord account as a tester. An email will be sent to your account email, go to your inbox and accept the invite.
8. Build with the following command: `DISCORD_CLIENT_ID="<PUT_CLIENT_ID_HERE>" DISCORD_CLIENT_SECRET="<PUT_CLIENT_SECRET_HERE>" cargo run --features "discord" --release`.

## Device Firmware
TODO

### Building
1. (Linux only) Add yourself to the dialout group with the following: `sudo usermod -a -G dialout $USER`. Install the [RPi Pico udev rules](https://github.com/raspberrypi/picotool/blob/master/udev/99-picotool.rules). Add the following rules:
```
SUBSYSTEM=="usb", \
    ATTRS{idVendor}=="2e8a", \
    ATTRS{idProduct}=="000c", \
    TAG+="uaccess" \
    MODE="660", \  
    GROUP="plugdev"
```

2. Install the appropriate target toolchain: `rustup target add thumbv6m-none-eabi`.
3. Install [cmake](https://cmake.org/download/) for tool compilation.
4. (Linux only) Install libudev-dev: `sudo apt install libudev-dev`.
5. Install tools: `cargo install flip-link`.
5. Install tools: `cargo install --locked probe-rs-tools`. This is for installing firmware over Pico probe.
6. Run `cargo run -F "keypad"` to install the keypad firmware to the connected device. You can use pedalpad or knobpad too.

## Case
Made with OpenSCAD 2025.03.31 (development snapshot), protects everything inside the JukeBox. You can get the printable STLs with the `build.sh` script.

## PCB
Made with KiCad 8, the external brains of the JukeBox.

Footprints and models:
- [Cherry/Kailh Switch footprint based on this.](https://github.com/luke-schutt/Pi5Keyboard/blob/main/Pi5-pcb/Pi5Footprints.pretty/Low%20Profile%20GC%20plus%20MX.kicad_mod)

Estimated power usage is 5 volts at 0.1 amps, or 0.5 watts.

### Bill of Materials
| Ref            | Part No.               | Qty | Value |
|----------------|------------------------|-----|-------|
| R1, R5, R14    | RMCF0402FT1K00         | 3   | 1k    |
| R3, R4         | RMCF0402FT27R0         | 2   | 27    |
| R6, R7         | RMCF0402FT5K11         | 2   | 5.11k |
| R8-12          | RMCF0402FT10K0         | 5   | 10k   |
| R13            | RMCF0402FT10R0         | 1   | 10    |
| R15            | RMCF0402FT33R0         | 1   | 33    |
| C1, C2         | GRM21BR61C106KE15K     | 2   | 10u   |
| C3, C4         | GCM1555C1H150JA16D     | 2   | 15p   |
| C5, C9, C11-18 | GRM155R71E104KE14J     | 1   | 100n  |
| C6-8, C10      | GRM155R71E472KA01D     | 4   | 4.7u  |
| L1             | AOTA-B201610S3R3-101-T | 1   | 3.3u  |
| D1-12          | 1N4148W-SOD-123        | 12  |       |
| D13-24         | WS2812B-2020           | 12  |       |
| D25            | 150080GS75000          | 1   |       |
| J1             | USB4105-GF-A           | 1   |       |
| Q1             | S8050-SOT-23           | 1   |       |
| SW1-12         | Keyboard Key Switch    | 12  |       |
| SW13           | RS-282G05A3-SM RT      | 1   |       |
| U1             | AZ1117IH-3.3TRG1       | 1   |       |
| U2             | W25Q128JVSIQ-TR        | 1   |       |
| U3             | RP2350A                | 1   |       |
| U4             | TFTQ-T20ST22ZP01       | 1   |       |
| U4             | FH34SRJ-22S-0.5SH(50)  | 1   |       |
| U5             | CAT24C512              | 1   |       |
| Y1             | ABM8-272-T3            | 1   |       |

# License
Copyright (c) 2020-2025 Logan "NotQuiteApex" Hickok-Dickson

All programming files, found in [`firmware/`](firmware/), [`software/`](software/), and [`case/`](case/), are licensed under the [MIT license](https://mit-license.org/).

All CAD files, found in [`hardware-pcb/`](hardware-pcb/), unless otherwise provided by an external source such as footprints or 3D models, are licesned under the [CC BY-NC-SA license](https://creativecommons.org/licenses/by-nc-sa/4.0/).

Both licenses can be found with the given links or as files in the root of this repostiory.

If you would like to sell a variation of the board designed by you, reach out and an alternative license can be discussed and granted.
