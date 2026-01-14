#![allow(dead_code)]

use std::sync::{Arc, RwLock};

use cuba_lib::{
    core::cuba::Cuba,
    shared::config::{ConfigEntryKey, ConfigEntryMut, ConfigEntryType},
};

use crate::{
    AppView,
    egui_widgets::{NPathEditor, NPathEditorState},
};

/// Defines a `ConfigView`.
pub struct ConfigView {
    egui_context: egui::Context,
    cuba: Arc<RwLock<Cuba>>,
    selected_config_entry_key: Option<ConfigEntryKey>,
    npath_editor_state: NPathEditorState,
}

/// Methods of `ConfigView`.
impl ConfigView {
    /// Creates a new `ConfigView`.
    pub fn new(egui_context: egui::Context, cuba: Arc<RwLock<Cuba>>) -> Self {
        Self {
            egui_context,
            cuba,
            selected_config_entry_key: None,
            npath_editor_state: NPathEditorState::new(),
        }
    }
}

/// Methods of `ConfigView`.
impl ConfigView {
    /// Renders the config entry_editor.
    fn render_entry_editor(&mut self, ui: &mut egui::Ui) {
        if let Some(config) = self.cuba.write().unwrap().config_mut()
            && let Some(entry_key) = &self.selected_config_entry_key
            && let Some(entry) = config.get_entry_mut(entry_key)
        {
            let row_height = ui.spacing().interact_size.y;

            match entry {
                ConfigEntryMut::LocalFS(local_fs) => {
                    ui.heading(format!("LocalFS: {}", entry_key.name));

                    // Separator.
                    ui.separator();

                    let label_width = 40.0;

                    // The Local table.
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::exact(label_width))
                        .size(egui_extras::Size::remainder())
                        .horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.label("Dir:");
                            });
                            strip.cell(|ui| {
                                ui.add_sized(
                                    [ui.available_width(), row_height],
                                    NPathEditor::new(
                                        &entry_key.to_string(),
                                        &mut local_fs.dir,
                                        &mut self.npath_editor_state,
                                    ),
                                );
                            });
                        });
                }
                ConfigEntryMut::WebDAVFS(webdav_fs) => {
                    ui.heading(format!("WebDAV: {}", entry_key.name));

                    // Separator.
                    ui.separator();

                    let label_width = 100.0;

                    // The WebDAV table.
                    egui_extras::StripBuilder::new(ui)
                        .size(egui_extras::Size::exact(row_height))
                        .size(egui_extras::Size::exact(row_height))
                        .size(egui_extras::Size::exact(row_height))
                        .size(egui_extras::Size::exact(row_height))
                        .vertical(|mut rows| {
                            // URL
                            rows.strip(|strip| {
                                strip
                                    .size(egui_extras::Size::exact(label_width))
                                    .size(egui_extras::Size::remainder())
                                    .horizontal(|mut row| {
                                        row.cell(|ui| {
                                            ui.label("Url:");
                                        });
                                        row.cell(|ui| {
                                            ui.add_sized(
                                                [ui.available_width(), row_height],
                                                NPathEditor::new(
                                                    &entry_key.to_string(),
                                                    &mut webdav_fs.url,
                                                    &mut self.npath_editor_state,
                                                ),
                                            );
                                        });
                                    });
                            });

                            // User
                            rows.strip(|strip| {
                                strip
                                    .size(egui_extras::Size::exact(label_width))
                                    .size(egui_extras::Size::remainder())
                                    .horizontal(|mut row| {
                                        row.cell(|ui| {
                                            ui.label("User:");
                                        });
                                        row.cell(|ui| {
                                            ui.add_sized(
                                                [ui.available_width(), row_height],
                                                egui::TextEdit::singleline(&mut webdav_fs.user),
                                            );
                                        });
                                    });
                            });

                            // Password ID
                            rows.strip(|strip| {
                                strip
                                    .size(egui_extras::Size::exact(label_width))
                                    .size(egui_extras::Size::remainder())
                                    .horizontal(|mut row| {
                                        row.cell(|ui| {
                                            ui.label("User:");
                                        });
                                        row.cell(|ui| {
                                            ui.add_sized(
                                                [ui.available_width(), row_height],
                                                egui::TextEdit::singleline(
                                                    &mut webdav_fs.password_id,
                                                ),
                                            );
                                        });
                                    });
                            });

                            // Timeout
                            rows.strip(|strip| {
                                strip
                                    .size(egui_extras::Size::exact(label_width))
                                    .size(egui_extras::Size::remainder())
                                    .horizontal(|mut row| {
                                        row.cell(|ui| {
                                            ui.label("User:");
                                        });
                                        row.cell(|ui| {
                                            ui.add_sized(
                                                [100.0, row_height],
                                                egui::DragValue::new(&mut webdav_fs.timeout_secs),
                                            );
                                        });
                                    });
                            });
                        });
                }
                ConfigEntryMut::Backup(_backup) => {}
                ConfigEntryMut::Restore(_restore) => {}
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
                            let mut entry_type_str;

                            for key in config.list_entry_keys() {
                                match key.entry_type {
                                    ConfigEntryType::LocalFS => {
                                        entry_type_str = "Local".to_string()
                                    }
                                    ConfigEntryType::WebDAVFS => {
                                        entry_type_str = "WebDAV".to_string()
                                    }
                                    ConfigEntryType::Backup => {
                                        entry_type_str = "Backup".to_string()
                                    }
                                    ConfigEntryType::Restore => {
                                        entry_type_str = "Restore".to_string()
                                    }
                                }

                                if ui
                                    .selectable_label(
                                        false,
                                        format!("{}.{}", entry_type_str, key.name),
                                    )
                                    .clicked()
                                {
                                    self.selected_config_entry_key = Some(key);
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
