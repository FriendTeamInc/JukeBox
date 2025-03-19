use eframe::egui::{vec2, Button, ProgressBar, RichText, Ui};
use rfd::FileDialog;
use tokio::spawn;

use crate::serial::SerialCommand;
use crate::update::{update_task, UpdateStatus};

use super::gui::JukeBoxGui;

impl JukeBoxGui {
    fn send_update_signal(&mut self, fw_path: String) {
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
            update_task(device_uid, fw_path, us_tx2).await;
        });
    }

    pub fn draw_update_page(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.allocate_space(vec2(0.0, 10.0));
            ui.heading(t!("update.title"));

            // TODO: add some basic info (firmware versions, "do not uplug or power off", etc)
            ui.allocate_space(vec2(0.0, 75.0));

            ui.horizontal(|ui| {
                let dl_update =
                    Button::new(RichText::new(t!("update.button"))).min_size(vec2(150.0, 30.0));
                let cfw_update = Button::new(RichText::new(t!("update.cfw_button")).size(8.0));

                ui.allocate_space(vec2(149.0, 0.0));

                if self.update_status != UpdateStatus::Start {
                    ui.disable();
                }

                if ui.add(dl_update).clicked() {
                    // TODO: download update from GitHub
                    // self.send_update_signal(???);
                }
                if ui.add(cfw_update).clicked() {
                    // TODO: ask for file, verify its good, then use it to update the device
                    if let Some(f) = FileDialog::new()
                        .add_filter(t!("update.filter_name"), &["uf2"])
                        .set_directory("~")
                        .pick_file()
                    {
                        self.send_update_signal(f.to_string_lossy().into());
                    }
                }
            });
            ui.allocate_space(vec2(0.0, 25.0));
            ui.horizontal(|ui| {
                ui.allocate_space(vec2(149.0, 0.0));

                while let Ok(p) = self.us_rx.try_recv() {
                    self.update_status = p;
                    match p {
                        UpdateStatus::Start => self.update_progress = 0.0,
                        UpdateStatus::Connecting => self.update_progress = 0.05,
                        UpdateStatus::PreparingFirmware => self.update_progress = 0.1,
                        UpdateStatus::ErasingOldFirmware(n) => self.update_progress = 0.1 + 0.3 * n,
                        UpdateStatus::WritingNewFirmware(n) => self.update_progress = 0.4 + 0.6 * n,
                        UpdateStatus::End => self.update_progress = 1.0,
                    }
                }

                let p = ProgressBar::new(self.update_progress)
                    // .animate(true)
                    .desired_width(150.0)
                    .desired_height(30.0)
                    .show_percentage();
                ui.add(p);
            });
            ui.allocate_space(vec2(0.0, 10.0));
            ui.label(match self.update_status {
                UpdateStatus::Start => t!("update.status.start"),
                UpdateStatus::Connecting => t!("update.status.connecting"),
                UpdateStatus::PreparingFirmware => t!("update.status.preparing"),
                UpdateStatus::ErasingOldFirmware(_) => t!("update.status.erasing"),
                UpdateStatus::WritingNewFirmware(_) => t!("update.status.writing"),
                UpdateStatus::End => t!("update.status.end"),
            });
        });
        ui.allocate_space(ui.available_size_before_wrap());
    }
}
