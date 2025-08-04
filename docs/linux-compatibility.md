# Linux Compatibility
You must add yourself to the `dialout` group in order for device communication to function: `sudo usermod -a -G dialout $USER`

You must also install the udev rules below for device communication to function.

## udev Rules
The following udev rules allow device communication access for all users in the dialout group, and device firmware update permissions for all users in the plugdev group. The latter is only necessary for manual updating.
```udev
SUBSYSTEM=="tty", ATTRS{idVendor}=="1209", ATTRS{idProduct}=="f209", TAG+="uaccess" MODE="0660", GROUP="dialout"
SUBSYSTEM=="tty", ATTRS{idVendor}=="1209", ATTRS{idProduct}=="f20a", TAG+="uaccess" MODE="0660", GROUP="dialout"
SUBSYSTEM=="tty", ATTRS{idVendor}=="1209", ATTRS{idProduct}=="f20b", TAG+="uaccess" MODE="0660", GROUP="dialout"
SUBSYSTEM=="tty", ATTRS{idVendor}=="1209", ATTRS{idProduct}=="f20c", TAG+="uaccess" MODE="0660", GROUP="dialout"
SUBSYSTEM=="usb", ATTRS{idVendor}=="2e8a", ATTRS{idProduct}=="0003", TAG+="uaccess" MODE="0660", GROUP="plugdev"
```

## Dependencies
Debian/Ubuntu:
`sudo apt install libgtk-3-dev libxdo-dev libayatana-appindicator3-dev`
