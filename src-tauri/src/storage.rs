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
//! Each computer works from its own copy of the library (items, hymns,
//! media, backgrounds, layouts, parish details), kept in step with the
//! shared folder through a change log: see sync.rs. Services and templates
//! are used in the shared folder itself.
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
    /// The parish library is a Google Drive folder reached directly
    /// (drive.rs); `folder` is then this computer's copy of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive: Option<crate::drive::DriveLink>,
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

/// The parish library folder, if one is chosen and reachable. A Google
/// Drive folder counts once its first copy here is made.
pub fn shared_root() -> Option<PathBuf> {
    let s = settings();
    if s.drive.is_some() && !crate::drive::mirror_ready() {
        return None;
    }
    s.folder.filter(|f| f.is_dir())
}
/// The data root as last worked out (see [`root`]).
static ROOT: RwLock<Option<PathBuf>> = RwLock::new(None);

/// This computer's copy of the parish library (see [`local_root`]), for
/// code without the app handle.
static LOCAL: RwLock<Option<PathBuf>> = RwLock::new(None);

/// The library on this computer (as [`local_root`]), for code without the
/// app handle; `library:…` paths in services are relative to it.
pub fn root() -> Option<PathBuf> {
    LOCAL
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .or_else(|| ROOT.read().unwrap_or_else(|e| e.into_inner()).clone())
}

/// Where this computer keeps the parish library (items, hymns, media,
/// backgrounds, the Bible file, layouts, parish details): EpiscoPresenter's
/// own data folder; for EpiscoPlanner with a shared folder chosen, a copy in
/// its local data folder, kept in step with the shared folder (sync.rs);
/// else the app's data folder.
///
/// Until EpiscoPlanner's copy has been made the first time (which can take a
/// while from Google Drive), it keeps working from the shared folder.
pub fn local_root(app: &AppHandle) -> CmdResult<PathBuf> {
    let copy = copy_root(app)?;
    match shared_root() {
        Some(shared) if !is_presenter() && !copy.join(COPY_MADE).is_file() => Ok(shared),
        _ => Ok(copy),
    }
}

/// Marks a finished first copy of the library (sync.rs).
pub const COPY_MADE: &str = ".copy-made";

/// Where this computer's copy of the library is kept (see [`local_root`]).
pub fn copy_root(app: &AppHandle) -> CmdResult<PathBuf> {
    let path = app.path();
    match settings().folder {
        Some(_) if !is_presenter() => Ok(path.app_local_data_dir().map_err(|e| e.to_string())?.join("parish-library")),
        _ => path.app_data_dir().map_err(|e| e.to_string()),
    }
}

/// After the first copy: from now on, work from it.
pub fn copy_made(app: &AppHandle) {
    set_local(app);
}

pub fn set_local(app: &AppHandle) {
    *LOCAL.write().unwrap_or_else(|e| e.into_inner()) = local_root(app).ok();
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
    set_local(app);
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

/// A folder (or file) of the library, e.g. `services` or `layouts.json`:
/// the parts kept on this computer (see sync.rs) come from its copy, the
/// rest (services, templates, bulletins) from the data root.
pub fn data_dir(app: &AppHandle, name: &str) -> CmdResult<PathBuf> {
    if crate::sync::is_synced(name) {
        return Ok(local_root(app)?.join(name));
    }
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
    apply(&app, engine.inner(), StorageSettings { folder, name: name.trim().to_string(), drive: None })
}

/// Saves the library settings and switches to them (storage_set, and
/// drive.rs for a Google Drive folder).
pub fn apply(app: &AppHandle, engine: &Engine, new: StorageSettings) -> CmdResult<StorageInfo> {
    let app = app.clone();
    let old = settings();
    if old.folder != new.folder && !is_presenter() {
        // EpiscoPlanner's copy belongs to the old folder: send it any changes
        // not yet sent, then start a new copy for the new folder.
        let _ = crate::sync::run(&app);
        if let Ok(copy) = copy_root(&app) {
            if copy.ends_with("parish-library") && copy.is_dir() {
                let _ = std::fs::remove_dir_all(&copy);
            }
        }
    }
    let path = settings_path(&app)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&new).map_err(|e| e.to_string())?)
        .map_err(|e| format!("could not save {}: {e}", path.display()))?;
    *SETTINGS.write().unwrap_or_else(|e| e.into_inner()) = Some(new.clone());
    set_local(&app);
    if old.folder != new.folder && !is_presenter() {
        // Slide layouts come from the new folder too.
        crate::templates::init(&app);
        crate::bulletin::init(&app);
        let _ = engine.load_service(None);
    }
    let _ = app.emit(EVENT, ());
    // Bring this computer's library into step with the new folder, then
    // (EpiscoPresenter) look for Ready services there.
    let later = app.clone();
    std::thread::spawn(move || {
        let _ = crate::sync::run(&later);
        if is_presenter() {
            crate::handoff::check(&later);
        }
    });
    storage_get(app)
}

/// Asks for the library folder.
#[tauri::command]
pub async fn storage_pick_folder(app: AppHandle) -> Option<PathBuf> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog().file().set_title("Choose the parish library folder").blocking_pick_folder()?.into_path().ok()
}
