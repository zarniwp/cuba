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
    AppView, UpdateHandler, ViewId,
    egui_widgets::ProgressSpinner,
    task_progress::{TaskMessageType, TaskProgress},
    util::make_cuba_runner,
};

/// Defines a `RestoreView`.
pub struct RestoreView {
    run_handle: RunHandle,
    sender: Sender<Arc<dyn Message>>,
    cuba: Arc<RwLock<Cuba>>,
    selected_profiles: HashSet<String>,
    msg_dispatcher: Arc<MsgDispatcher<Arc<dyn Message>>>,
    task_progress: Arc<TaskProgress>,
}

/// Methods of `RestoreView`.
impl RestoreView {
    /// Creates a new `RestoreView`.
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

/// Impl of `AppView` for `RestoreView`.
impl AppView for RestoreView {
    /// Returns the name of the view.
    fn name(&self) -> &str {
        "Restore"
    }

    /// Returns the view id.
    fn view_id(&self) -> ViewId {
        ViewId::Restore
    }

    /// Renders the view for egui.
    fn ui(&mut self, ui: &mut egui::Ui) {
        // Set height.
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
                            for profile in config.restore.keys() {
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
                // Set height.
                ui.set_height(height);

                if let Some(config) = self.cuba.read().unwrap().config() {
                    if self.task_progress.transfer_threads() != config.transfer_threads {
                        self.task_progress
                            .set_transfer_threads(config.transfer_threads);
                    }

                    // Profile(s) information.
                    let mut profiles = String::new();

                    // Gather information.
                    for selected_profile in &self.selected_profiles {
                        if profiles.is_empty() {
                            profiles = selected_profile.clone();
                        } else {
                            profiles = format!("{}, {}", profiles, selected_profile);
                        }
                    }

                    // Profiles label.
                    ui.heading(format!("Profile(s): {}", profiles));

                    // Separator.
                    ui.separator();

                    // The task message table.
                    egui::Grid::new("Tasks").show(ui, |ui| {
                        for thread_number in 0..config.transfer_threads {
                            ui.add(
                                ProgressSpinner::new(
                                    &self.task_progress.get_task_progress(thread_number),
                                )
                                .size(16.0)
                                .spinning_color(Color32::WHITE)
                                .invalid_color(Color32::DARK_GRAY),
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
                            // The restore button.
                            if ui.button("Start Restore").clicked() {
                                run(
                                    "Restore".to_string(),
                                    Box::new(|cuba, run_handle, profile| {
                                        cuba.read().unwrap().run_restore(run_handle, &profile)
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
