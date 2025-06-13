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
