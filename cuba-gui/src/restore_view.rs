#![allow(dead_code)]

use crate::{AppView, ViewId};

/// Defines a `RestoreView`.
pub struct RestoreView {
    egui_context: egui::Context,
}

/// Methods of `RestoreView`.
impl RestoreView {
    /// Creates a new `RestoreView`.
    pub fn new(egui_context: egui::Context) -> Self {
        Self { egui_context }
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
    fn ui(&mut self, _ui: &mut egui::Ui) {}
}
