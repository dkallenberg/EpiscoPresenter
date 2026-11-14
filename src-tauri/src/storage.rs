//! Where services, templates, library items, media and slide layouts live.
//!
//! By default that is the app's own data folder on this computer. With a
//! **parish library** folder chosen (for example a folder in Google Drive,
//! shared by the rector, administrator, musician and technician), everything
//! is kept there instead, so every planner (and the presentation computer)
//! works from the same files:
//!
//! ```text
//! Parish Library/
//!   services/  templates/  library/  media/  backgrounds/  layouts.json
//! ```
//!
//! The choice, and the user's name (shown to the others as who changed
//! what), are kept per computer in `library.json` in the app's config folder.
//! The downloaded announcements deck stays per computer (deck.rs).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

/// Emitted to every window when the library folder or the user's name changes.
pub const EVENT: &str = "episco://storage";

const SETTINGS_FILE: &str = "library.json";
const SUBFOLDERS: [&str; 5] = ["services", "templates", "library", "media", "backgrounds"];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StorageSettings {
    /// The shared parish library folder; `None` keeps everything on this computer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    /// The person using this computer, e.g. "Rector" or "Music".
    #[serde(default)]
    pub name: String,
}

static SETTINGS: RwLock<Option<StorageSettings>> = RwLock::new(None);
/// EpiscoPresenter: the parish library is only a source of services marked
/// Ready (see handoff.rs); the app always works from this computer's folder.
static PRESENTER: AtomicBool = AtomicBool::new(false);

/// Marks this app as the presentation app (call before [`init`]).
pub fn set_presenter() {
    PRESENTER.store(true, Ordering::Relaxed);
}

pub fn is_presenter() -> bool {
    PRESENTER.load(Ordering::Relaxed)
}

/// The parish library folder, if one is chosen and reachable.
pub fn shared_root() -> Option<PathBuf> {
    settings().folder.filter(|f| f.is_dir())
}
/// The data root as last worked out (see [`root`]).
static ROOT: RwLock<Option<PathBuf>> = RwLock::new(None);

/// The folder in use (as [`data_root`]), for code without the app handle;
/// `library:…` paths in services are relative to it.
pub fn root() -> Option<PathBuf> {
    let settings = settings();
    match settings.folder {
        Some(f) if f.is_dir() && !is_presenter() => Some(f),
        _ => ROOT.read().unwrap_or_else(|e| e.into_inner()).clone(),
    }
}

fn settings_path(app: &AppHandle) -> CmdResult<PathBuf> {
    Ok(app.path().app_config_dir().map_err(|e| e.to_string())?.join(SETTINGS_FILE))
}

/// Reads `library.json` at start-up.
pub fn init(app: &AppHandle) {
    let settings = settings_path(app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    *SETTINGS.write().unwrap_or_else(|e| e.into_inner()) = Some(settings);
    *ROOT.write().unwrap_or_else(|e| e.into_inner()) = app.path().app_data_dir().ok();
}

pub fn settings() -> StorageSettings {
    SETTINGS.read().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default()
}

/// The folder everything is kept in: the parish library if one is chosen and
/// reachable, else this computer's data folder.
pub fn data_root(app: &AppHandle) -> CmdResult<PathBuf> {
    match settings().folder {
        Some(folder) if folder.is_dir() && !is_presenter() => Ok(folder),
        _ => app.path().app_data_dir().map_err(|e| e.to_string()),
    }
}

/// A folder inside the data root, e.g. `services`.
pub fn data_dir(app: &AppHandle, name: &str) -> CmdResult<PathBuf> {
    Ok(data_root(app)?.join(name))
}

fn prepare(folder: &Path) -> CmdResult<()> {
    for sub in SUBFOLDERS {
        let dir = folder.join(sub);
        std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }
    Ok(())
}

#[derive(Serialize)]
pub struct StorageInfo {
    #[serde(flatten)]
    settings: StorageSettings,
    /// The folder in use now (the library, or this computer's data folder).
    root: PathBuf,
    /// A library folder is chosen but cannot be reached (e.g. Drive not running).
    missing: bool,
    /// This computer's own data folder.
    local: PathBuf,
    /// EpiscoPresenter: the library is only where Ready services come from.
    presenter: bool,
}

/// The library folder, the folder in use and the user's name.
#[tauri::command]
pub fn storage_get(app: AppHandle) -> CmdResult<StorageInfo> {
    let settings = settings();
    let missing = settings.folder.as_ref().is_some_and(|f| !f.is_dir());
    Ok(StorageInfo {
        root: data_root(&app)?,
        missing,
        local: app.path().app_data_dir().map_err(|e| e.to_string())?,
        presenter: is_presenter(),
        settings,
    })
}

/// Chooses the library folder (`None`: this computer only) and the user's
/// name. Changing the folder closes the open service, since it belongs to
/// the old folder.
#[tauri::command]
pub fn storage_set(
    app: AppHandle,
    engine: State<'_, Engine>,
    folder: Option<PathBuf>,
    name: String,
) -> CmdResult<StorageInfo> {
    if let Some(f) = &folder {
        if !f.is_dir() {
            return Err(format!("{} is not a folder", f.display()));
        }
        prepare(f)?;
    }
    let new = StorageSettings { folder, name: name.trim().to_string() };
    let old = settings();
    let path = settings_path(&app)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&new).map_err(|e| e.to_string())?)
        .map_err(|e| format!("could not save {}: {e}", path.display()))?;
    *SETTINGS.write().unwrap_or_else(|e| e.into_inner()) = Some(new.clone());
    if old.folder != new.folder && !is_presenter() {
        // Slide layouts come from the new folder too.
        crate::templates::init(&app);
        crate::bulletin::init(&app);
        let _ = engine.load_service(None);
    }
    let _ = app.emit(EVENT, ());
    if is_presenter() {
        // Look for Ready services in the new folder now.
        crate::handoff::check(&app);
    }
    storage_get(app)
}

/// Asks for the library folder.
#[tauri::command]
pub async fn storage_pick_folder(app: AppHandle) -> Option<PathBuf> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog().file().set_title("Choose the parish library folder").blocking_pick_folder()?.into_path().ok()
}
