//! EpiscoPresenter desktop application.
//!
//! Startup:
//! 1. Load the per-machine [`NodeConfig`] (`node.json` in the app config dir,
//!    created with defaults on first run; see `docs/architecture.md`).
//! 2. Start the [`Engine`]: sync server on master / standalone nodes,
//!    sync client on slave nodes.
//! 3. Open this node's output windows on their monitors.

mod autosave;
mod commands;
mod engine;
mod library;
mod media;
mod services;
mod templates;
mod windows;

use std::path::{Path, PathBuf};

use episco_sync::{NodeConfig, NodeRole};
use tauri::Manager;
use tracing_subscriber::EnvFilter;

use crate::engine::Engine;

const CONFIG_FILE: &str = "node.json";

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config = load_config(app.handle());
            templates::init(app.handle());
            tracing::info!("starting as {:?} node `{}`", config.role, config.node_name);

            let handle = app.handle().clone();
            let engine = tauri::async_runtime::block_on(Engine::start(handle, &config));
            app.manage(engine);

            if let Some(control) = app.get_webview_window("control") {
                let _ = control.set_title(&format!("EpiscoPresenter — {}", role_title(config.role)));
            }
            windows::open_outputs(app.handle(), &config)?;
            app.manage(config);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_node_config,
            commands::get_snapshot,
            commands::send_command,
            commands::vocabulary,
            commands::parse_ewm,
            commands::format_ewm,
            commands::confirm,
            commands::pick_image,
            commands::open_service,
            commands::new_service,
            commands::load_example_service,
            commands::add_items,
            commands::new_item,
            commands::update_cue_document,
            commands::set_cue_overrides,
            commands::set_announcement_mode,
            commands::set_cue_look,
            commands::set_default_look,
            commands::set_service_details,
            commands::list_backgrounds,
            commands::import_backgrounds,
            commands::pick_background,
            commands::move_cue,
            commands::remove_cue,
            commands::save_cue,
            commands::save_service,
            library::library_list,
            library::library_preview,
            library::library_add,
            library::library_save_cue,
            library::library_delete,
            library::library_folder,
            library::parse_propers,
            library::add_propers,
            library::psalm_arrange,
            library::lectionary_propers,
            library::reading_set_text,
            library::reading_url,
            library::open_passage,
            library::hymn_set_text,
            library::library_import_hymns,
            media::media_list,
            media::media_import,
            media::media_delete,
            media::media_rename,
            media::media_folder,
            media::media_add,
            media::blank_add,
            templates::layouts_get,
            templates::layouts_set,
            templates::layouts_reset,
            services::liturgical_info,
            services::services_list,
            services::service_create,
            services::services_schedule,
            services::service_delete,
            services::service_keep,
            services::templates_list,
            services::template_save,
            services::template_update,
            services::template_delete,
            services::template_open,
        ])
        .run(tauri::generate_context!())
        .expect("error while running EpiscoPresenter");
}

fn role_title(role: NodeRole) -> &'static str {
    match role {
        NodeRole::Master => "Sanctuary (Master)",
        NodeRole::Slave => "Live Stream (Slave)",
        NodeRole::Standalone => "Standalone",
    }
}

/// Reads the node config.
///
/// Resolution order: `$EPISCO_CONFIG` path, else `<app config dir>/node.json`
/// (written with defaults if absent). `$EPISCO_ROLE` and `$EPISCO_MASTER_URL`
/// override the file, which makes it easy to run a master and a slave side by
/// side on one development machine.
fn load_config(app: &tauri::AppHandle) -> NodeConfig {
    let path = std::env::var_os("EPISCO_CONFIG")
        .map(PathBuf::from)
        .or_else(|| app.path().app_config_dir().ok().map(|d| d.join(CONFIG_FILE)));

    let mut config = match &path {
        Some(p) if p.exists() => read_config(p),
        Some(p) => {
            let config = NodeConfig::default();
            write_default(p, &config);
            config
        }
        None => NodeConfig::default(),
    };

    if let Ok(role) = std::env::var("EPISCO_ROLE") {
        match serde_json::from_value(serde_json::Value::String(role.to_ascii_lowercase())) {
            Ok(r) => config.role = r,
            Err(_) => tracing::warn!("ignoring EPISCO_ROLE=`{role}` (expected master, slave or standalone)"),
        }
    }
    if let Ok(url) = std::env::var("EPISCO_MASTER_URL") {
        config.master_url = url;
    }
    if let Err(e) = config.validate() {
        tracing::error!("invalid node config: {e}; falling back to standalone");
        config.role = NodeRole::Standalone;
    }
    config
}

fn read_config(path: &Path) -> NodeConfig {
    match std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str::<NodeConfig>(&s).map_err(|e| e.to_string()))
    {
        Ok(c) => {
            tracing::info!("node config: {}", path.display());
            c
        }
        Err(e) => {
            tracing::error!("could not read {}: {e}; using defaults", path.display());
            NodeConfig::default()
        }
    }
}

fn write_default(path: &Path, config: &NodeConfig) {
    let result = path
        .parent()
        .map(std::fs::create_dir_all)
        .transpose()
        .and_then(|_| std::fs::write(path, serde_json::to_string_pretty(config).unwrap_or_default()));
    match result {
        Ok(_) => tracing::info!("wrote default node config to {}", path.display()),
        Err(e) => tracing::warn!("could not write default config to {}: {e}", path.display()),
    }
}
