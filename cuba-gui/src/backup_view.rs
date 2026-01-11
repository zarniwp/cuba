use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use crossbeam_channel::Sender;
use cuba_lib::{
    core::cuba::{Cuba, RunHandle},
    shared::{message::Message, msg_dispatcher::MsgDispatcher},
};
use egui::Color32;

use crate::{
    AppView, UpdateHandler,
    egui_widgets::progress_spinner_widget,
    task_progress::{TaskMessageType, TaskProgress},
    util::make_cuba_runner,
};

/// Defines a `BackupView`.
pub struct BackupView {
    run_handle: RunHandle,
    sender: Sender<Arc<dyn Message>>,
    cuba: Arc<RwLock<Cuba>>,
    selected_profiles: HashSet<String>,
    msg_dispatcher: Arc<MsgDispatcher<Arc<dyn Message>>>,
    task_progress: Arc<TaskProgress>,
}

/// Methods of `BackupView`.
impl BackupView {
    /// Creates a new `BackupView`.
    pub fn new(
        egui_context: egui::Context,
        sender: Sender<Arc<dyn Message>>,
        cuba: Arc<RwLock<Cuba>>,
        msg_dispatcher: Arc<MsgDispatcher<Arc<dyn Message>>>,
    ) -> Self {
        let task_progress = Arc::new(TaskProgress::new(UpdateHandler::new(egui_context.clone())));

        Self {
            run_handle: RunHandle::default(),
            sender,
            cuba,
            selected_profiles: HashSet::new(),
            msg_dispatcher,
            task_progress,
        }
    }
}

/// Impl of `AppView` for `BackupView`.
impl AppView for BackupView {
    /// Returns the name of the view.
    fn name(&self) -> &str {
        "Backup"
    }

    /// Renders the view for egui.
    fn ui(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();

        // Horizontal layout (profile list, profile content).
        ui.horizontal(|ui| {
            // Vertical layout (heading, list).
            ui.vertical(|ui| {
                ui.set_width(200.0);
                ui.set_height(height);

                // Profile list heading.
                ui.heading("Profiles");

                // Separator.
                ui.separator();

                // Profile list.
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        if let Some(config) = self.cuba.read().unwrap().config() {
                            for profile in config.backup.keys() {
                                let selected = self.selected_profiles.contains(profile);

                                if ui.selectable_label(selected, profile).clicked() {
                                    if selected {
                                        self.selected_profiles.remove(profile);
                                    } else {
                                        self.selected_profiles.insert(profile.clone());
                                    }
                                }
                            }
                        }
                    });
            });

            // Separator.
            ui.separator();

            // Vertical layout (profile content).
            ui.vertical(|ui| {
                ui.set_height(height);

                if let Some(config) = self.cuba.read().unwrap().config() {
                    if self.task_progress.transfer_threads() != config.transfer_threads {
                        self.task_progress
                            .set_transfer_threads(config.transfer_threads);
                    }

                    // Profile(s) information.
                    let mut profiles = String::new();
                    let mut compression = String::new();
                    let mut encyption = String::new();

                    // Gather information.
                    for selected_profile in &self.selected_profiles {
                        if let Some(backup_profile) = config.backup.get(selected_profile) {
                            if profiles.is_empty() {
                                profiles = selected_profile.clone();
                            } else {
                                profiles = format!("{}, {}", profiles, selected_profile);
                            }

                            if compression.is_empty() {
                                if backup_profile.compression {
                                    compression = "Yes".to_string();
                                } else {
                                    compression = "No".to_string();
                                }
                            } else if backup_profile.compression {
                                compression = format!("{}, {}", compression, "Yes");
                            } else {
                                compression = format!("{}, {}", compression, "No");
                            }

                            if encyption.is_empty() {
                                if backup_profile.encrypt {
                                    encyption = "Yes".to_string();
                                } else {
                                    encyption = "No".to_string();
                                }
                            } else if backup_profile.encrypt {
                                encyption = format!("{}, {}", encyption, "Yes");
                            } else {
                                encyption = format!("{}, {}", encyption, "No");
                            }
                        }
                    }

                    // Profiles label.
                    ui.heading(format!("Profile(s): {}", profiles));

                    // Separator.
                    ui.separator();

                    // The compress label.
                    ui.label(format!("Compression: {}", compression));

                    // The encryption label.
                    ui.label(format!("Encyption: {}", encyption));

                    // Separator.
                    ui.separator();

                    // The task message table.
                    egui::Grid::new("Tasks").show(ui, |ui| {
                        for thread_number in 0..config.transfer_threads {
                            progress_spinner_widget(
                                ui,
                                &self.task_progress.get_task_progress(thread_number),
                                Color32::WHITE,
                                Color32::DARK_GRAY,
                                16_f32,
                            );

                            let task_message = self.task_progress.get_task_message(thread_number);

                            let msg_color = match task_message.msg_type {
                                TaskMessageType::Info => Color32::LIGHT_GREEN,
                                TaskMessageType::Error => Color32::LIGHT_RED,
                            };

                            ui.label(
                                egui::RichText::new(task_message.message)
                                    .monospace()
                                    .color(msg_color),
                            );

                            ui.label(
                                egui::RichText::new(task_message.path)
                                    .monospace()
                                    .color(Color32::LIGHT_GRAY),
                            );

                            ui.end_row();
                        }
                    });

                    // Separator.
                    ui.separator();

                    // The progress bar.
                    let progress = self.task_progress.get_total_progress().normalized();

                    ui.add(
                        egui::ProgressBar::new(progress).text(
                            egui::RichText::new(format!("{:.1} %", progress * 100.0))
                                .monospace()
                                .color(Color32::LIGHT_GRAY),
                        ),
                    );

                    // Separator.
                    ui.separator();

                    // Prepare a runner.
                    let run = make_cuba_runner(
                        self.run_handle.clone(),
                        self.sender.clone(),
                        self.cuba.clone(),
                        self.selected_profiles.clone(),
                        self.msg_dispatcher.clone(),
                        self.task_progress.clone(),
                    );

                    // Horizontal layout (run buttons).
                    ui.horizontal(|ui| {
                        if self.run_handle.is_running() {
                            if self.run_handle.is_canceled() {
                                ui.label("Canceling ...");
                            } else {
                                // The cancel button.
                                if ui.button("Cancel").clicked() {
                                    self.run_handle.request_cancel();
                                }
                            }
                        } else {
                            // The backup button.
                            if ui.button("Start Backup").clicked() {
                                run(
                                    "Backup".to_string(),
                                    Box::new(|cuba, run_handle, profile| {
                                        cuba.read().unwrap().run_backup(run_handle, &profile)
                                    }),
                                );
                            }

                            // The verify new button.
                            if ui.button("Start Verify new").clicked() {
                                run(
                                    "Verify".to_string(),
                                    Box::new(|cuba, run_handle, profile| {
                                        cuba.read()
                                            .unwrap()
                                            .run_verify(run_handle, &profile, &false)
                                    }),
                                );
                            }

                            // The verify all button.
                            if ui.button("Start Verify all").clicked() {
                                run(
                                    "Verify".to_string(),
                                    Box::new(|cuba, run_handle, profile| {
                                        cuba.read().unwrap().run_verify(run_handle, &profile, &true)
                                    }),
                                );
                            }

                            // The clean button.
                            if ui.button("Start Clean").clicked() {
                                run(
                                    "Clean".to_string(),
                                    Box::new(|cuba, run_handle, profile| {
                                        cuba.read().unwrap().run_clean(run_handle, &profile)
                                    }),
                                );
                            }
                        }
                    });
                }
            });
        });
    }
}
