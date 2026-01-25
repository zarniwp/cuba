#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod backup_view;
mod config_view;
mod egui_widgets;
mod keyring_view;
mod msg_log_views;
mod password_ids;
mod restore_view;
mod task_progress;
mod util;

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, RwLock},
};

use crate::{
    backup_view::BackupView,
    config_view::ConfigView,
    keyring_view::KeyringView,
    msg_log_views::{MsgLogLevel, MsgLogView},
    password_ids::PasswordIDs,
    restore_view::RestoreView,
};
use crossbeam_channel::{Sender, unbounded};
use cuba_lib::{
    core::cuba::Cuba,
    send_error,
    shared::{config::load_config_from_file, message::Message, msg_dispatcher::MsgDispatcher},
};
use eframe::egui;
use egui::{FontData, FontDefinitions, FontFamily};
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use serde::{Deserialize, Serialize};

/// The layout file.
const LAYOUT_FILE: &str = "cuba-gui-layout.json";

/// Sets up the fonts for egui.
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Load monospace font.
    fonts.font_data.insert(
        "jetbrains_mono".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );

    // Use it for monospace text.
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains_mono".to_owned());

    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    // Set egui options.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(1200.0, 800.0))
            .with_min_inner_size(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Cuba GUI",
        options,
        Box::new(|creation_ctx| Ok(Box::new(CubaGui::new(creation_ctx)))),
    )
}

/// Defines a `UpdateHandler`.
///
/// This is for view models to signal that data has been updated.
#[derive(Clone)]
struct UpdateHandler {
    egui_context: egui::Context,
}

/// Methods of `UpdateHandler`.
impl UpdateHandler {
    /// Creates a new `UpdateHandler`.
    pub fn new(egui_context: egui::Context) -> Self {
        Self { egui_context }
    }

    /// Signal that data has been updated.
    pub fn update(&self) {
        self.egui_context.request_repaint();
    }
}

// Defines the different views in the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum ViewId {
    Backup,
    Restore,
    Config,
    Keyring,
    InfoLog,
    WarningLog,
    ErrorLog,
}

/// Defines the trait `AppView`.
trait AppView {
    /// Returns the name of the view.
    fn name(&self) -> &str;

    /// Renders the view for egui.
    fn ui(&mut self, ui: &mut egui::Ui);

    // Returns the view id.
    fn view_id(&self) -> ViewId;
}

/// Defines a `AppViewer`.
struct AppViewer<'a> {
    app_views: &'a HashMap<ViewId, Arc<RwLock<dyn AppView>>>,
}

/// Impl of `TabViewer` for `AppViewer`.
impl<'a> TabViewer for AppViewer<'a> {
    type Tab = ViewId;

    /// Returns the title of the `AppView` as a `egui::WidgetText`.
    fn title(&mut self, view_id: &mut ViewId) -> egui::WidgetText {
        egui::RichText::new(self.app_views[view_id].read().unwrap().name())
            .font(egui::FontId::proportional(16.0))
            .into()
    }

    /// Renders each view.
    fn ui(&mut self, ui: &mut egui::Ui, view_id: &mut ViewId) {
        self.app_views[view_id].write().unwrap().ui(ui);
    }
}

/// Defines a `CubaGui`.
struct CubaGui {
    sender: Sender<Arc<dyn Message>>,
    cuba: Arc<RwLock<Cuba>>,
    _msg_dispatcher: Arc<MsgDispatcher<Arc<dyn Message>>>,
    app_views: HashMap<ViewId, Arc<RwLock<dyn AppView>>>,
    dock_state: DockState<ViewId>,
    post_init_done: bool,
    show_about: bool,
}

/// Methods of `CubaGui`.
impl CubaGui {
    /// Creates a new `CubaGui`.
    fn new(creation_ctx: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&creation_ctx.egui_ctx);

        // Set fonts.
        let mut style = (*creation_ctx.egui_ctx.style()).clone();

        style.text_styles = [
            (
                egui::TextStyle::Heading,
                egui::FontId::new(20.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(16.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(14.0, egui::FontFamily::Monospace),
            ),
        ]
        .into();

        // Set style.
        creation_ctx.egui_ctx.set_style(style);

        // Sender and receiver for messages between the GUI and the Cuba instance.
        let (sender, receiver) = unbounded::<Arc<dyn Message>>();

        // The message dispatcher.
        let mut msg_dispatcher = MsgDispatcher::new(receiver.clone());
        msg_dispatcher.start();
        let arc_msg_dispatcher = Arc::new(msg_dispatcher);

        // The Cuba instance.
        let cuba = Arc::new(RwLock::new(Cuba::new(sender.clone())));

        // The password ids.
        let password_ids = Arc::new(PasswordIDs::new(cuba.clone()));
        password_ids.update();

        // The backup view.
        let backup_view = Arc::new(RwLock::new(BackupView::new(
            creation_ctx.egui_ctx.clone(),
            sender.clone(),
            cuba.clone(),
            arc_msg_dispatcher.clone(),
        )));

        // The restore view.
        let restore_view = Arc::new(RwLock::new(RestoreView::new(creation_ctx.egui_ctx.clone())));

        // The config view.
        let config_view = Arc::new(RwLock::new(ConfigView::new(
            cuba.clone(),
            password_ids.clone(),
        )));

        // The keyring view.
        let keyring_view = Arc::new(RwLock::new(KeyringView::new(
            cuba.clone(),
            password_ids.clone(),
        )));

        // The infos view.
        let infos_view = Arc::new(RwLock::new(MsgLogView::new(
            creation_ctx.egui_ctx.clone(),
            MsgLogLevel::Info,
            arc_msg_dispatcher.clone(),
        )));

        // The warnings view.
        let warnings_view = Arc::new(RwLock::new(MsgLogView::new(
            creation_ctx.egui_ctx.clone(),
            MsgLogLevel::Warning,
            arc_msg_dispatcher.clone(),
        )));

        // The errors view.
        let errors_view = Arc::new(RwLock::new(MsgLogView::new(
            creation_ctx.egui_ctx.clone(),
            MsgLogLevel::Error,
            arc_msg_dispatcher.clone(),
        )));

        let mut app_views = HashMap::<ViewId, Arc<RwLock<dyn AppView>>>::new();
        app_views.insert(ViewId::Backup, backup_view);
        app_views.insert(ViewId::Restore, restore_view);
        app_views.insert(ViewId::Config, config_view);
        app_views.insert(ViewId::Keyring, keyring_view);
        app_views.insert(ViewId::InfoLog, infos_view);
        app_views.insert(ViewId::WarningLog, warnings_view);
        app_views.insert(ViewId::ErrorLog, errors_view);

        let mut dock_state: DockState<ViewId> = DockState::new(Vec::new());

        CubaGui::set_default_layout(&mut dock_state);

        Self {
            sender: sender.clone(),
            cuba: cuba.clone(),
            _msg_dispatcher: arc_msg_dispatcher,
            app_views,
            dock_state,
            post_init_done: false,
            show_about: false,
        }
    }

    // Adds a view button.
    fn add_view_button(&mut self, app_view: &Arc<RwLock<dyn AppView>>, ui: &mut egui::Ui) {
        let view_location = self.dock_state.find_tab_from(|existing_view_id: &ViewId| {
            *existing_view_id == app_view.read().unwrap().view_id()
        });

        if ui.button(app_view.read().unwrap().name()).clicked() {
            if let Some(view_location) = view_location {
                self.dock_state.set_active_tab(view_location);
            } else {
                self.dock_state
                    .push_to_focused_leaf(app_view.read().unwrap().view_id());
            }
        }
    }

    // Set the active tab.
    fn set_active_view(&mut self, view_id: &ViewId) {
        let view_location = self
            .dock_state
            .find_tab_from(|existing_view_id: &ViewId| *existing_view_id == *view_id);

        if let Some(view_location) = view_location {
            self.dock_state.set_active_tab(view_location);
        }
    }

    /// Reset the default layout of the GUI.
    pub fn reset_default_layout(&mut self) {
        self.dock_state = egui_dock::DockState::new(Vec::new());
        CubaGui::set_default_layout(&mut self.dock_state);
    }

    /// Set the default layout of the GUI.
    fn set_default_layout(dock_state: &mut egui_dock::DockState<ViewId>) {
        let surface = dock_state.main_surface_mut();

        surface.push_to_first_leaf(ViewId::Backup);
        surface.push_to_first_leaf(ViewId::Restore);
        surface.push_to_first_leaf(ViewId::Config);
        surface.push_to_first_leaf(ViewId::Keyring);

        let bottom = surface.split_below(NodeIndex::root(), 0.6, vec![ViewId::InfoLog]);

        surface.split_right(bottom[1], 0.5, vec![ViewId::WarningLog, ViewId::ErrorLog]);
    }

    /// Post initialization.
    fn post_init(&mut self) {
        if let Some(config) = load_config_from_file(self.sender.clone(), "cuba.toml") {
            self.cuba.write().unwrap().set_config(config);
        }

        // Set active view.
        self.set_active_view(&ViewId::Backup);

        // Load layout if it exists.
        if Path::new(LAYOUT_FILE).exists() {
            self.load_layout();
        }
    }

    /// Save the current layout state to a file.
    pub fn save_layout(&self) {
        let serialized = match serde_json::to_string(&self.dock_state) {
            Ok(serialized) => serialized,
            Err(err) => {
                send_error!(self.sender, err);
                return;
            }
        };

        if let Err(err) = std::fs::write(LAYOUT_FILE, serialized) {
            send_error!(self.sender, err);
        }
    }

    /// Load the layout state from a file.
    pub fn load_layout(&mut self) {
        let serialized = match std::fs::read_to_string(LAYOUT_FILE) {
            Ok(serialized) => serialized,
            Err(err) => {
                send_error!(self.sender, err);
                return;
            }
        };

        match serde_json::from_str(&serialized) {
            Ok(dock_state) => self.dock_state = dock_state,
            Err(err) => send_error!(self.sender, err),
        }
    }
}

/// Impl of `eframe::App` for `CubaGui`.
impl eframe::App for CubaGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.post_init_done {
            self.post_init();
            self.post_init_done = true;
        }

        egui::TopBottomPanel::top("Menu").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Views", |ui| {
                    for app_view in self.app_views.clone().values() {
                        self.add_view_button(app_view, ui);
                    }
                });

                ui.menu_button("Layout", |ui| {
                    if ui.button("Reset Layout").clicked() {
                        self.reset_default_layout();
                    }

                    if ui.button("Save Layout").clicked() {
                        self.save_layout();
                    }

                    if ui.button("Load Layout").clicked() {
                        self.load_layout();
                    }
                });

                if ui.button("About").clicked() {
                    self.show_about = true;
                };
            });
        });

        // The about dialog.
        if self.show_about {
            egui::Window::new("About")
                .collapsible(false)
                .resizable(false)
                .default_size([600.0, 200.0])
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.heading(env!("CARGO_PKG_NAME"));
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.separator();
                    ui.label("Cuba is a lightweight and flexible backup tool for your local data. It allows you to back up files to WebDAV cloud or network drives while keeping them in their original form by default. Optional compression and encryption ensure your backups are efficient and secure, and because standard formats are used, your files can also be accessed or restored with public tools if needed.");
                    ui.separator();
                    ui.label(format!("© 2026 {}", env!("CARGO_PKG_AUTHORS")));
                    ui.separator();
                    ui.label(format!("License: {}", env!("CARGO_PKG_LICENSE")));
                    ui.separator();
                    egui::Grid::new("Hyperlinks").show(ui, |ui| {
                        ui.label("Homepage:");
                        ui.hyperlink(env!("CARGO_PKG_HOMEPAGE"));
                        ui.end_row();

                        ui.label("Repository:");
                        ui.hyperlink(env!("CARGO_PKG_REPOSITORY"));
                        ui.end_row();

                        ui.label("Documentation:");
                        ui.hyperlink(format!("https://docs.rs/{}/{}",
                            env!("CARGO_PKG_NAME"),
                            env!("CARGO_PKG_VERSION")));
                        ui.end_row();
                    });

                    ui.separator();
                    ui.label("This project bundles the JetBrains Mono font. JetBrains Mono is licensed under the SIL Open Font License 1.1. Copyright © 2020 JetBrains s.r.o.");

                    ui.add_space(12.0);

                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 0.0),
                        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| {
                            if ui.button("OK").clicked() {
                                self.show_about = false;
                            }
                        },
                    );
                });
        }

        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(
                ctx,
                &mut AppViewer {
                    app_views: &self.app_views,
                },
            );
    }
}
