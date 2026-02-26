/// Show the about dialog.
pub fn show_about(ctx: &egui::Context, show_about: &mut bool, icon_texture: &egui::TextureHandle) {
    egui::Window::new("About")
        .collapsible(false)
        .resizable(false)
        .default_size([600.0, 200.0])
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(icon_texture)
                        .max_width(50.0)
                        .corner_radius(10),
                );
                ui.vertical(|ui| {
                    ui.heading(env!("CARGO_PKG_NAME"));
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                });
            });
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
                        *show_about = false;
                    }
                },
            );
        });
}
