#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all, rust_2018_idioms)]

mod app;

fn main() {
    let app = app::App::default();
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(eframe::egui::Vec2 { x: 250.0, y: 350.0 });
    eframe::run_native(Box::new(app), native_options);
}
