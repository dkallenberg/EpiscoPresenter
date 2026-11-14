//! Slide layouts edited in the Layout tool: positions and fonts of every
//! piece of each slide type. App-wide, saved as `layouts.json` in the app's
//! data folder, and copied into the live service so every node renders the
//! same way (see `episco_core::template`).

use std::path::PathBuf;
use std::sync::RwLock;

use episco_core::template::{self, SlideTemplate, Templates};
use tauri::{AppHandle, State};

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

static CURRENT: RwLock<Templates> = RwLock::new(Templates::new());

fn file(app: &AppHandle) -> CmdResult<PathBuf> {
    crate::storage::data_dir(app, "layouts.json")
}

/// Loads the saved layouts at startup.
pub fn init(app: &AppHandle) {
    // A folder without layouts.json uses the built-in layouts.
    let mut loaded = Templates::new();
    if let Ok(path) = file(app) {
        if let Ok(text) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<Templates>(&text) {
                Ok(t) => loaded = t,
                Err(e) => tracing::warn!("ignoring {}: {e}", path.display()),
            }
        }
    }
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = loaded;
}

/// The layouts every loaded service gets.
pub fn current() -> Templates {
    CURRENT.read().unwrap_or_else(|e| e.into_inner()).clone()
}

fn store(app: &AppHandle, engine: &Engine, mut templates: Templates) -> CmdResult<Templates> {
    template::prune(&mut templates);
    let path = file(app)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(&templates).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("could not save {}: {e}", path.display()))?;
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = templates.clone();
    if engine.service().is_some() {
        let t = templates.clone();
        engine.edit_service(move |s| {
            s.templates = t;
            // How much text a slide holds can change the slides.
            s.rebuild_all();
            Ok(())
        })?;
    }
    Ok(templates)
}

/// The operator's changes to the built-in layouts.
#[tauri::command]
pub fn layouts_get() -> Templates {
    current()
}

/// Replaces one slide type's changes (an empty template resets it).
#[tauri::command]
pub fn layouts_set(
    app: AppHandle,
    engine: State<'_, Engine>,
    key: String,
    template: SlideTemplate,
) -> CmdResult<Templates> {
    let mut all = current();
    all.insert(key, template);
    store(&app, &engine, all)
}

/// The parish's own default layouts (`layouts-default.json`, kept in step
/// with the shared folder like the layouts themselves).
fn default_file(app: &AppHandle) -> CmdResult<PathBuf> {
    crate::storage::data_dir(app, "layouts-default.json")
}

fn read_defaults(app: &AppHandle) -> Templates {
    default_file(app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// The parish's defaults: what Reset goes back to (else the built-in).
#[tauri::command]
pub fn layouts_defaults(app: AppHandle) -> Templates {
    read_defaults(&app)
}

/// Makes a slide type's layout as it is now (or, with no key, every one)
/// the parish's default, which Reset goes back to.
#[tauri::command]
pub fn layouts_make_default(app: AppHandle, key: Option<String>) -> CmdResult<Templates> {
    let now = current();
    let mut defaults = read_defaults(&app);
    match key {
        Some(k) => match now.get(&k) {
            Some(t) => {
                defaults.insert(k, t.clone());
            }
            // As built: the default is the built-in layout.
            None => {
                defaults.remove(&k);
            }
        },
        None => defaults = now,
    }
    template::prune(&mut defaults);
    let path = default_file(&app)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&defaults).map_err(|e| e.to_string())?)
        .map_err(|e| format!("could not save {}: {e}", path.display()))?;
    Ok(defaults)
}

/// Resets one slide type (or, with no key, every one) to the parish's
/// default, or with `builtin` to the built-in layout.
#[tauri::command]
pub fn layouts_reset(
    app: AppHandle,
    engine: State<'_, Engine>,
    key: Option<String>,
    builtin: Option<bool>,
) -> CmdResult<Templates> {
    let defaults = if builtin.unwrap_or(false) { Templates::new() } else { read_defaults(&app) };
    let mut all = current();
    match key {
        Some(k) => match defaults.get(&k) {
            Some(t) => {
                all.insert(k, t.clone());
            }
            None => {
                all.remove(&k);
            }
        },
        None => all = defaults,
    }
    store(&app, &engine, all)
}
