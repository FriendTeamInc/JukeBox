# Updating Your JukeBox Device Firmware
There are two methods for updating your JukeBox, using the desktop application and manually flashing the firmware.

## 1. Updating via the Desktop Application
1. Connect your device to your computer and open the desktop application. Your device is connected when the desktop application shows a green plug symbol.
2. Click the update button next to the green plug symbol to open the update page. Do note, that if you haven't already, you may need to configure the device to be allowed to update on [Windows with Zadig](docs/windows-compatibility.md#updating-the-device-firmware) or [Linux with udev rules](docs/windows-compatibility.md#updating-the-device-firmware).
3. Click the front and center update button, and allow the application to flash new firmware onto the device. You can also load a custom firmware file with the CFW button, though this is not recommended nor supported.
4. All done! Enjoy your updated device!

## 2. Manually Flashing New Firmware on Tour Device
1. Unplug the device from your computer.
2. Locate the BOOT button on your device. Hold this button down.
    - On the JukeBox KeyPad, this BOOT button is above the rightmost key next to the screen.
3. While holding the button, connect the device to your computer. The device SHOULD NOT boot or light up normally.
4. Let go of the BOOT button. A USB storage device should appear in your file explorer, named something like "RPI-RP2".
5. Download the latest firmware [here](https://github.com/FriendTeamInc/JukeBox/releases/latest). The firmware is a UF2 file.
6. Drag and drop the firmware into the "RPI-RP2" storage device and wait for the transfer to complete. When finished, your JukeBox device will reboot with the latest firmware installed.
7. All done! Enjoy your updated device!
