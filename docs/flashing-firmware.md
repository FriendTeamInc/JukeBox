# Manually flashing new firmware on the JukeBox
This document is a step by step process on how to load new firmware onto your JukeBox, in cases where the JukeBox Desktop application can't do it for you.

## 0. Try the JukeBox desktop software!
If you can plug in your JukeBox and it connects to the desktop software, you can update the firmware in-app. In the bottom right of the device management page, click the update device button. Click update and keep the device connected, the application will download and install the latest firmware. You can also click the "CFW" button to install a custom firmware.

(TODO: add pictures)

If for whatever reason your JukeBox device no longer connects to the desktop software, you can manually install new (or custom) firmware by following these steps.

## 1. Getting your device into flashing mode
Unplug your JukeBox device from your computer. Press and hold down the FLASH button. While holding the FLASH button, plug the JukeBox device into your computer. A new USB storage device should appear named RPI-RP2 (you may need to let go of the FLASH button before it appears).

(TODO: add pictures)

## 2. Flashing the firmware
Navigate to the [latest release page](https://github.com/FriendTeamInc/JukeBox/releases/latest), and download the appropriate firmware, which is a .UF2 file. Drag and drop that .UF2 file into the RPI-RP2 storage device that appeared. After a moment, the device will disappear and the JukeBox will reboot with its new firmware installed. You should now be able to connect your JukeBox to the desktop software again.

(TODO: add pictures)
