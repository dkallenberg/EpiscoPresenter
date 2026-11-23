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
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

type CmdResult<T> = Result<T, String>;

/// The parish's update signing key (public half, from `cargo tauri signer
/// generate`), given to the build as `EPISCO_UPDATE_PUBKEY`. Empty: no
/// update checks.
pub const PUBKEY: &str = match option_env!("EPISCO_UPDATE_PUBKEY") {
    Some(k) => k,
    None => "",
};

/// Where each app finds its newest version: `presenter.json` lists the
/// newest of all (betas too), `presenter-release.json` the newest full
/// release (the Build installers workflow writes both).
const UPDATES: &str = "https://github.com/dkallenberg/EpiscoPresenter/releases/download/updates";

/// This computer's choice of updates (`updates.json` in the app's settings
/// folder). Unset: EpiscoPlanner takes betas, EpiscoPresenter, the computer
/// that runs Sunday, only full releases.
#[derive(Debug, Default, Clone, Serialize, serde::Deserialize)]
struct UpdateSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    betas: Option<bool>,
}

fn settings_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("updates.json"))
}

fn settings(app: &AppHandle) -> UpdateSettings {
    settings_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Whether this computer takes betas.
fn betas(app: &AppHandle, planner: bool) -> bool {
    settings(app).betas.unwrap_or(planner)
}

fn endpoint(planner: bool, betas: bool) -> String {
    let app = if planner { "planner" } else { "presenter" };
    if betas {
        format!("{UPDATES}/{app}.json")
    } else {
        format!("{UPDATES}/{app}-release.json")
    }
}

/// Which app this is (set at start-up).
static PLANNER: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn is_planner() -> bool {
    PLANNER.load(std::sync::atomic::Ordering::Relaxed)
}

/// Whether this copy can update itself: a release build with the key.
fn can_update() -> bool {
    !PUBKEY.is_empty() && !cfg!(debug_assertions)
}

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
    PLANNER.store(planner, std::sync::atomic::Ordering::Relaxed);
    if !can_update() {
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
    let url = endpoint(planner, betas(app, planner));
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

/// ⚙ Settings → Updates…: the version installed and this computer's choice.
#[derive(Debug, Serialize)]
pub struct UpdateStatus {
    version: String,
    betas: bool,
    /// A release build that can update itself.
    can_update: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    ready: Option<UpdateInfo>,
}

#[tauri::command]
pub fn update_status(app: AppHandle) -> UpdateStatus {
    UpdateStatus {
        version: app.package_info().version.to_string(),
        betas: betas(&app, is_planner()),
        can_update: can_update(),
        ready: update_ready(),
    }
}

/// Takes betas on this computer, or only full releases.
#[tauri::command]
pub fn update_set_betas(app: AppHandle, betas: bool) -> CmdResult<()> {
    let path = settings_path(&app).ok_or("no settings folder")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&UpdateSettings { betas: Some(betas) }).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("could not save the setting: {e}"))
}

/// Looks for an update now (Updates… → Check now); one found is downloaded
/// and offered as at start-up.
#[tauri::command]
pub async fn update_check_now(app: AppHandle) -> CmdResult<Option<UpdateInfo>> {
    if !can_update() {
        return Err("This copy of the app can't update itself: install new versions from the Releases page.".into());
    }
    let info = check(&app, is_planner()).await?;
    if let Some(i) = &info {
        let _ = app.emit(EVENT, i);
    }
    Ok(info)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_computer_reads_its_list() {
        assert!(endpoint(false, false).ends_with("/updates/presenter-release.json"), "full releases only");
        assert!(endpoint(false, true).ends_with("/updates/presenter.json"), "betas too");
        assert!(endpoint(true, true).ends_with("/updates/planner.json"));
        assert!(endpoint(true, false).ends_with("/updates/planner-release.json"));
    }
}
