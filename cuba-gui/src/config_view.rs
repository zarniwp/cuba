#![allow(dead_code)]

use std::sync::{Arc, RwLock};

use cuba_lib::{
    core::cuba::Cuba,
    shared::{
        config::{ConfigEntryKey, ConfigEntryMut},
        npath::{Abs, Dir, Rel},
    },
};

use crate::{
    AppView,
    egui_widgets::{NPathEditor, NPathEditorBuffer, build_row, label_value_table},
};

/// Defines a `ConfigView`.
pub struct ConfigView {
    egui_context: egui::Context,
    cuba: Arc<RwLock<Cuba>>,
    selected_config_entry_key: Option<ConfigEntryKey>,
    npath_editor_buffer: NPathEditorBuffer,
}

/// Methods of `ConfigView`.
impl ConfigView {
    /// Creates a new `ConfigView`.
    pub fn new(egui_context: egui::Context, cuba: Arc<RwLock<Cuba>>) -> Self {
        Self {
            egui_context,
            cuba,
            selected_config_entry_key: None,
            npath_editor_buffer: NPathEditorBuffer::new(),
        }
    }
}

/// Methods of `ConfigView`.
impl ConfigView {
    /// Renders the config entry_editor.
    fn render_entry_editor(&mut self, ui: &mut egui::Ui) {
        if let Some(config) = self.cuba.write().unwrap().config_mut() {
            let fs_entries = config.list_fs_keys();

            if let Some(entry_key) = &self.selected_config_entry_key
                && let Some(entry) = config.get_entry_mut(entry_key)
            {
                let row_height: f32 = 25.0;

                match entry {
                    ConfigEntryMut::LocalFS(local_fs) => {
                        ui.heading(format!("LocalFS: {}", entry_key.name));

                        // Separator.
                        ui.separator();

                        let label_width = egui_extras::Size::exact(40.0);

                        // The local fs table.
                        label_value_table(ui, 1, row_height, |rows| {
                            build_row(
                                rows,
                                label_width,
                                "Dir:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.add(NPathEditor::<Abs, Dir>::new(
                                        &entry_key.to_string(),
                                        &mut local_fs.dir,
                                        &mut self.npath_editor_buffer,
                                    ));
                                },
                            );
                        });
                    }
                    ConfigEntryMut::WebDAVFS(webdav_fs) => {
                        ui.heading(format!("WebDAV: {}", entry_key.name));

                        // Separator.
                        ui.separator();

                        let label_width = egui_extras::Size::exact(120.0);

                        // The WebDAV fs table.
                        label_value_table(ui, 4, row_height, |rows| {
                            // Url.
                            build_row(
                                rows,
                                label_width,
                                "Url:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.add(NPathEditor::<Abs, Dir>::new(
                                        &entry_key.to_string(),
                                        &mut webdav_fs.url,
                                        &mut self.npath_editor_buffer,
                                    ));
                                },
                            );

                            // User.
                            build_row(
                                rows,
                                label_width,
                                "User:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut webdav_fs.user)
                                            .desired_width(f32::INFINITY),
                                    );
                                },
                            );

                            // Password ID.
                            build_row(
                                rows,
                                label_width,
                                "Password ID:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut webdav_fs.password_id)
                                            .desired_width(f32::INFINITY),
                                    );
                                },
                            );

                            // Timeout.
                            build_row(
                                rows,
                                label_width,
                                "Timeout (secs):",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.add(egui::DragValue::new(&mut webdav_fs.timeout_secs));
                                },
                            );
                        });
                    }
                    ConfigEntryMut::Backup(backup) => {
                        ui.heading(format!("Backup: {}", entry_key.name));

                        // Separator.
                        ui.separator();

                        let label_width = egui_extras::Size::exact(120.0);

                        // The Backup table.
                        label_value_table(ui, 6, row_height, |rows| {
                            // Source fs.
                            build_row(
                                rows,
                                label_width,
                                "Source:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    egui::ComboBox::from_id_salt("SourceFS")
                                        .selected_text(backup.src_fs.to_string())
                                        .show_ui(ui, |ui| {
                                            for fs_entry in &fs_entries {
                                                ui.selectable_value(
                                                    &mut backup.src_fs,
                                                    fs_entry.to_string(),
                                                    fs_entry.to_string(),
                                                );
                                            }
                                        });
                                },
                            );

                            // Destination fs.
                            build_row(
                                rows,
                                label_width,
                                "Destination:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    egui::ComboBox::from_id_salt("DestFS")
                                        .selected_text(backup.dest_fs.to_string())
                                        .show_ui(ui, |ui| {
                                            for fs_entry in &fs_entries {
                                                ui.selectable_value(
                                                    &mut backup.dest_fs,
                                                    fs_entry.name.to_string(),
                                                    fs_entry.name.to_string(),
                                                );
                                            }
                                        });
                                },
                            );

                            // Source dir.
                            build_row(
                                rows,
                                label_width,
                                "Source dir:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.add(NPathEditor::<Rel, Dir>::new(
                                        &(entry_key.to_string() + ".src"),
                                        &mut backup.src_dir,
                                        &mut self.npath_editor_buffer,
                                    ));
                                },
                            );

                            // Destination dir.
                            build_row(
                                rows,
                                label_width,
                                "Destination dir:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.add(NPathEditor::<Rel, Dir>::new(
                                        &(entry_key.to_string() + ".dest"),
                                        &mut backup.dest_dir,
                                        &mut self.npath_editor_buffer,
                                    ));
                                },
                            );

                            // Compression.
                            build_row(
                                rows,
                                label_width,
                                "Compression",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.checkbox(&mut backup.compression, "");
                                },
                            );

                            // Encryption.
                            build_row(
                                rows,
                                label_width,
                                "Encryption",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.checkbox(&mut backup.encrypt, "");
                                },
                            );
                        });
                    }
                    ConfigEntryMut::Restore(_restore) => {}
                }
            }
        }
    }
}

/// Impl of `AppView` for `ConfigView`.
impl AppView for ConfigView {
    /// Returns the name of the view.
    fn name(&self) -> &str {
        "Config"
    }

    /// Renders the view for egui.
    fn ui(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();

        // Horizontal layout (config entry list, entry content).
        ui.horizontal(|ui| {
            // Vertical layout (heading, list).
            ui.vertical(|ui| {
                ui.set_width(400.0);
                ui.set_height(height);

                // Entry list heading.
                ui.heading("Entries");

                // Separator.
                ui.separator();

                // Entry list.
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        if let Some(config) = self.cuba.read().unwrap().config() {
                            for entry_key in config.list_entry_keys() {
                                if ui
                                    .selectable_label(false, format!("{}", entry_key))
                                    .clicked()
                                {
                                    self.selected_config_entry_key = Some(entry_key);
                                }
                            }
                        }
                    });
            });

            // Separator.
            ui.separator();

            // Vertical layout (config entry content).
            ui.vertical(|ui| {
                ui.set_height(height);

                self.render_entry_editor(ui);
            });
        });
    }
}
