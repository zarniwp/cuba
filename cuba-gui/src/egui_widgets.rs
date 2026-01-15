#![allow(dead_code)]

use cuba_lib::shared::npath::NPath;
use egui::{
    Color32, Vec2,
    ahash::{HashMap, HashMapExt},
};

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

/// Defines a `NPathEditorBuffer`.
pub struct NPathEditorBuffer {
    buffer: HashMap<String, String>,
}

/// Methods of `NPathEditorBuffer`.
impl NPathEditorBuffer {
    /// Creates a new `NPathEditorBuffer`.
    pub fn new() -> Self {
        Self {
            buffer: HashMap::new(),
        }
    }

    /// Clears the buffer.
    pub fn clear(&mut self) {
        self.buffer.clear()
    }
}

/// Impl of `Default` for `NPathEditorBuffer`.
impl Default for NPathEditorBuffer {
    fn default() -> Self {
        NPathEditorBuffer::new()
    }
}

/// Defines a `NPathEditor`.
pub struct NPathEditor<'a, NpathK, NpathT> {
    key: &'a str,
    path: &'a mut NPath<NpathK, NpathT>,
    npath_buffer: &'a mut NPathEditorBuffer,
    desired_width: f32,
}

/// Methods of `NPathEditor`.
impl<'a, NpathK, NpathT> NPathEditor<'a, NpathK, NpathT> {
    /// Creates a new `NPathEditor`.
    pub fn new(
        key: &'a str,
        path: &'a mut NPath<NpathK, NpathT>,
        npath_buffer: &'a mut NPathEditorBuffer,
    ) -> Self {
        Self {
            key,
            path,
            npath_buffer,
            desired_width: f32::INFINITY,
        }
    }

    /// Sets the desired width of the editor.
    pub fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = desired_width;
        self
    }
}

/// Impl `egui::Widget` for `NPathEditor`.
impl<NpathK, NpathT> egui::Widget for NPathEditor<'_, NpathK, NpathT>
where
    for<'s> NPath<NpathK, NpathT>: TryFrom<&'s str>,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let buffer: &mut String = if let Some(buffer) = self.npath_buffer.buffer.get_mut(self.key) {
            if buffer.is_empty() {
                *buffer = self.path.to_string();
            }

            buffer
        } else {
            self.npath_buffer
                .buffer
                .insert(self.key.to_string(), self.path.to_string());
            self.npath_buffer.buffer.get_mut(self.key).unwrap()
        };

        let valid;

        match NPath::<NpathK, NpathT>::try_from(buffer.as_str()) {
            Ok(new_path) => {
                *self.path = new_path;
                valid = true;
            }
            Err(_) => {
                valid = false;
            }
        }

        let text_edit = if valid {
            egui::TextEdit::singleline(buffer).desired_width(self.desired_width)
        } else {
            egui::TextEdit::singleline(buffer)
                .background_color(Color32::DARK_RED)
                .desired_width(self.desired_width)
        };

        ui.add(text_edit)
    }
}

/// Builds a table with labels and values.
pub fn label_value_table(
    ui: &mut egui::Ui,
    rows: usize,
    row_height: f32,
    mut add_rows: impl FnMut(&mut egui_extras::Strip),
) {
    egui_extras::StripBuilder::new(ui)
        .sizes(egui_extras::Size::exact(row_height), rows)
        .vertical(|mut rows_strip| {
            add_rows(&mut rows_strip);
        });
}

/// Builds a row with a label and a value.
pub fn build_row(
    rows: &mut egui_extras::Strip,
    label_width: egui_extras::Size,
    label: &str,
    value_width: egui_extras::Size,
    add: impl FnOnce(&mut egui::Ui),
) {
    rows.strip(|strip| {
        strip
            .size(label_width)
            .size(value_width)
            .horizontal(|mut row| {
                row.cell(|ui| {
                    ui.label(label);
                });
                row.cell(|ui| {
                    add(ui);
                });
            });
    });
}
