//! From the planner to the presentation computer, safely.
//!
//! In EpiscoPlanner a service is marked **Ready** (a `ready.json` beside its
//! plan in the shared parish library). EpiscoPresenter never works from the
//! shared folder: it copies each Ready service, with its texts, the pictures
//! and videos it uses and the slide layouts, into this computer's own
//! folder, and works from that copy. A Drive outage, a sync conflict or a
//! late edit cannot change what is on the screens during a service.
//!
//! Every minute (and when asked) the presenter compares the shared and local
//! copies:
//!
//! * **new**: Ready, not on this computer yet → copied.
//! * **updated**: changed in the planner since it was copied → copied again,
//!   unless it is open here or was changed here; then the operator chooses
//!   (Refresh).
//! * **changed_here**: edited on this computer since it was copied.
//! * **copied**: the same in both.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::engine::Engine;
use crate::services::PLAN_FILE;

type CmdResult<T> = Result<T, String>;

/// Emitted with the list of Ready services after each check.
pub const EVENT: &str = "episco://handoff";

const READY_FILE: &str = "ready.json";
const SOURCE_FILE: &str = "source.json";

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Who marked a service Ready, and when (seconds since 1970).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyMark {
    #[serde(default)]
    pub by: String,
    pub at: u64,
}

pub fn ready_mark(plan: &Path) -> Option<ReadyMark> {
    let dir = plan.parent()?;
    serde_json::from_str(&std::fs::read_to_string(dir.join(READY_FILE)).ok()?).ok()
}

/// EpiscoPlanner: marks a service Ready for the presentation computer, or not.
#[tauri::command]
pub fn service_set_ready(path: PathBuf, ready: bool) -> CmdResult<Option<ReadyMark>> {
    let dir = path.parent().ok_or("bad service path")?;
    let file = dir.join(READY_FILE);
    if !ready {
        if file.exists() {
            std::fs::remove_file(&file).map_err(|e| e.to_string())?;
        }
        return Ok(None);
    }
    let mark = ReadyMark { by: crate::storage::settings().name, at: now() };
    std::fs::write(&file, serde_json::to_string_pretty(&mark).map_err(|e| e.to_string())?)
        .map_err(|e| format!("could not save {}: {e}", file.display()))?;
    Ok(Some(mark))
}

/// What was copied, kept beside the local copy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Source {
    /// Newest change in the shared folder when it was copied.
    shared_modified: u64,
    /// Newest change in the local copy right after copying.
    local_modified: u64,
    copied_at: u64,
}

/// The newest modification time (seconds) of the files in a service folder,
/// leaving out the Ready and source records.
fn newest(dir: &Path) -> u64 {
    let mut best = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name();
            if name == READY_FILE || name == SOURCE_FILE {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if let Some(t) = e.metadata().ok().and_then(|m| m.modified().ok()) {
                best = best.max(t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
            }
        }
    }
    best
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffState {
    New,
    Copied,
    Updated,
    ChangedHere,
    /// Changed in the planner and here.
    Conflict,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadyService {
    /// The service's folder name (the same in both places).
    folder: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    ready: ReadyMark,
    state: HandoffState,
    /// The local plan, once copied.
    #[serde(skip_serializing_if = "Option::is_none")]
    local: Option<PathBuf>,
}

fn local_services(app: &AppHandle) -> CmdResult<PathBuf> {
    // The presenter's data root is always this computer's folder.
    crate::storage::data_dir(app, "services")
}

/// The Ready services in the parish library and how they compare with the
/// copies here.
pub fn status(app: &AppHandle) -> CmdResult<Vec<ReadyService>> {
    let Some(shared) = crate::storage::shared_root() else { return Ok(Vec::new()) };
    Ok(status_in(&shared, &local_services(app)?))
}

/// A plan's title and date, from its own file only (not its texts, which a
/// shared folder may have to download first).
fn plan_header(plan: &Path) -> Option<(String, Option<String>)> {
    #[derive(serde::Deserialize)]
    struct Header {
        #[serde(default)]
        title: String,
        date: Option<String>,
    }
    let h: Header = toml::from_str(&std::fs::read_to_string(plan).ok()?).ok()?;
    Some((h.title, h.date))
}

/// [`status`] for a shared library folder and a local services folder.
fn status_in(shared: &Path, local: &Path) -> Vec<ReadyService> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(shared.join("services")) else { return out };
    for e in entries.flatten() {
        let dir = e.path();
        let plan = dir.join(PLAN_FILE);
        let Some(ready) = ready_mark(&plan) else { continue };
        if !plan.is_file() {
            continue;
        }
        let folder = e.file_name().to_string_lossy().to_string();
        let (title, date) = plan_header(&plan).unwrap_or_else(|| (folder.clone(), None));
        let here = local.join(&folder);
        let src: Option<Source> =
            std::fs::read_to_string(here.join(SOURCE_FILE)).ok().and_then(|s| serde_json::from_str(&s).ok());
        let state = match (&src, here.join(PLAN_FILE).is_file()) {
            (_, false) | (None, _) => HandoffState::New,
            (Some(src), true) => {
                let theirs = newest(&dir) > src.shared_modified;
                let ours = newest(&here) > src.local_modified;
                match (theirs, ours) {
                    (true, true) => HandoffState::Conflict,
                    (true, false) => HandoffState::Updated,
                    (false, true) => HandoffState::ChangedHere,
                    (false, false) => HandoffState::Copied,
                }
            }
        };
        let local_plan = here.join(PLAN_FILE);
        out.push(ReadyService { folder, title, date, ready, state, local: local_plan.is_file().then_some(local_plan) });
    }
    out.sort_by(|a, b| a.date.cmp(&b.date).then(a.title.cmp(&b.title)));
    out
}

fn copy_dir(from: &Path, to: &Path) -> CmdResult<()> {
    std::fs::create_dir_all(to).map_err(|e| format!("could not create {}: {e}", to.display()))?;
    for e in std::fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let p = e.path();
        let name = e.file_name();
        if name == READY_FILE {
            continue;
        }
        let dest = to.join(&name);
        if p.is_dir() {
            copy_dir(&p, &dest)?;
        } else {
            std::fs::copy(&p, &dest).map_err(|e| format!("could not copy {}: {e}", p.display()))?;
        }
    }
    Ok(())
}

/// `library:…` paths used in a service's files (pictures, videos, backgrounds).
fn library_paths(dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let mut rest = text.as_str();
            while let Some(i) = rest.find(episco_core::portable::PREFIX) {
                let tail = &rest[i + episco_core::portable::PREFIX.len()..];
                // A page of a PDF (`….pdf#page=3`) is the PDF itself.
                let end = tail.find(['"', '\'', ')', '\n', '\r', '#']).unwrap_or(tail.len());
                let rel = tail[..end].trim();
                if !rel.is_empty() && !rel.contains("..") {
                    out.insert(rel.to_string());
                }
                rest = &tail[end..];
            }
        }
    }
    out
}

fn copy_if_newer(from: &Path, to: &Path) -> CmdResult<bool> {
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    if !from.is_file() || (to.is_file() && modified(to) >= modified(from)) {
        return Ok(false);
    }
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::copy(from, to).map_err(|e| format!("could not copy {}: {e}", from.display()))?;
    Ok(true)
}

/// Copies a Ready service (and what it uses) to this computer, replacing
/// the local copy. Returns the local plan.
pub fn copy(app: &AppHandle, folder: &str) -> CmdResult<PathBuf> {
    let shared = crate::storage::shared_root().ok_or("the parish library folder cannot be reached")?;
    let local_root = crate::storage::data_root(app)?;
    // The library comes by the change log (sync.rs): bring it up to date
    // first; the hand-off then only fills in anything still on its way.
    let _ = crate::sync::run(app);
    let (plan, layouts) = copy_between(&shared, &local_root, folder, false)?;
    if layouts {
        crate::templates::init(app);
    }
    // The parish details come too (the welcome slide names the parish).
    crate::bulletin::init(app);
    Ok(plan)
}

/// [`copy`] between a shared library folder and this computer's folder.
/// Returns the local plan and whether the slide layouts were updated.
/// With `library` false (the library is kept in step by sync.rs), the
/// pictures and videos it uses are copied only if missing here, and the
/// layouts and parish details are left to the sync.
fn copy_between(shared: &Path, local_root: &Path, folder: &str, library: bool) -> CmdResult<(PathBuf, bool)> {
    if folder.is_empty() || folder.contains(['/', '\\']) || folder.starts_with('.') {
        return Err("bad service folder".into());
    }
    let from = shared.join("services").join(folder);
    if !from.join(PLAN_FILE).is_file() {
        return Err(format!("{folder} is not in the parish library"));
    }
    let to = local_root.join("services").join(folder);
    // Copy beside, then swap, so a failed copy leaves the old one intact.
    let tmp = to.with_extension("incoming");
    let _ = std::fs::remove_dir_all(&tmp);
    copy_dir(&from, &tmp)?;
    for rel in library_paths(&tmp) {
        let to = local_root.join(&rel);
        if library || !to.is_file() {
            copy_if_newer(&shared.join(&rel), &to)?;
        }
    }
    let mut layouts = false;
    if library {
        // The slide layouts go with the services.
        layouts = copy_if_newer(&shared.join("layouts.json"), &local_root.join("layouts.json"))?;
        // The parish details too (service times, clergy, the bulletin's back page).
        copy_if_newer(&shared.join("parish.json"), &local_root.join("parish.json"))?;
    }
    let _ = std::fs::remove_dir_all(&to);
    std::fs::rename(&tmp, &to).map_err(|e| format!("could not replace {}: {e}", to.display()))?;
    let src = Source { shared_modified: newest(&from), local_modified: newest(&to), copied_at: now() };
    std::fs::write(to.join(SOURCE_FILE), serde_json::to_string_pretty(&src).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok((to.join(PLAN_FILE), layouts))
}

/// The local plan open in the engine, if any.
fn open_plan(app: &AppHandle) -> Option<PathBuf> {
    app.try_state::<Engine>()?.service()?.plan_path.clone()
}

/// Copies new Ready services and quiet updates, then tells the windows.
pub fn check(app: &AppHandle) {
    let Ok(list) = status(app) else { return };
    let open = open_plan(app);
    let mut changed = false;
    for s in &list {
        let is_open = open.is_some() && open == s.local;
        let copy_now = match s.state {
            HandoffState::New => true,
            HandoffState::Updated => !is_open,
            _ => false,
        };
        if copy_now {
            match copy(app, &s.folder) {
                Ok(_) => changed = true,
                Err(e) => tracing::warn!("could not copy {}: {e}", s.folder),
            }
        }
    }
    let list = if changed { status(app).unwrap_or(list) } else { list };
    let _ = app.emit(EVENT, &list);
}

/// Checks every minute in the background.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || loop {
        check(&app);
        std::thread::sleep(Duration::from_secs(60));
    });
}

/// The Ready services and their state (checks now). Run apart from the
/// window: copying from a slow shared folder can take a while.
#[tauri::command]
pub async fn handoff_status(app: AppHandle) -> CmdResult<Vec<ReadyService>> {
    tauri::async_runtime::spawn_blocking(move || {
        check(&app);
        status(&app)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Copies a Ready service again (Refresh), and reopens it if it is open.
#[tauri::command]
pub fn handoff_refresh(app: AppHandle, engine: State<'_, Engine>, folder: String) -> CmdResult<PathBuf> {
    let open = open_plan(&app);
    let plan = copy(&app, &folder)?;
    if open.as_deref() == Some(plan.as_path()) {
        let loaded = episco_core::Service::load_plan(&plan).map_err(|e| e.to_string())?;
        engine.load_service(Some(loaded.service))?;
    }
    let _ = app.emit(EVENT, status(&app).unwrap_or_default());
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_library_paths_and_newest_files() {
        let dir = std::env::temp_dir().join(format!("episco-handoff-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("liturgies")).unwrap();
        std::fs::write(
            dir.join("service.toml"),
            "[[cue]]\nfile = \"liturgies/a.ewm\"\n[cue.sanctuary]\nbackground = \"library:backgrounds/glass.jpg\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("liturgies/a.ewm"),
            "---\ntitle: A\nmedia: library:media/Psalm 23.mp4\n---\n![Antiphon](library:media/antiphon.png)\n\n---\n\n![Slide 2](library:media/talk.pdf#page=2){full}\n",
        )
        .unwrap();
        std::fs::write(dir.join(READY_FILE), "{\"at\": 1}").unwrap();
        let paths: Vec<String> = library_paths(&dir).into_iter().collect();
        assert_eq!(paths, ["backgrounds/glass.jpg", "media/Psalm 23.mp4", "media/antiphon.png", "media/talk.pdf"]);
        assert!(newest(&dir) > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ready_services_are_copied_updated_and_kept_safe() {
        let base = std::env::temp_dir().join(format!("episco-handoff-flow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (shared, local) = (base.join("Drive/Parish Library"), base.join("local"));
        let svc = shared.join("services/2026-11-15-proper-28");
        std::fs::create_dir_all(svc.join("liturgies")).unwrap();
        std::fs::create_dir_all(shared.join("media")).unwrap();
        std::fs::create_dir_all(local.join("services")).unwrap();
        std::fs::write(shared.join("media/antiphon.png"), b"png").unwrap();
        std::fs::write(shared.join("layouts.json"), "{}").unwrap();
        std::fs::write(
            svc.join("liturgies/psalm.ewm"),
            "---\ntitle: Psalm 123\ntype: psalm\n---\n![Antiphon](library:media/antiphon.png)\n",
        )
        .unwrap();
        std::fs::write(
            svc.join(PLAN_FILE),
            "title = \"Proper 28\"\ndate = \"2026-11-15\"\n\n[[cue]]\nfile = \"liturgies/psalm.ewm\"\n",
        )
        .unwrap();
        let state = |l: &Path| status_in(&shared, &l.join("services")).first().map(|s| s.state);

        // Not Ready: nothing to copy.
        assert!(status_in(&shared, &local.join("services")).is_empty());
        // Marked Ready in the planner.
        service_set_ready(svc.join(PLAN_FILE), true).unwrap();
        assert_eq!(state(&local), Some(HandoffState::New));
        let (plan, layouts) = copy_between(&shared, &local, "2026-11-15-proper-28", true).unwrap();
        assert!(plan.is_file() && layouts);
        assert!(local.join("media/antiphon.png").is_file(), "the pictures it uses come too");
        assert!(!plan.parent().unwrap().join(READY_FILE).exists());
        assert_eq!(state(&local), Some(HandoffState::Copied));
        let loaded = episco_core::Service::load_plan(&plan).unwrap();
        assert_eq!(loaded.service.title, "Proper 28");

        // Changed in the planner later.
        std::thread::sleep(Duration::from_millis(1100));
        std::fs::write(svc.join("liturgies/psalm.ewm"), "---\ntitle: Psalm 123 (changed)\ntype: psalm\n---\n").unwrap();
        assert_eq!(state(&local), Some(HandoffState::Updated));
        // …and here too: the operator chooses.
        std::fs::write(plan.parent().unwrap().join("liturgies/psalm.ewm"), "---\ntitle: Local edit\n---\n").unwrap();
        assert_eq!(state(&local), Some(HandoffState::Conflict));
        // Refresh takes the planner's version.
        copy_between(&shared, &local, "2026-11-15-proper-28", true).unwrap();
        assert_eq!(state(&local), Some(HandoffState::Copied));
        let text = std::fs::read_to_string(plan.parent().unwrap().join("liturgies/psalm.ewm")).unwrap();
        assert!(text.contains("(changed)"));

        // Unmarking stops it being offered (the copy here stays).
        service_set_ready(svc.join(PLAN_FILE), false).unwrap();
        assert!(status_in(&shared, &local.join("services")).is_empty());
        assert!(plan.is_file());
        assert!(copy_between(&shared, &local, "../x", true).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }
}
