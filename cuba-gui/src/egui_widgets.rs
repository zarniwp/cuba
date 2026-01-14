#![allow(dead_code)]

use cuba_lib::shared::npath::{Abs, Dir, NPath};
use egui::{Color32, Vec2};

/// Defines a `ProgressState`
#[derive(Clone, Copy)]
pub struct ProgressState {
    progress: u64,
    duration: u64,
    valid: bool,
}

/// Methods of `ProgressState`.
impl ProgressState {
    /// Creates a new `ProgressState`.
    pub fn new(duration: u64) -> Self {
        Self {
            progress: 0,
            duration,
            valid: false,
        }
    }

    /// Sets the duration.
    pub fn set_duration(&mut self, duration: u64) {
        self.duration = duration;
    }

    /// Clears the `ProgressState`.
    pub fn clear(&mut self) {
        self.valid = false;
        self.progress = 0;
    }

    /// Advances the progress by one.
    pub fn advance_one(&mut self) {
        self.valid = true;
        self.progress = (self.progress + 1) % (self.duration + 1);
    }

    /// Advances the progress by ticks.
    pub fn advance_ticks(&mut self, ticks: u64) {
        self.valid = true;
        self.progress = (self.progress + ticks) % (self.duration + 1);
    }

    /// Returns the normalized progress.
    pub fn normalized(&mut self) -> f32 {
        self.progress as f32 / self.duration as f32
    }
}

/// Impl of `Default` for `ProgressState`.
impl Default for ProgressState {
    fn default() -> Self {
        ProgressState::new(64)
    }
}

/// Defines a `ProgressSpinner`.
pub struct ProgressSpinner<'a> {
    state: &'a ProgressState,
    size: f32,
    color_spinning: Color32,
    color_invalid: Color32,
}

/// Methods of `ProgressSpinner`.
impl<'a> ProgressSpinner<'a> {
    /// Creates a new `ProgressSpinner`.
    pub fn new(state: &'a ProgressState) -> Self {
        Self {
            state,
            size: 24.0,
            color_spinning: Color32::LIGHT_BLUE,
            color_invalid: Color32::GRAY,
        }
    }

    /// Sets the size of the spinner.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets the color of the spinner.
    pub fn spinning_color(mut self, color: Color32) -> Self {
        self.color_spinning = color;
        self
    }

    /// Sets the color of the spinner when invalid.
    pub fn invalid_color(mut self, color: Color32) -> Self {
        self.color_invalid = color;
        self
    }
}

/// Impl `egui::Widget` for `ProgressSpinner`.
impl egui::Widget for ProgressSpinner<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(self.size), egui::Sense::hover());

        let painter = ui.painter();
        let center = rect.center();
        let radius = self.size * 0.4;
        let segments = 32;

        let mut points = Vec::with_capacity(segments + 1);

        if !self.state.valid {
            for i in 0..=segments {
                let t = i as f32 / segments as f32;
                let angle = t * std::f32::consts::TAU;
                let dir = egui::vec2(angle.cos(), angle.sin());
                points.push(center + dir * radius);
            }

            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(2.0, self.color_invalid),
            ));
        } else {
            let t = (self.state.progress % self.state.duration) as f32 / self.state.duration as f32;

            let angle = t * std::f32::consts::TAU;
            let start = angle - std::f32::consts::FRAC_PI_2;
            let sweep = std::f32::consts::PI * 1.5;
            let end = start + sweep;

            for i in 0..=segments {
                let t = i as f32 / segments as f32;
                let a = start + (end - start) * t;
                let dir = egui::vec2(a.cos(), a.sin());
                points.push(center + dir * radius);
            }

            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(2.0, self.color_spinning),
            ));

            ui.ctx().request_repaint(); // keep animation alive
        }

        response
    }
}

/// Defines a `NPathEditorState`.
pub struct NPathEditorState {
    pub key: String,
    pub refresh: bool,
    pub buffer: String,
}

/// Methods of `NPathEditorState`.
impl NPathEditorState {
    pub fn new() -> Self {
        Self {
            key: String::new(),
            refresh: true,
            buffer: String::new(),
        }
    }
}

/// Impl of `Default` for `NPathEditorState`.
impl Default for NPathEditorState {
    fn default() -> Self {
        NPathEditorState::new()
    }
}

/// Defines a `NPathEditor`.
pub struct NPathEditor<'a> {
    key: &'a str,
    path: &'a mut NPath<Abs, Dir>,
    npath_state: &'a mut NPathEditorState,
    desired_width: f32,
}

/// Methods of `NPathEditor`.
impl<'a> NPathEditor<'a> {
    /// Creates a new `NPathEditor`.
    pub fn new(
        key: &'a str,
        path: &'a mut NPath<Abs, Dir>,
        npath_state: &'a mut NPathEditorState,
    ) -> Self {
        Self {
            key,
            path,
            npath_state,
            desired_width: f32::INFINITY,
        }
    }

    /// Sets the desired width of the editor.
    pub fn desired_with(mut self, desired_width: f32) -> Self {
        self.desired_width = desired_width;
        self
    }
}

/// Impl `egui::Widget` for `NPathEditor`.
impl egui::Widget for NPathEditor<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        // Key must be the same or refresh.
        if self.npath_state.key != self.key {
            self.npath_state.key = self.key.to_owned();
            self.npath_state.refresh = true;
        }

        // Refresh editor from source.
        if self.npath_state.refresh {
            self.npath_state.buffer = self.path.to_string();
            self.npath_state.refresh = false;
        }

        let valid;

        match NPath::<Abs, Dir>::try_from(self.npath_state.buffer.as_str()) {
            Ok(new_path) => {
                *self.path = new_path;
                valid = true;
            }
            Err(_) => {
                valid = false;
            }
        }

        let text_edit = if valid {
            egui::TextEdit::singleline(&mut self.npath_state.buffer)
                .desired_width(self.desired_width)
        } else {
            egui::TextEdit::singleline(&mut self.npath_state.buffer)
                .background_color(Color32::DARK_RED)
                .desired_width(self.desired_width)
        };

        ui.add(text_edit)
    }
}
