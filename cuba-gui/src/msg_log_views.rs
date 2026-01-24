use std::{
    error::Error,
    sync::{Arc, RwLock},
};

use cuba_lib::shared::{
    message::{Info, Message},
    msg_dispatcher::MsgDispatcher,
    msg_receiver::{MsgHandler, MsgReceiver, trace_error},
    npath::{Rel, UNPath},
};

use crate::{AppView, UpdateHandler, ViewId};

/// Defines a `MsgLogView`.
pub struct MsgLogView {
    log_level: MsgLogLevel,
    _msg_receiver: MsgReceiver,
    msg_log: Arc<MsgLog>,
}

/// Methods of `MsgLogView`.
impl MsgLogView {
    /// Creates a new `MsgLogView`.
    pub fn new(
        egui_context: egui::Context,
        log_level: MsgLogLevel,
        msg_dispatcher: Arc<MsgDispatcher<Arc<dyn Message>>>,
    ) -> Self {
        let msg_log = Arc::new(MsgLog::new(
            UpdateHandler::new(egui_context.clone()),
            log_level.clone(),
        ));
        let mut msg_receiver = MsgReceiver::new(msg_dispatcher.subscribe(), msg_log.clone());

        msg_receiver.start();

        Self {
            log_level,
            _msg_receiver: msg_receiver,
            msg_log,
        }
    }
}

/// Impl of `AppView` for `MsgLogView`.
impl AppView for MsgLogView {
    /// Returns the name of the view.
    fn name(&self) -> &str {
        match self.log_level {
            MsgLogLevel::Info => "Infos",
            MsgLogLevel::Warning => "Warnings",
            MsgLogLevel::Error => "Errors",
        }
    }

    /// Returns the view id.
    fn view_id(&self) -> ViewId {
        match self.log_level {
            MsgLogLevel::Info => ViewId::InfoLog,
            MsgLogLevel::Warning => ViewId::WarningLog,
            MsgLogLevel::Error => ViewId::ErrorLog,
        }
    }

    /// Renders the view for egui.
    fn ui(&mut self, ui: &mut egui::Ui) {
        let text_color = match self.log_level {
            MsgLogLevel::Info => egui::Color32::LIGHT_GREEN,
            MsgLogLevel::Warning => egui::Color32::LIGHT_YELLOW,
            MsgLogLevel::Error => egui::Color32::LIGHT_RED,
        };

        // Add a copy and clear button at the top of the tab.
        ui.horizontal(|ui| {
            // Align buttons to the right.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Clear button.
                if ui.small_button("🗑 Clear").clicked() {
                    self.msg_log.messages.write().unwrap().clear();
                    self.msg_log.update_handler.update();
                }

                // Copy button.
                if ui.small_button("📋 Copy").clicked() {
                    let text = self.msg_log.snapshot();
                    ui.ctx().copy_text(text);
                }
            });
        });

        let mut messages = self.msg_log.snapshot();
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
        let desired_rows = (ui.available_height() / row_height) as usize;

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                egui::TextEdit::multiline(&mut messages)
                    .font(egui::TextStyle::Monospace)
                    .text_color(text_color)
                    .desired_width(ui.available_width())
                    .desired_rows(desired_rows)
                    .interactive(false)
                    .show(ui);
            });
    }
}

/// Defines a `MsgLogLevel`.
#[derive(Debug, Clone, PartialEq)]
pub enum MsgLogLevel {
    Info,
    Warning,
    Error,
}

/// Defines a `MsgLog`.
pub struct MsgLog {
    log_level: MsgLogLevel,
    messages: RwLock<String>,
    update_handler: UpdateHandler,
}

/// Methods of `MsgLog`.
impl MsgLog {
    /// Creates a new `MsgLog`.
    pub fn new(update_handler: UpdateHandler, log_level: MsgLogLevel) -> Self {
        MsgLog {
            log_level,
            messages: RwLock::new(String::new()),
            update_handler,
        }
    }

    /// Returns a snapshot of the `MsgLog`.
    pub fn snapshot(&self) -> String {
        self.messages.read().unwrap().clone()
    }
}

/// Impl of `MsgHandler` for `MsgLogTab`.
impl MsgHandler for MsgLog {
    /// Handles a `TaskInfo::Transferred` message.
    fn task_transferred(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        if self.log_level == MsgLogLevel::Info {
            self.messages
                .write()
                .unwrap()
                .push_str(&format!("{:?} : {}\n", rel_path, info));
            self.update_handler.update();
        }
    }

    /// Handles a `TaskInfo::Verified` message.
    fn task_verified(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        info: &(dyn Info + Send + Sync),
    ) {
        if self.log_level == MsgLogLevel::Info {
            self.messages
                .write()
                .unwrap()
                .push_str(&format!("{:?} : {}\n", rel_path, info));
            self.update_handler.update();
        }
    }

    /// Handles a `TaskMessage` with error.
    fn task_error(
        &self,
        _thread_number: usize,
        rel_path: &UNPath<Rel>,
        error: &(dyn Error + Send + Sync),
    ) {
        if self.log_level == MsgLogLevel::Error {
            self.messages.write().unwrap().push_str(&format!(
                "{:?} : {}\n",
                rel_path,
                trace_error(error)
            ));
            self.update_handler.update();
        }
    }

    /// Handles a `CleanInfo::Removed` message.
    fn clean_removed(&self, rel_path: &UNPath<Rel>, info: &(dyn Info + Send + Sync)) {
        if self.log_level == MsgLogLevel::Info {
            self.messages
                .write()
                .unwrap()
                .push_str(&format!("{:?} : {}\n", rel_path, info));
            self.update_handler.update();
        }
    }

    /// Handles a `CleanMessage` with error.
    fn clean_error(&self, rel_path: &UNPath<Rel>, error: &(dyn Error + Send + Sync)) {
        if self.log_level == MsgLogLevel::Error {
            self.messages.write().unwrap().push_str(&format!(
                "{:?} : {}\n",
                rel_path,
                trace_error(error)
            ));
            self.update_handler.update();
        }
    }

    /// Handles a `InfoMessage`.
    fn info(&self, info: &(dyn Info + Send + Sync)) {
        if self.log_level == MsgLogLevel::Info {
            self.messages
                .write()
                .unwrap()
                .push_str(&format!("{}\n", info));
            self.update_handler.update();
        }
    }

    /// Handles a `WarnMessage`.
    fn warn(&self, warning: &(dyn Info + Send + Sync)) {
        if self.log_level == MsgLogLevel::Warning {
            self.messages
                .write()
                .unwrap()
                .push_str(&format!("{}\n", warning));
            self.update_handler.update();
        }
    }

    /// Handles a `ErrorMessage`.
    fn error(&self, error: &(dyn Error + Send + Sync)) {
        if self.log_level == MsgLogLevel::Error {
            self.messages
                .write()
                .unwrap()
                .push_str(&format!("{}\n", trace_error(error)));
            self.update_handler.update();
        }
    }
}
