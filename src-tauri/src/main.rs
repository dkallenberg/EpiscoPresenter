// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // EpiscoPresenter, or EpiscoPlanner when built with the planner's
    // bundle config (its identity decides; see `start`).
    episco_presenter_lib::start(tauri::generate_context!())
}
