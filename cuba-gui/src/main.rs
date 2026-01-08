#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod backup_view;
mod egui_widgets;
mod msg_log_views;
mod restore_view;
mod task_progress;
mod util;

use std::sync::{Arc, RwLock};

use crate::{
    backup_view::BackupView,
    msg_log_views::{MsgLogLevel, MsgLogView},
    restore_view::RestoreView,
};
use crossbeam_channel::{Sender, unbounded};
use cuba_lib::{
    core::cuba::Cuba,
    shared::{config::load_config_from_file, message::Message, msg_dispatcher::MsgDispatcher},
};
use eframe::egui;
use egui::{FontData, FontDefinitions, FontFamily};
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};

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

/// Defines the trait `AppView`.
trait AppView {
    /// Returns the name of the view.
    fn name(&self) -> &str;

    /// Renders the view for egui.
    fn ui(&mut self, ui: &mut egui::Ui);
}

/// Defines a `AppViewer`.
struct AppViewer;

/// Impl of `TabViewer` for `AppViewer`.
impl TabViewer for AppViewer {
    type Tab = Arc<RwLock<dyn AppView>>;

    /// Returns the title of the `AppView` as a `egui::WidgetText`.
    fn title(&mut self, app_view: &mut Arc<RwLock<dyn AppView>>) -> egui::WidgetText {
        egui::WidgetText::from(app_view.read().unwrap().name())
    }

    /// Renders each view.
    fn ui(&mut self, ui: &mut egui::Ui, app_view: &mut Arc<RwLock<dyn AppView>>) {
        app_view.write().unwrap().ui(ui);
    }
}

/// Defines a `CubaGui`.
struct CubaGui {
    sender: Sender<Arc<dyn Message>>,
    cuba: Arc<RwLock<Cuba>>,
    _msg_dispatcher: Arc<MsgDispatcher<Arc<dyn Message>>>,
    app_views: Vec<Arc<RwLock<dyn AppView>>>,
    dock_state: DockState<Arc<RwLock<dyn AppView>>>,
    post_init_done: bool,
}

/// Methods of `CubaGui`.
impl CubaGui {
    /// Creates a new `CubaGui`.
    fn new(creation_ctx: &eframe::CreationContext<'_>) -> Self {
        setup_fonts(&creation_ctx.egui_ctx);

        let (sender, receiver) = unbounded::<Arc<dyn Message>>();

        let mut msg_dispatcher = MsgDispatcher::new(receiver.clone());

        msg_dispatcher.start();

        let arc_msg_dispatcher = Arc::new(msg_dispatcher);

        let cuba = Arc::new(RwLock::new(Cuba::new(sender.clone())));

        let backup_view = Arc::new(RwLock::new(BackupView::new(
            creation_ctx.egui_ctx.clone(),
            sender.clone(),
            cuba.clone(),
            arc_msg_dispatcher.clone(),
        )));
        let restore_view = Arc::new(RwLock::new(RestoreView::new(creation_ctx.egui_ctx.clone())));
        let infos_view = Arc::new(RwLock::new(MsgLogView::new(
            creation_ctx.egui_ctx.clone(),
            MsgLogLevel::Info,
            arc_msg_dispatcher.clone(),
        )));
        let warnings_view = Arc::new(RwLock::new(MsgLogView::new(
            creation_ctx.egui_ctx.clone(),
            MsgLogLevel::Warning,
            arc_msg_dispatcher.clone(),
        )));
        let errors_view = Arc::new(RwLock::new(MsgLogView::new(
            creation_ctx.egui_ctx.clone(),
            MsgLogLevel::Error,
            arc_msg_dispatcher.clone(),
        )));

        let app_views: Vec<Arc<RwLock<dyn AppView>>> = vec![
            backup_view,
            restore_view,
            infos_view,
            warnings_view,
            errors_view,
        ];
        let mut dock_state: DockState<Arc<RwLock<dyn AppView>>> = DockState::new(Vec::new());

        CubaGui::set_default_layout(&app_views, &mut dock_state);

        Self {
            sender: sender.clone(),
            cuba: cuba.clone(),
            _msg_dispatcher: arc_msg_dispatcher,
            app_views,
            dock_state,
            post_init_done: false,
        }
    }

    // Adds a view button.
    fn add_view_button(&mut self, app_view: &Arc<RwLock<dyn AppView>>, ui: &mut egui::Ui) {
        let view_location =
            self.dock_state
                .find_tab_from(|existing_view: &Arc<RwLock<dyn AppView>>| {
                    Arc::ptr_eq(existing_view, app_view)
                });

        if ui.button(app_view.read().unwrap().name()).clicked() {
            if let Some(view_location) = view_location {
                self.dock_state.set_active_tab(view_location);
            } else {
                self.dock_state.push_to_focused_leaf(app_view.clone());
            }
        }
    }

    /// Reset the default layout of the GUI.
    pub fn reset_default_layout(&mut self) {
        self.dock_state = egui_dock::DockState::new(Vec::new());
        CubaGui::set_default_layout(&self.app_views, &mut self.dock_state);
    }

    /// Set the default layout of the GUI.
    fn set_default_layout(
        app_views: &[Arc<RwLock<dyn AppView>>],
        dock_state: &mut egui_dock::DockState<Arc<RwLock<dyn AppView>>>,
    ) {
        let surface = dock_state.main_surface_mut();

        surface.push_to_first_leaf(app_views[0].clone());
        surface.push_to_first_leaf(app_views[1].clone());

        let bottom = surface.split_below(NodeIndex::root(), 0.6, vec![app_views[2].clone()]);

        surface.split_right(
            bottom[1],
            0.5,
            vec![app_views[4].clone(), app_views[3].clone()],
        );
    }

    /// Post initialization.
    fn post_init(&mut self) {
        if let Some(config) = load_config_from_file(self.sender.clone(), "cuba.toml") {
            self.cuba.write().unwrap().set_config(config);
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
                    for app_view in self.app_views.clone() {
                        self.add_view_button(&app_view, ui);
                    }
                });

                ui.menu_button("Layout", |ui| {
                    if ui.button("Reset Layout").clicked() {
                        self.reset_default_layout();
                    }
                });
            });
        });

        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut AppViewer);
    }
}
