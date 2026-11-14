//! "Open on another computer": EpiscoPlanner marks the service (or
//! template) it has open, so two people do not change it at once.
//!
//! The mark is a small file in the shared folder's `in-use/` folder, named
//! for the service's folder, with who has it open and when they last
//! confirmed it: the app rewrites it every 20 seconds while the service is
//! open, and removes it on closing. A mark not confirmed for 3 minutes (a
//! computer that crashed or went to sleep) no longer counts. It is kept
//! apart from the service's own folder so that it does not look like a
//! change to the service (EpiscoPresenter copies a Ready service when its
//! folder changes).
//!
//! Opening a service someone else has open gives it view only, with "Take
//! over": that reloads the service as last saved and marks it as ours. The
//! other computer sees the new mark on its next check and turns view only.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

/// Emitted to the windows when the service's in-use status changes.
pub const EVENT: &str = "episco://in-use";
const DIR: &str = "in-use";
/// How often a mark is confirmed.
const BEAT: Duration = Duration::from_secs(20);
/// A mark older than this no longer counts.
const STALE: u64 = 180;

/// Who has a service open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    /// The name chosen with the library folder (e.g. "Rector").
    pub name: String,
    /// This run of the app (one person may have two computers).
    pub session: String,
    /// When it was opened (seconds since 1970).
    pub since: u64,
    /// When it was last confirmed.
    pub beat: u64,
}

/// The open service's status, for the windows.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Status {
    /// Someone else has it open: it is view only here.
    pub view_only: bool,
    /// Who has it (when view only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<Mark>,
    /// We had it and they took it over.
    pub taken: bool,
}

#[derive(Default)]
struct Current {
    /// The open plan, if it is in the shared folder.
    plan: Option<PathBuf>,
    /// Ours since.
    since: u64,
    status: Status,
    /// The next load of this plan takes it over.
    take_over: Option<PathBuf>,
}

static CURRENT: Mutex<Option<Current>> = Mutex::new(None);
static APP: OnceLock<AppHandle> = OnceLock::new();
static VIEW_ONLY: AtomicBool = AtomicBool::new(false);

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn session() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| {
        let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        format!("{:x}", n ^ (u128::from(std::process::id()) << 64))
    })
}

/// Edits are refused while someone else has the open service.
pub fn is_view_only() -> bool {
    VIEW_ONLY.load(Ordering::Relaxed)
}

/// The mark file for a plan in `shared` (none for a plan elsewhere).
fn mark_file(shared: &Path, plan: &Path) -> Option<PathBuf> {
    let rel = plan.parent()?.strip_prefix(shared).ok()?;
    let key: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    (!key.is_empty()).then(|| shared.join(DIR).join(format!("{}.json", key.join("__"))))
}

fn read(file: &Path) -> Option<Mark> {
    serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()
}

/// Someone else's mark that still counts.
fn other(file: &Path, at: u64) -> Option<Mark> {
    read(file).filter(|m| m.session != session() && at.saturating_sub(m.beat) < STALE)
}

fn write(file: &Path, since: u64, at: u64) {
    let mark = Mark { name: crate::storage::settings().name, session: session().into(), since, beat: at };
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, serde_json::to_string(&mark).unwrap_or_default());
}

/// Removes our mark (if it is still ours).
fn release(file: &Path) {
    if read(file).is_some_and(|m| m.session == session()) {
        let _ = std::fs::remove_file(file);
    }
}

fn set(cur: Current, app: &AppHandle) {
    VIEW_ONLY.store(cur.status.view_only, Ordering::Relaxed);
    let status = cur.status.clone();
    *CURRENT.lock().unwrap_or_else(|e| e.into_inner()) = Some(cur);
    let _ = app.emit(EVENT, status);
}

/// A service was opened (or closed: `None`). EpiscoPlanner only, and only
/// for services and templates in the shared folder.
pub fn opened(plan: Option<&Path>) {
    let Some(app) = APP.get() else { return };
    let shared = crate::storage::shared_root().filter(|_| !crate::storage::is_presenter());
    let mut cur = CURRENT.lock().unwrap_or_else(|e| e.into_inner()).take().unwrap_or_default();
    let take_over = cur.take_over.take();
    // Leave the one we had (unless it is the same one again).
    if let (Some(old), Some(shared)) = (&cur.plan, &shared) {
        if plan != Some(old.as_path()) && !cur.status.view_only {
            if let Some(f) = mark_file(shared, old) {
                release(&f);
            }
        }
    }
    let file = shared.as_deref().zip(plan).and_then(|(s, p)| mark_file(s, p));
    let Some(file) = file else {
        return set(Current::default(), app);
    };
    let plan = plan.map(Path::to_path_buf);
    let at = now();
    let again = cur.plan == plan && !cur.status.view_only;
    let since = if again { cur.since } else { at };
    let status = match other(&file, at) {
        Some(holder) if take_over.as_ref() != plan.as_ref() => {
            Status { view_only: true, holder: Some(holder), taken: false }
        }
        _ => {
            write(&file, since, at);
            Status::default()
        }
    };
    set(Current { plan, since, status, take_over: None }, app);
}

/// Confirms our mark, and notices a take-over or a holder who has gone.
fn beat(app: &AppHandle) {
    let Some(shared) = crate::storage::shared_root() else { return };
    let mut guard = CURRENT.lock().unwrap_or_else(|e| e.into_inner());
    let Some(cur) = guard.as_mut() else { return };
    let Some(file) = cur.plan.as_deref().and_then(|p| mark_file(&shared, p)) else { return };
    let at = now();
    let before = cur.status.clone();
    match (other(&file, at), cur.status.view_only) {
        // Someone took it over (their mark is newer than ours).
        (Some(m), false) if m.since >= cur.since => {
            cur.status = Status { view_only: true, holder: Some(m), taken: true };
        }
        (Some(_), false) | (None, false) => write(&file, cur.since, at),
        // Still theirs: keep the name and time up to date.
        (Some(m), true) => cur.status.holder = Some(m),
        // They have gone: it can be edited after taking it over (nothing
        // here has changed, so taking over just reloads it).
        (None, true) => cur.status.holder = None,
    }
    if cur.status.view_only != before.view_only || cur.status.holder != before.holder {
        VIEW_ONLY.store(cur.status.view_only, Ordering::Relaxed);
        let _ = app.emit(EVENT, cur.status.clone());
    }
}

/// Confirms the mark every 20 seconds.
pub fn spawn(app: AppHandle) {
    let _ = APP.set(app.clone());
    std::thread::spawn(move || loop {
        std::thread::sleep(BEAT);
        beat(&app);
    });
}

/// Removes our mark when the app closes.
pub fn close() {
    let Some(shared) = crate::storage::shared_root() else { return };
    let cur = CURRENT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(cur) = cur.as_ref().filter(|c| !c.status.view_only) {
        if let Some(f) = cur.plan.as_deref().and_then(|p| mark_file(&shared, p)) {
            release(&f);
        }
    }
}

/// Who has a service open on another computer, if anyone (for the
/// Services screen).
pub fn holder(plan: &Path) -> Option<Mark> {
    if crate::storage::is_presenter() {
        return None;
    }
    let shared = crate::storage::shared_root()?;
    other(&mark_file(&shared, plan)?, now())
}

/// The open service's status.
#[tauri::command]
pub fn in_use_status() -> Status {
    CURRENT.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|c| c.status.clone()).unwrap_or_default()
}

/// Takes the open service over from whoever has it: reloads it as last
/// saved and marks it as ours.
#[tauri::command]
pub fn in_use_take_over(engine: State<'_, Engine>) -> CmdResult<()> {
    let plan = engine.service().and_then(|s| s.plan_path.clone()).ok_or("no service is open")?;
    let loaded = episco_core::Service::load_plan(&plan).map_err(|e| e.to_string())?;
    if let Some(c) = CURRENT.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        c.take_over = Some(plan);
    }
    engine.load_service(Some(loaded.service))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_are_kept_apart_from_the_service_and_expire() {
        let shared = std::env::temp_dir().join(format!("episco-inuse-{}", std::process::id()));
        let plan = shared.join("services/2026-11-22-christ-the-king/service.toml");
        let file = mark_file(&shared, &plan).unwrap();
        assert_eq!(file, shared.join("in-use/services__2026-11-22-christ-the-king.json"));
        assert!(mark_file(&shared, Path::new("/elsewhere/x/service.toml")).is_none());

        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let theirs = Mark { name: "Rector".into(), session: "someone-else".into(), since: 100, beat: 1000 };
        std::fs::write(&file, serde_json::to_string(&theirs).unwrap()).unwrap();
        assert_eq!(other(&file, 1000 + STALE - 1).map(|m| m.name), Some("Rector".into()));
        assert_eq!(other(&file, 1000 + STALE), None, "not confirmed for 3 minutes: gone");

        // Our own mark is not someone else's, and release removes only ours.
        release(&file);
        assert!(file.is_file(), "theirs is left alone");
        write(&file, 5, now());
        assert_eq!(other(&file, now()), None);
        release(&file);
        assert!(!file.is_file());
        let _ = std::fs::remove_dir_all(&shared);
    }
}
