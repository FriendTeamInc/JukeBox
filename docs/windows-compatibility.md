# Windows Compatibility
Windows users should not need any extra software or drivers to use the JukeBox device or desktop application. However, updating the firmware using the desktop application does require the installation of a Zadig driver.

## Updating the Device Firmware
If you have not done so before, when updating the device firmware you will need to install a driver for the device to be updated by the desktop application. To do so, follow these steps:

1. Connect your device to your computer, open the application, and open the firmware update page for your device when the device connects.
2. Click the update button. The device will reboot with a blank screen, and the update will fail with an error. Do NOT unplug the device yet.
3. Download and open [Zadig](https://zadig.akeo.ie/). In the dropdown, find and select the device named "RP2 Boot". For the driver, select WinUSB and click the install button. You are free to close Zadig now.
4. Now, reconnect your device to your computer, it should appear to function as it did before.
5. Navigate back to the firmware update page for your device, and attempt to update the device. It should reboot successfully with the latest firmware installed.

From now on you should be free to install new firmware using just the desktop application!

## Problematic Software
The following programs are known to cause issue with serial ports, which JukeBox uses to communicate between the device and the application over USB. When these applications run, it may be impossible for the JukeBox device to connect to the desktop application.

- [UltiMaker Cura](https://ultimaker.com/software/ultimaker-cura/)
- [NZXT CAM](https://nzxt.com/pages/cam)
