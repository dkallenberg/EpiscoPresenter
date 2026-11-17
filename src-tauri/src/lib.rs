//! EpiscoPresenter desktop application.
//!
//! Startup:
//! 1. Load the per-machine [`NodeConfig`] (`node.json` in the app config dir,
//!    created with defaults on first run; see `docs/architecture.md`).
//! 2. Start the [`Engine`]: sync server on master / standalone nodes,
//!    sync client on slave nodes.
//! 3. Output windows open from the control window (Outputs), or at start-up
//!    with `displays.open_at_start`.

mod autosave;
mod bulletin;
mod cache;
mod commands;
mod deck;
mod drive;
mod engine;
mod handoff;
mod inuse;
mod library;
mod made;
mod media;
mod occasions;
mod scripture;
mod services;
mod storage;
mod sync;
mod templates;
mod undo;
mod updater;
mod windows;

use std::path::{Path, PathBuf};

use episco_sync::{NodeConfig, NodeRole};
use tauri::Manager;
use tracing_subscriber::EnvFilter;

use crate::engine::Engine;

const CONFIG_FILE: &str = "node.json";

/// Which program is running: the presentation app, or EpiscoPlanner (the
/// planning tool: no output windows, live controls or network sync).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppMode {
    Presenter,
    Planner,
}

#[tauri::command]
fn app_mode(mode: tauri::State<'_, AppMode>) -> AppMode {
    *mode
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();
}

/// EpiscoPlanner: plan services, keep the parish library in a shared
/// folder, and (later) make scripts and bulletins.
///
/// `context` is `tauri::generate_context!("planner/tauri.conf.json")`, made in
/// the planner's own binary: each app's context must live in its own binary
/// (on macOS it embeds an Info.plist under a fixed symbol name).
pub fn run_planner(context: tauri::Context<tauri::Wry>) {
    init_tracing();
    builder()
        // Tell the page it is the planner before it draws anything, so the
        // presenter's layout never shows first.
        .append_invoke_initialization_script("window.__EPISCO_MODE__ = 'planner';")
        .setup(|app| {
            storage::init(app.handle());
            templates::init(app.handle());
            bulletin::init(app.handle());
            tracing::info!("starting EpiscoPlanner");
            // The window title comes from the config the app was built with,
            // which may be the presenter's (see `start`).
            if let Some(w) = app.get_webview_window("control") {
                let _ = w.set_title("EpiscoPlanner");
            }
            let engine = tauri::async_runtime::block_on(Engine::start_local(app.handle().clone()));
            app.manage(engine);
            app.manage(AppMode::Planner);
            // Commands that read the node config get a plain one (no outputs are opened).
            app.manage(NodeConfig::default());
            sync::spawn(app.handle().clone());
            drive::spawn(app.handle().clone());
            undo::init(app.handle());
            inuse::spawn(app.handle().clone());
            updater::spawn(app.handle().clone(), true);
            Ok(())
        })
        .build(context)
        .expect("error while building EpiscoPlanner")
        .run(|_, event| {
            // The open service is no longer open here.
            if let tauri::RunEvent::Exit = event {
                inuse::close();
            }
        });
}

/// EpiscoPlanner's app identity (`src-tauri/planner/bundle.json`).
pub const PLANNER_IDENTIFIER: &str = "org.episcopresenter.planner";

/// Starts the app the program was built as: EpiscoPlanner when its config
/// carries the planner's identity, else EpiscoPresenter.
///
/// `cargo tauri build --config src-tauri/planner/bundle.json` does not
/// install the `episco-planner` program: it builds the main program with
/// the planner's identity merged into its config and renames it. So the
/// main program has to tell from its config which app it is.
pub fn start(context: tauri::Context<tauri::Wry>) {
    if context.config().identifier == PLANNER_IDENTIFIER {
        run_planner(context)
    } else {
        run(context)
    }
}

/// The presentation app. `context` is `tauri::generate_context!()`, made in
/// the app's binary (see [`run_planner`]).
pub fn run(context: tauri::Context<tauri::Wry>) {
    init_tracing();
    builder()
        .setup(|app| {
            let config = load_config(app.handle());
            storage::set_presenter();
            storage::init(app.handle());
            app.manage(AppMode::Presenter);
            templates::init(app.handle());
            bulletin::init(app.handle());
            tracing::info!("starting as {:?} node `{}`", config.role, config.node_name);

            let handle = app.handle().clone();
            let engine = tauri::async_runtime::block_on(Engine::start(handle, &config));
            app.manage(engine);

            if let Some(control) = app.get_webview_window("control") {
                let _ = control.set_title(&format!("EpiscoPresenter — {}", role_title(config.role)));
                // Closing the main window closes the outputs (and so the app).
                let handle = app.handle().clone();
                control.on_window_event(move |e| {
                    if matches!(e, tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed) {
                        windows::close_outputs(&handle);
                    }
                });
            }
            windows::open_at_start(app.handle(), &config)?;
            app.manage(config);
            // Copy services marked Ready in the parish library to this computer.
            sync::spawn(app.handle().clone());
            drive::spawn(app.handle().clone());
            undo::init(app.handle());
            handoff::spawn(app.handle().clone());
            updater::spawn(app.handle().clone(), false);
            Ok(())
        })
        .run(context)
        .expect("error while running EpiscoPresenter");
}

/// What both programs share: plugins and commands.
fn builder() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default().plugin(tauri_plugin_dialog::init()).plugin(updater::plugin()).invoke_handler(
        tauri::generate_handler![
            updater::update_ready,
            updater::update_install,
            app_mode,
            bulletin::parish_get,
            bulletin::parish_set,
            bulletin::parish_pick_logo,
            bulletin::bulletin_html,
            bulletin::bulletin_export,
            bulletin::bulletin_styles,
            bulletin::set_cue_bulletin,
            bulletin::set_cue_script,
            bulletin::set_service_paper,
            bulletin::set_service_time,
            bulletin::bulletin_edit,
            bulletin::bulletin_set_title,
            scripture::bible_status,
            scripture::bible_pick_file,
            scripture::bible_import,
            scripture::bible_remove,
            scripture::reading_fill,
            occasions::occasions_list,
            occasions::set_service_bishop,
            occasions::set_service_deacon,
            occasions::service_add_baptismal,
            handoff::service_set_ready,
            handoff::handoff_status,
            handoff::handoff_refresh,
            storage::storage_get,
            storage::storage_set,
            storage::storage_pick_folder,
            commands::get_node_config,
            commands::node_info,
            commands::node_settings_save,
            windows::outputs_status,
            deck::deck_fetch,
            deck::deck_local,
            deck::deck_set_slides,
            windows::output_show,
            windows::output_close,
            windows::output_set_fullscreen,
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
            commands::set_prelude,
            commands::set_postlude,
            services::service_set_prayers,
            services::service_set_eucharistic_prayer,
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
            library::library_fill_cue,
            library::anthem_add,
            library::anthem_set,
            sync::library_sync,
            sync::library_sync_status,
            drive::drive_status,
            drive::drive_import_client,
            drive::drive_sign_in,
            drive::drive_sign_out,
            drive::drive_folders,
            drive::drive_use_folder,
            inuse::in_use_status,
            inuse::in_use_take_over,
            undo::undo,
            undo::redo,
            undo::undo_status,
            library::library_delete,
            library::library_merge,
            library::library_set_uses,
            library::library_copy,
            library::cue_make_version,
            made::slides_add,
            made::slides_pick_pdf,
            made::web_video_add,
            library::library_set_default,
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
            library::library_create_from_text,
            library::gospel_set_music,
            media::media_list,
            media::media_import,
            media::media_delete,
            media::music_list,
            media::music_import,
            media::media_rename,
            media::media_set_tags,
            media::media_folder,
            media::media_add,
            media::blank_add,
            templates::layouts_get,
            templates::layouts_set,
            templates::layouts_reset,
            templates::layouts_defaults,
            templates::layouts_make_default,
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
        ],
    )
}

fn role_title(role: NodeRole) -> &'static str {
    match role {
        NodeRole::Master => "Sanctuary (Master)",
        NodeRole::Slave => "Live Stream (Slave)",
        NodeRole::Standalone => "Standalone",
    }
}

/// Where this computer's node config is (`$EPISCO_CONFIG`, else
/// `<app config dir>/node.json`).
pub(crate) fn config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    std::env::var_os("EPISCO_CONFIG")
        .map(PathBuf::from)
        .or_else(|| app.path().app_config_dir().ok().map(|d| d.join(CONFIG_FILE)))
}

/// Reads the node config.
///
/// Resolution order: `$EPISCO_CONFIG` path, else `<app config dir>/node.json`
/// (written with defaults if absent). `$EPISCO_ROLE` and `$EPISCO_MASTER_URL`
/// override the file, which makes it easy to run a master and a slave side by
/// side on one development machine.
fn load_config(app: &tauri::AppHandle) -> NodeConfig {
    let path = config_path(app);

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
