# v0.5.0 - Tut Trials (TODO)
- Add a welcome message on very first startup with some important info for the user.
- Remove donate "link".

# v0.4.6 - I'm Blue (June 26, 2025)
- Update default theme for device when not connected to application. This now includes an indicator of USB connection status (red if not connecting, green if connected with host), the device's UID, and its firmware version. It's also blue now.
- Fix "N/A" temperature not displaying correctly on device.

# v0.4.5 - Clock Forever (June 22, 2025)
- Fix keyboard key requests being way too frequent.
- Only send RGB and Screen packets to keypad devices, futureproofing.

# v0.4.4 - Initiliazing Fixes (June 22, 2025)
- Fix potential device nickname collisions on first connection.
- Fix device not receiving default configuration on first connection.

# v0.4.3 - Patch TODOs (June 21, 2025)
- Switch Profile action will now better switch its target profile when the original target profile is deleted or renamed.
- Disable CPU temperature read out on Windows (since Windows and sysinfo do not have methods to read temperature currently).
- Add info statement to list device firmware and UID on connection.
- Fix application remembering device firmware of a disconnected device.
- Take last 4 characters of UID for name instead of first 4 due to collisions.
- Fix panics and stuck loops when packet length read fails.
- Fix stuck loop when serial disconnects with data in its buffer.
- The above fixes do not solve the random disconnects that serial experiences, and will need to be investigated further later. For some reason, either the application drops random bytes, or the firmware consumes random bytes.

# v0.4.2 - IADs Fix (June 15, 2025)
- Fix device not functioning on Windows due to misconfigured USB descriptors.

# v0.4.1 - Feelin' Hot (June 14, 2025)
- Add preliminary CPU temperature reading support. This may only work on AMD-based CPU's, and has been only tested on Linux.

# v0.4.0 - Mouse Controls, Not for Mice (June 14, 2025)
- Add mouse event action in app and firmware.
- Fix device not appearing correctly as composite device.
- Fix device not requesting enough power.
- Fix device max packet size to be larger for performance.
- Update some firmware dependencies.

# v0.3.1 - Update Fixes (June 13, 2025)
- Fix inability to leave other screens after a failed update occurred.

# v0.3.0 - Icon and Save Update (June 13, 2025)
- Images of arbitrary size can be used as icons now.
- Device connection status is now more prominent.
- Added popup warning for if your config file failed to load.
- Added explicit save button on configuration screens.
- Further increase timeout of serial to be at most 250 milliseconds.

# v0.2.5 - More Stability (June 12, 2025)
- Fix USB mass storage device from appearing when the JukeBox is put into update mode from software.
- Fix firmware update confirmation button translation string.
- Adjust USB serial internal buffer sizes in firmware.
- Revert tray icon functionality for Windows while we wait for egui to add `App::tick()`.
- Force timeout of serial to be at most 100 milliseconds.
- Update firmware dependencies.

# v0.2.4 - Firmware Update Stability (June 11, 2025)
- Update internal libraries. This specifically addresses an issue where the firmware update page may fail when no drivers are available on Windows.
- Added a message to the firmware update error page to reconnect the device physically or manually update the firmware.

# v0.2.3 - Iconic Async (June 11, 2025)
- Fix icon device upload in async thread.

# v0.2.2 - Debugging Causes Problems (June 9, 2025)
- Add pop up for any errors from the update process.
- Allow console window on Windows, for any panics to be traced. This will be removed later.
- Disable blocking in firmware debug output, which causes firmware to eventually freeze.

# v0.2.1 - Albrecht Entrati (June 8, 2025)
- Major refactor internally for actions. Old config files will be discarded!
- Add lots of debug prints for the update process.
- Fix config directory missing for app lock purposes.
- Begin push for Discord integration available by default. Unfinished.
- Changed delete profile and forget device to be single click instead of double click.
- The save and exit pop up will now only show when a change has actually been made.
- Drop some old dependencies from the desktop app.

# v0.2.0 - Debugging Days (June 8, 2025)
- Changed default brightness for RGB from 255 to 100.
- Added log file for testers to send in.
- Prevent new instances of the desktop application from launching.

# v0.1.3 - Removing Some Unfinished Work (June 3, 2025)
- Fix crash when GPU name is too long.
- Rename Profile Name Color to Text Color in screen configuration.
- Finish brightness in screen configuration.
- Disabled Soundboard functionality, since it is currently unfinished.
- Disabled Wave RGB pattern, since it is currently unfinished.
- Vertically expanded the window, since on certain platforms the window is cut off at the bottom.

# v0.1.2 - Allow Updating Firmware (June 1, 2025)
- Desktop application can now download and apply new firmware.

# v0.1.1 - Minor Update Notif Fix (June 1, 2025)
- Change the Update Notification in the desktop application to show the current and new version.
- Update desktop app dependencies.
- Add changelog.

# v0.1.0 - Beta Begins (June 1, 2025)
Initial release.
