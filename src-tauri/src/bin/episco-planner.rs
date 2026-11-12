// EpiscoPlanner: the planning tool (services, the shared parish library,
// scripts and bulletins), without output windows or live controls.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The planner's own config (name, app identity, window title).
    episco_presenter_lib::run_planner(tauri::generate_context!("planner/tauri.conf.json"))
}
