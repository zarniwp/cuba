#![allow(dead_code)]

use cuba_lib::shared::npath::{Abs, Dir, NPath};
use egui::{Color32, Shape, Stroke, Vec2};

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

/// Draws a progress spinner
pub fn progress_spinner_widget(
    ui: &mut egui::Ui,
    progress_state: &ProgressState,
    color_spinning: Color32,
    color_invalid: Color32,
    size: f32,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let painter = ui.painter();
    let center = rect.center();
    let radius = size * 0.4;

    let segments = 32; // smoothness.
    let mut points = Vec::with_capacity(segments + 1);

    if !progress_state.valid {
        // Draw a full gray circle.
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let angle = t * std::f32::consts::TAU;
            let dir = egui::vec2(angle.cos(), angle.sin());
            points.push(center + dir * radius);
        }

        painter.add(Shape::line(points, Stroke::new(2.0, color_invalid)));
    } else {
        // Draw progress spinner.
        let t = (progress_state.progress % progress_state.duration) as f32
            / progress_state.duration as f32;
        let angle = t * std::f32::consts::TAU;

        let start_angle = angle - std::f32::consts::FRAC_PI_2;
        let sweep = std::f32::consts::PI * 1.5; // 270°
        let end_angle = start_angle + sweep;

        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let a = start_angle + (end_angle - start_angle) * t;
            let dir = egui::vec2(a.cos(), a.sin());
            points.push(center + dir * radius);
        }

        painter.add(Shape::line(points, Stroke::new(2.0, color_spinning)));
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

/// Draws a npath editor
pub fn npath_editor_widget(
    ui: &mut egui::Ui,
    label: &str,
    key: &str,
    path: &mut NPath<Abs, Dir>,
    npath_state: &mut NPathEditorState,
) {
    ui.horizontal(|ui| {
        ui.label(label);

        // Key must be the same or refresh.
        if npath_state.key != key {
            npath_state.key = key.to_owned();
            npath_state.refresh = true;
        }

        // Refresh editor from source.
        if npath_state.refresh {
            npath_state.buffer = path.to_string();
            npath_state.refresh = false;
        }

        let valid;

        match NPath::<Abs, Dir>::try_from(npath_state.buffer.as_str()) {
            Ok(new_path) => {
                *path = new_path;
                valid = true;
            }
            Err(_) => {
                valid = false;
            }
        }

        let text_edit = if valid {
            egui::TextEdit::singleline(&mut npath_state.buffer)
        } else {
            egui::TextEdit::singleline(&mut npath_state.buffer).background_color(Color32::DARK_RED)
        };

        text_edit.show(ui);
    });
}
