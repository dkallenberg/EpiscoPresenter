//! Slide layouts edited in the Layout tool: positions and fonts of every
//! piece of each slide type. App-wide, saved as `layouts.json` in the app's
//! data folder, and copied into the live service so every node renders the
//! same way (see `episco_core::template`).

use std::path::PathBuf;
use std::sync::RwLock;

use episco_core::template::{self, SlideTemplate, Templates};
use tauri::{AppHandle, Manager, State};

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

static CURRENT: RwLock<Templates> = RwLock::new(Templates::new());

fn file(app: &AppHandle) -> CmdResult<PathBuf> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join("layouts.json"))
}

/// Loads the saved layouts at startup.
pub fn init(app: &AppHandle) {
    let Ok(path) = file(app) else { return };
    let Ok(text) = std::fs::read_to_string(&path) else { return };
    match serde_json::from_str::<Templates>(&text) {
        Ok(t) => *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = t,
        Err(e) => tracing::warn!("ignoring {}: {e}", path.display()),
    }
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

/// Resets one slide type (or, with no key, every one) to the built-in layout.
#[tauri::command]
pub fn layouts_reset(app: AppHandle, engine: State<'_, Engine>, key: Option<String>) -> CmdResult<Templates> {
    let mut all = current();
    match key {
        Some(k) => {
            all.remove(&k);
        }
        None => all.clear(),
    }
    store(&app, &engine, all)
}
