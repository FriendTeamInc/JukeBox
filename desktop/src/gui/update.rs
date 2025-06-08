use eframe::egui::{vec2, Button, Color32, ProgressBar, RichText, Ui};
use rfd::FileDialog;
use tokio::spawn;

use crate::firmware_update::{firmware_update_task, FirmwareUpdateStatus};
use crate::get_reqwest_client;
use crate::serial::SerialCommand;
use crate::software_update::get_github_release;

use super::gui::JukeBoxGui;

impl JukeBoxGui {
    fn send_custom_firmware_update_signal(&mut self, fw_path: String) {
        {
            let scmd_txs = self.scmd_txs.blocking_lock();
            if let Some(tx) = scmd_txs.get(&self.current_device) {
                tx.send(SerialCommand::Update)
                    .expect("failed to send update command");
            }
        }

        let fw = std::fs::read(fw_path).unwrap();

        let us_tx2 = self.us_tx.clone();
        let device_uid = self.current_device.clone();
        spawn(async move {
            firmware_update_task(device_uid, fw, us_tx2).await;
        });
    }

    fn send_update_signal(&mut self) {
        {
            let scmd_txs = self.scmd_txs.blocking_lock();
            if let Some(tx) = scmd_txs.get(&self.current_device) {
                tx.send(SerialCommand::Update)
                    .expect("failed to send update command");
            }
        }

        let us_tx2 = self.us_tx.clone();
        let device_uid = self.current_device.clone();
        spawn(async move {
            let fw = match get_github_release("FriendTeamInc", "JukeBox", "latest").await {
                Ok(release) => {
                    // TODO: maybe signal in gui that a firmware is being downloaded and if any errors occur?
                    let fw_asset = release
                        .assets
                        .iter()
                        .filter(|a| a.name == "jukebox_firmware.uf2")
                        .next()
                        .unwrap()
                        .clone();
                    Some(
                        get_reqwest_client()
                            .get(fw_asset.browser_download_url)
                            .send()
                            .await
                            .unwrap()
                            .bytes()
                            .await
                            .unwrap()
                            .to_vec(),
                    )
                }
                Err(e) => {
                    log::warn!("firmware update release error: {:?}", e);
                    None
                }
            }
            .unwrap();

            firmware_update_task(device_uid, fw, us_tx2).await;
        });
    }

    pub fn draw_update_page(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.allocate_space(vec2(0.0, 5.0));
            ui.heading(t!("update.title"));
            ui.allocate_space(vec2(0.0, 3.0));

            let version = self
                .devices
                .get(&self.current_device)
                .and_then(|d| d.firmware_version.clone())
                .and_then(|v| Some(v.to_string()));
            ui.label(t!(
                "update.current_firmware_version",
                version = version.unwrap_or_default()
            ));
            ui.label(t!(
                "update.new_firmware_version",
                version = self.available_version.to_string()
            ));

            ui.allocate_space(vec2(0.0, 5.0));
            ui.label(RichText::new(t!("update.warning")).color(Color32::DARK_RED)); // TODO

            ui.allocate_space(vec2(0.0, 13.0));

            ui.horizontal(|ui| {
                let dl_update =
                    Button::new(RichText::new(t!("update.button"))).min_size(vec2(150.0, 30.0));
                let cfw_update = Button::new(RichText::new(t!("update.cfw_button")).size(8.0));

                ui.allocate_space(vec2(149.0, 0.0));

                if self.update_status != FirmwareUpdateStatus::Start {
                    ui.disable();
                }

                if ui.add(dl_update).clicked() {
                    log::debug!("doing standard update with github");
                    self.send_update_signal();
                }
                if ui.add(cfw_update).clicked() {
                    if let Some(f) = FileDialog::new()
                        .add_filter(t!("update.filter_name"), &["uf2"])
                        .set_directory("~")
                        .pick_file()
                    {
                        let p = f.to_string_lossy().into();
                        log::debug!("doing cfw update with {}", p);
                        self.send_custom_firmware_update_signal(p);
                    }
                }
            });
            ui.allocate_space(vec2(0.0, 10.0));
            ui.horizontal(|ui| {
                ui.allocate_space(vec2(149.0 - 12.5, 0.0));

                while let Ok(p) = self.us_rx.try_recv() {
                    self.update_status = p;
                    match p {
                        FirmwareUpdateStatus::Start => self.update_progress = 0.0,
                        FirmwareUpdateStatus::Connecting => self.update_progress = 0.05,
                        FirmwareUpdateStatus::PreparingFirmware => self.update_progress = 0.1,
                        FirmwareUpdateStatus::ErasingOldFirmware(n) => {
                            self.update_progress = 0.1 + 0.3 * n
                        }
                        FirmwareUpdateStatus::WritingNewFirmware(n) => {
                            self.update_progress = 0.4 + 0.6 * n
                        }
                        FirmwareUpdateStatus::End => self.update_progress = 1.0,
                    }
                }

                let p = ProgressBar::new(self.update_progress)
                    // .animate(true)
                    .desired_width(175.0)
                    .desired_height(30.0)
                    .show_percentage();
                ui.add(p);
            });
            ui.allocate_space(vec2(0.0, 2.0));
            ui.label(match self.update_status {
                FirmwareUpdateStatus::Start => t!("update.status.start"),
                FirmwareUpdateStatus::Connecting => t!("update.status.connecting"),
                FirmwareUpdateStatus::PreparingFirmware => t!("update.status.preparing"),
                FirmwareUpdateStatus::ErasingOldFirmware(_) => t!("update.status.erasing"),
                FirmwareUpdateStatus::WritingNewFirmware(_) => t!("update.status.writing"),
                FirmwareUpdateStatus::End => t!("update.status.end"),
            });
        });
        ui.allocate_space(ui.available_size_before_wrap());
    }
}
