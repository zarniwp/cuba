#![allow(dead_code)]

use std::sync::{Arc, RwLock};

use cuba_lib::{
    core::cuba::Cuba,
    shared::config::{ConfigEntryKey, ConfigEntryMut, ConfigEntryType},
};

use crate::{
    AppView,
    egui_widgets::{NPathEditorState, npath_editor_widget},
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
            match entry {
                ConfigEntryMut::LocalFS(local_fs) => {
                    ui.heading("LocalFS");

                    // Separator.
                    ui.separator();

                    let editor_label = "Dir:";
                    npath_editor_widget(
                        ui,
                        editor_label,
                        &ConfigView::npath_editor_key(entry_key, editor_label),
                        &mut local_fs.dir,
                        &mut self.npath_editor_state,
                    );
                }

                ConfigEntryMut::WebDAVFS(webdav_fs) => {
                    ui.heading("WebDAV");

                    // Separator.
                    ui.separator();

                    let editor_label = "Url:";
                    npath_editor_widget(
                        ui,
                        editor_label,
                        &ConfigView::npath_editor_key(entry_key, editor_label),
                        &mut webdav_fs.url,
                        &mut self.npath_editor_state,
                    );

                    ui.horizontal(|ui| {
                        ui.label("User");
                        ui.text_edit_singleline(&mut webdav_fs.user);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Password ID");
                        ui.text_edit_singleline(&mut webdav_fs.password_id);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Timeout (secs)");
                        ui.add(egui::DragValue::new(&mut webdav_fs.timeout_secs));
                    });
                }

                ConfigEntryMut::Backup(backup) => {
                    ui.heading("Backup");

                    // Separator.
                    ui.separator();

                    ui.checkbox(&mut backup.encrypt, "Encrypt");
                    ui.checkbox(&mut backup.compression, "Compression");

                    if backup.encrypt {
                        ui.text_edit_singleline(backup.password_id.get_or_insert_default());
                    }
                }
                _ => {}
            }
        }
    }

    fn npath_editor_key(entry: &ConfigEntryKey, label: &str) -> String {
        format!("{:?}.{}.{}", entry.entry_type, entry.name, label)
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
