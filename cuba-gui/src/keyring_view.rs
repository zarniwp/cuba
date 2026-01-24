#![allow(dead_code)]

use core::f32;
use std::sync::{Arc, RwLock};

use cuba_lib::core::cuba::Cuba;
use secrecy::{ExposeSecret, SecretString};

use crate::{
    AppView, ViewId,
    egui_widgets::{build_row, label_value_table},
    password_ids::PasswordIDs,
};

/// Defines a `KeyringView`.
pub struct KeyringView {
    cuba: Arc<RwLock<Cuba>>,
    password_ids: Arc<PasswordIDs>,
    show_password: bool,
    password_id: String,
    password: String,
}

/// Methods of `KeyringView`.
impl KeyringView {
    /// Creates a new `KeyringView`.
    pub fn new(cuba: Arc<RwLock<Cuba>>, password_ids: Arc<PasswordIDs>) -> Self {
        Self {
            cuba,
            password_ids,
            password_id: String::new(),
            password: String::new(),
            show_password: false,
        }
    }
}

/// Impl of `AppView` for `KeyringView`.
impl AppView for KeyringView {
    /// Returns the name of the view.
    fn name(&self) -> &str {
        "Keyring"
    }

    /// Returns the view id.
    fn view_id(&self) -> ViewId {
        ViewId::Keyring
    }

    /// Renders the view for egui.
    fn ui(&mut self, ui: &mut egui::Ui) {
        let height = ui.available_height();

        // Horizontal layout (entry list, entry content).
        ui.horizontal(|ui| {
            // Vertical layout (heading, entry list).
            ui.vertical(|ui| {
                // Set width/height.
                ui.set_width(400.0);
                ui.set_height(height);

                // Entry list heading.
                ui.heading("Entries");

                // Separator.
                ui.separator();

                // Entry list.
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .id_salt("Entries")
                    .show(ui, |ui| {
                        for id in self.password_ids.get() {
                            let selected = self.password_id == id;

                            if ui.selectable_label(selected, id.clone()).clicked() {
                                // Set password id.
                                self.password_id = id;

                                // Retrieve password.
                                match self.cuba.read().unwrap().get_password(&self.password_id) {
                                    Some(password) => {
                                        self.password = password.expose_secret().to_string()
                                    }
                                    None => self.password.clear(),
                                }
                            }
                        }
                    });
            });

            // Separator.
            ui.separator();

            // Vertical layout (entry content).
            ui.vertical(|ui| {
                ui.set_height(height);

                // Horizontal layout (heading, buttons).
                ui.horizontal(|ui| {
                    // The heading.
                    ui.heading(self.password_id.to_string());

                    // Add stretch.
                    ui.add_space(ui.available_width() - 190.0);

                    // The save entry button.
                    if ui.button("Save Entry").clicked() {
                        self.cuba.read().unwrap().set_password(
                            &self.password_id,
                            &SecretString::from(self.password.clone()),
                        );
                        self.password_ids.update();
                    }

                    // The delete entry button.
                    if ui.button("Delete Entry").clicked() {
                        self.cuba.read().unwrap().delete_password(&self.password_id);
                        self.password_ids.update();
                        self.password_id.clear();
                        self.password.clear();
                    }
                });

                // Separator.
                ui.separator();

                // Define widths.
                let row_height: f32 = 25.0;
                let label_width = egui_extras::Size::exact(100.0);
                let value_width = egui_extras::Size::exact(400.0);

                // The keyring entry table.
                label_value_table(ui, 2, row_height, |rows| {
                    // The password id row.
                    build_row(rows, label_width, "Password ID:", value_width, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.password_id)
                                .desired_width(f32::INFINITY),
                        );
                    });

                    // The password row.
                    rows.strip(|strip| {
                        strip
                            .size(label_width)
                            .size(value_width)
                            .size(egui_extras::Size::remainder())
                            .horizontal(|mut row| {
                                row.cell(|ui| {
                                    // Password label.
                                    ui.label("Password:");
                                });
                                row.cell(|ui| {
                                    // The password edit.
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.password)
                                            .password(!self.show_password)
                                            .desired_width(f32::INFINITY),
                                    );
                                });
                                row.cell(|ui| {
                                    // The password show/hide button.
                                    if ui.button("👁").clicked() {
                                        self.show_password = !self.show_password;
                                    }
                                });
                            });
                    });
                });
            });
        });
    }
}
