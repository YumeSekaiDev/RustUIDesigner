//! Visual Basic-style form designer / small Rust project workspace.
//!
//! Targeted egui/eframe version: **0.37.0**.

mod app;
mod canvas;
mod code_editor;
mod codegen;
mod explorer;
mod git_panel;
mod inspector;
mod model;
mod project;
mod pty_term;
mod terminal_panel;
mod theme;
mod toolbox;

use app::FormDesignerApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("Form Designer"),
        ..Default::default()
    };

    eframe::run_native(
        "Form Designer",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(FormDesignerApp::default()))
        }),
    )
}
