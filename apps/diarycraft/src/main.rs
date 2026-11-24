//! DiaryCraft desktop app.
#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use diarycraft_ui_egui::DiaryCraftApp;

/// Fixed install identity: also the Wayland app id.
const APP_ID: &str = "it.mauromazzieri.diarycraft";

/// The window icon, 64x64 RGBA, drawn by `tools/icons.py`.
const ICON: &[u8] = include_bytes!("../../../assets/app-icon/window-64.rgba");

fn main() -> eframe::Result {
    let open = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DiaryCraft")
            .with_app_id(APP_ID)
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([620.0, 420.0])
            .with_icon(egui::IconData { rgba: ICON.to_vec(), width: 64, height: 64 }),
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native("DiaryCraft", options, Box::new(|cc| Ok(Box::new(DiaryCraftApp::new(cc, open)))))
}
