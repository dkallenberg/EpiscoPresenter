//! Tauri commands invoked from the web front-end.

use std::path::PathBuf;

use episco_core::ewm::{self, Parsed};
use episco_core::role::RoleStyle;
use episco_core::service::Loaded;
use episco_core::{Command, LiturgyDocument, Role, Service};
use episco_sync::NodeConfig;
use serde::Serialize;
use tauri::State;

use crate::engine::{Engine, Snapshot};

#[tauri::command]
pub fn get_node_config(config: State<'_, NodeConfig>) -> NodeConfig {
    config.inner().clone()
}

#[tauri::command]
pub fn get_snapshot(engine: State<'_, Engine>) -> Snapshot {
    engine.snapshot()
}

#[tauri::command]
pub fn send_command(engine: State<'_, Engine>, command: Command) -> Result<(), String> {
    engine.send(command)
}

/// Parses `.ewm` source from the editor.
#[tauri::command]
pub fn parse_ewm(source: String) -> Result<Parsed, String> {
    ewm::parse(&source).map_err(|e| e.to_string())
}

/// Serialises an edited document back to `.ewm` source.
#[tauri::command]
pub fn format_ewm(document: LiturgyDocument) -> String {
    ewm::to_string(&document)
}

#[tauri::command]
pub fn open_ewm(path: PathBuf) -> Result<Parsed, String> {
    ewm::load_file(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_ewm(path: PathBuf, document: LiturgyDocument) -> Result<(), String> {
    std::fs::write(&path, ewm::to_string(&document)).map_err(|e| format!("could not save {}: {e}", path.display()))
}

/// Loads a TOML service plan and makes it the live service.
#[tauri::command]
pub fn load_service_plan(engine: State<'_, Engine>, path: PathBuf) -> Result<Vec<String>, String> {
    let Loaded { service, warnings } = Service::load_plan(&path).map_err(|e| e.to_string())?;
    engine.load_service(Some(service))?;
    Ok(warnings)
}

/// Presents a single `.ewm` source as a one-cue service (quick preview).
#[tauri::command]
pub fn present_ewm(engine: State<'_, Engine>, source: String) -> Result<Vec<String>, String> {
    let Parsed { document, warnings } = ewm::parse(&source).map_err(|e| e.to_string())?;
    engine.load_service(Some(Service::from_document(document, None)))?;
    Ok(warnings.iter().map(ToString::to_string).collect())
}

#[derive(Serialize)]
pub struct RoleInfo {
    role: Role,
    congregational: bool,
    style: RoleStyle,
}

/// The editor's role selector entries with their formatting.
#[tauri::command]
pub fn role_styles() -> Vec<RoleInfo> {
    Role::STANDARD
        .into_iter()
        .map(|role| RoleInfo { congregational: role.is_congregational(), style: role.style(), role })
        .collect()
}
