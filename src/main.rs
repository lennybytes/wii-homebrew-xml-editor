#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod meta;

use std::path::PathBuf;

use app::MetaEditorApp;

fn main() -> eframe::Result {
    let initial_path = std::env::args_os().nth(1).map(PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 720.0])
            .with_min_inner_size([900.0, 560.0])
            .with_app_id("wii-meta-editor"),
        ..Default::default()
    };

    eframe::run_native(
        "Wii Meta XML Editor",
        native_options,
        Box::new(move |cc| Ok(Box::new(MetaEditorApp::new(cc, initial_path.clone())))),
    )
}
