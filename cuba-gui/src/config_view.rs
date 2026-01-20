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
    egui_widgets::{GlobListWidget, NPathEditor, NPathEditorBuffer, build_row, label_value_table},
    password_ids::PasswordIDs,
};

/// Defines a `ConfigView`.
pub struct ConfigView {
    cuba: Arc<RwLock<Cuba>>,
    password_ids: Arc<PasswordIDs>,
    selected_config_entry_key: Option<ConfigEntryKey>,
    npath_editor_buffer: NPathEditorBuffer,
}

/// Methods of `ConfigView`.
impl ConfigView {
    /// Creates a new `ConfigView`.
    pub fn new(cuba: Arc<RwLock<Cuba>>, password_ids: Arc<PasswordIDs>) -> Self {
        Self {
            cuba,
            password_ids,
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
                // Set row height.
                let row_height: f32 = 25.0;

                match entry {
                    ConfigEntryMut::LocalFS(local_fs) => {
                        // The heading.
                        ui.heading(format!("LocalFS: {}", entry_key.name));

                        // Separator.
                        ui.separator();

                        // Set label width.
                        let label_width = egui_extras::Size::exact(40.0);

                        // The local fs table.
                        label_value_table(ui, 1, row_height, |rows| {
                            // The dir row.
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
                        // The heading.
                        ui.heading(format!("WebDAV: {}", entry_key.name));

                        // Separator.
                        ui.separator();

                        // The label width.
                        let label_width = egui_extras::Size::exact(120.0);

                        // The WebDAV fs table.
                        label_value_table(ui, 4, row_height, |rows| {
                            // The url row.
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

                            // The user row.
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

                            // The password id row.
                            build_row(
                                rows,
                                label_width,
                                "Password ID:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    egui::ComboBox::from_id_salt("PasswordID")
                                        .selected_text(webdav_fs.password_id.to_string())
                                        .show_ui(ui, |ui| {
                                            for password_id in &self.password_ids.get() {
                                                ui.selectable_value(
                                                    &mut webdav_fs.password_id,
                                                    password_id.to_string(),
                                                    password_id,
                                                );
                                            }
                                        });
                                },
                            );

                            // The timeout row.
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
                        // The heading.
                        ui.heading(format!("Backup: {}", entry_key.name));

                        // Separator.
                        ui.separator();

                        // The label width.
                        let label_width = egui_extras::Size::exact(120.0);

                        // The backup table.
                        label_value_table(ui, 9, row_height, |rows| {
                            // The source fs row.
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

                            // The destination fs row.
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

                            // The source dir row.
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

                            // The destination dir row.
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

                            // The compression row.
                            build_row(
                                rows,
                                label_width,
                                "Compression",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.checkbox(&mut backup.compression, "");
                                },
                            );

                            // The encryption row.
                            build_row(
                                rows,
                                label_width,
                                "Encryption",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.checkbox(&mut backup.encrypt, "");
                                },
                            );

                            // The password id row.
                            if backup.encrypt {
                                let password_id =
                                    backup.password_id.get_or_insert_with(String::new);

                                build_row(
                                    rows,
                                    label_width,
                                    "Password ID:",
                                    egui_extras::Size::remainder(),
                                    |ui| {
                                        egui::ComboBox::from_id_salt("PasswordID")
                                            .selected_text(password_id.as_str())
                                            .show_ui(ui, |ui| {
                                                for id in &self.password_ids.get() {
                                                    ui.selectable_value(
                                                        password_id,
                                                        id.clone(),
                                                        id,
                                                    );
                                                }
                                            });
                                    },
                                );
                            } else {
                                backup.password_id = None;
                            }

                            // The include row.
                            build_row(
                                rows,
                                label_width,
                                "Include:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.add(GlobListWidget::new(&mut backup.include));
                                },
                            );

                            // The exclude row.
                            build_row(
                                rows,
                                label_width,
                                "Exclude:",
                                egui_extras::Size::remainder(),
                                |ui| {
                                    ui.add(GlobListWidget::new(&mut backup.exclude));
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
                                let selected =
                                    self.selected_config_entry_key == Some(entry_key.clone());

                                if ui
                                    .selectable_label(selected, format!("{}", entry_key))
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
            egui::ScrollArea::both().show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_height(height);

                    self.render_entry_editor(ui);
                });
            });
        });
    }
}
