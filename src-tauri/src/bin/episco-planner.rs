// EpiscoPlanner: the planning tool (services, the shared parish library,
// scripts and bulletins), without output windows or live controls.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    episco_presenter_lib::run_planner()
}
