//! Updates from the project's GitHub Releases.
//!
//! A little after start-up the app asks GitHub for the newest version of
//! itself (`presenter.json` / `planner.json` in the `updates` release, which
//! the Build installers workflow writes). If there is one it is downloaded in
//! the background and checked against the parish's public key, and the
//! window offers "Restart to update". Nothing is installed until someone
//! clicks it, so an update never interrupts a service.
//!
//! Updates are signed with the parish's private key (the GitHub secret
//! `TAURI_SIGNING_PRIVATE_KEY`, used by the workflow); [`PUBKEY`] is the
//! matching public key, built in from the repository variable
//! `TAURI_SIGNING_PUBLIC_KEY`. Without it the app does not look for updates.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Update, UpdaterExt};

type CmdResult<T> = Result<T, String>;

/// The parish's update signing key (public half, from `cargo tauri signer
/// generate`), given to the build as `EPISCO_UPDATE_PUBKEY`. Empty: no
/// update checks.
pub const PUBKEY: &str = match option_env!("EPISCO_UPDATE_PUBKEY") {
    Some(k) => k,
    None => "",
};

/// Where each app finds its newest version.
const PRESENTER: &str = "https://github.com/dkallenberg/EpiscoPresenter/releases/download/updates/presenter.json";
const PLANNER: &str = "https://github.com/dkallenberg/EpiscoPresenter/releases/download/updates/planner.json";

/// Emitted when an update is downloaded and ready to install.
pub const EVENT: &str = "episco://update";

/// How long after start-up to look (the app settles first).
const DELAY: Duration = Duration::from_secs(10);

/// An update downloaded and checked, waiting for "Restart to update".
static READY: Mutex<Option<(Update, Vec<u8>)>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    /// The new version ("0.2.1").
    pub version: String,
    /// What changed, as the release says.
    pub notes: Option<String>,
}

/// The updater plugin, with the parish's key.
pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R, tauri_plugin_updater::Config> {
    tauri_plugin_updater::Builder::new().pubkey(PUBKEY).build()
}

/// Looks for an update a little after start-up (release builds only).
pub fn spawn(app: AppHandle, planner: bool) {
    if PUBKEY.is_empty() || cfg!(debug_assertions) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(DELAY).await;
        match check(&app, planner).await {
            Ok(Some(info)) => {
                tracing::info!("update {} downloaded", info.version);
                let _ = app.emit(EVENT, &info);
            }
            Ok(None) => tracing::info!("no update"),
            Err(e) => tracing::warn!("update check: {e}"),
        }
    });
}

async fn check(app: &AppHandle, planner: bool) -> CmdResult<Option<UpdateInfo>> {
    let url = if planner { PLANNER } else { PRESENTER };
    let updater = app
        .updater_builder()
        .endpoints(vec![url.parse().map_err(|e| format!("{e}"))?])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else { return Ok(None) };
    let bytes = update.download(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    let info = UpdateInfo { version: update.version.clone(), notes: update.body.clone() };
    *READY.lock().unwrap_or_else(|e| e.into_inner()) = Some((update, bytes));
    Ok(Some(info))
}

/// The update waiting to be installed, if any.
#[tauri::command]
pub fn update_ready() -> Option<UpdateInfo> {
    READY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|(u, _)| UpdateInfo { version: u.version.clone(), notes: u.body.clone() })
}

/// Installs the downloaded update and restarts the app (on Windows the
/// installer closes the app and opens it again when done).
#[tauri::command]
pub fn update_install(app: AppHandle) -> CmdResult<()> {
    let Some((update, bytes)) = READY.lock().unwrap_or_else(|e| e.into_inner()).take() else {
        return Err("No update is waiting.".into());
    };
    update.install(bytes).map_err(|e| format!("could not install the update: {e}"))?;
    app.restart();
}
