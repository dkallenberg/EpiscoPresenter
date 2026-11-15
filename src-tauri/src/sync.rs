//! The parish library on this computer, kept in step with the shared
//! parish library folder by a change log.
//!
//! Each computer keeps its own copy of the library (library items and
//! hymns, media, backgrounds, the Bible file, slide layouts and the parish
//! details) and works from it, which is quick even when the shared folder
//! is a Google Drive folder that downloads files only when they are read.
//!
//! The shared folder holds a change log per computer
//! (`change-log/<computer>.jsonl`): one line for each file it added,
//! changed or removed. Every 15 seconds, soon after a save here, and at
//! start-up the app:
//!
//! 1. reads the lines the other computers have added since it last looked,
//!    and copies those files here (or removes them);
//! 2. looks for files changed here since the last time (a quick look at
//!    this computer's own folder), copies them to the shared folder and
//!    adds a line for each to its own log.
//!
//! The first time (or with a new shared folder) it compares the two folders
//! file by file instead: a file on one side only is copied to the other,
//! and where both have one the newer wins.
//!
//! Google Drive's conflict copies (`name (1).ewm` beside `name.ewm`) are
//! left alone.
//!
//! Services and templates are not copied: they stay in the shared folder
//! (EpiscoPresenter takes its services from there when they are Ready).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

type CmdResult<T> = Result<T, String>;

/// The folders kept in step.
const DIRS: [&str; 4] = ["library", "media", "backgrounds", "bible"];
/// The files at the top of the library kept in step.
const FILES: [&str; 4] = ["layouts.json", "layouts-default.json", "parish.json", "gospel-music.json"];
/// The change logs, in the shared folder.
const LOG_DIR: &str = "change-log";
const STATE_FILE: &str = "library-sync.json";

/// Whether a top-level name (`"media"`, `"layouts.json"`) is kept on this
/// computer and in step with the shared folder.
pub fn is_synced(name: &str) -> bool {
    DIRS.contains(&name) || FILES.contains(&name) || name.starts_with("parish-logo.")
}

/// A file's modification time (ns) and size.
type Stamp = (u128, u64);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct State {
    /// This computer's log name.
    id: String,
    /// The shared folder this state is for.
    shared: Option<PathBuf>,
    /// The first comparison is done.
    ready: bool,
    /// The files here, as they were after the last sync.
    files: BTreeMap<String, Stamp>,
    /// How many lines of each other computer's log are done.
    read: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Op {
    Put,
    Del,
}

/// One line of a change log.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Change {
    /// When (seconds since 1970).
    t: u64,
    op: Op,
    /// Relative to the library folder, with `/`.
    path: String,
    #[serde(default)]
    size: u64,
    /// Who (the name set with the library folder).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    by: String,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn stamp_of(m: &std::fs::Metadata) -> Stamp {
    let t = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_nanos());
    (t.unwrap_or(0), m.len())
}

fn hidden(name: &str) -> bool {
    name.starts_with('.') || name.starts_with('~') || name == "desktop.ini" || name.starts_with("Icon\r")
}

/// Every kept file under `root`: relative path → stamp. Only looks at
/// names and dates (no contents are read).
fn walk(root: &Path) -> BTreeMap<String, Stamp> {
    let mut out = BTreeMap::new();
    let Ok(entries) = std::fs::read_dir(root) else { return out };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if hidden(&name) || !is_synced(&name) {
            continue;
        }
        let Ok(m) = e.metadata() else { continue };
        if m.is_file() {
            out.insert(name, stamp_of(&m));
        } else if m.is_dir() {
            let mut stack = vec![(e.path(), name)];
            while let Some((dir, rel)) = stack.pop() {
                let Ok(entries) = std::fs::read_dir(&dir) else { continue };
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if hidden(&name) {
                        continue;
                    }
                    let Ok(m) = e.metadata() else { continue };
                    let rel = format!("{rel}/{name}");
                    if m.is_dir() {
                        stack.push((e.path(), rel));
                    } else if m.is_file() {
                        out.insert(rel, stamp_of(&m));
                    }
                }
            }
        }
    }
    let copies: Vec<String> = out.keys().filter(|k| conflict_copy(k, &out)).cloned().collect();
    for k in copies {
        out.remove(&k);
    }
    out
}

/// Google Drive's (and Dropbox's) copy of a file two computers changed at
/// once: `name (1).ewm` beside `name.ewm`. Not kept in step: the library
/// would show the item twice.
fn conflict_copy(rel: &str, all: &BTreeMap<String, Stamp>) -> bool {
    let (dir, name) = rel.rsplit_once('/').map(|(d, n)| (Some(d), n)).unwrap_or((None, rel));
    let (stem, ext) = name.rsplit_once('.').map(|(s, e)| (s, Some(e))).unwrap_or((name, None));
    let Some(open) = stem.rfind(" (") else { return false };
    let n = &stem[open + 2..];
    if !n.ends_with(')') || n.len() < 2 || !n[..n.len() - 1].chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let base = &stem[..open];
    let original = match (dir, ext) {
        (Some(d), Some(e)) => format!("{d}/{base}.{e}"),
        (Some(d), None) => format!("{d}/{base}"),
        (None, Some(e)) => format!("{base}.{e}"),
        (None, None) => base.to_string(),
    };
    all.contains_key(&original)
}

/// A relative path from a log, if it is a kept file and stays inside.
fn safe(rel: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = rel.split('/').collect();
    let ok = parts.first().is_some_and(|p| is_synced(p))
        && parts.iter().all(|p| !p.is_empty() && *p != "." && *p != ".." && !p.contains('\\') && !p.contains(':'));
    ok.then(|| parts.iter().collect())
}

/// Copies a file, making its folder, by way of a temporary file so a reader
/// never sees half of it.
fn copy(from: &Path, to: &Path) -> CmdResult<()> {
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    }
    let tmp = to.with_extension("episco-part");
    std::fs::copy(from, &tmp).map_err(|e| format!("could not copy {}: {e}", from.display()))?;
    std::fs::rename(&tmp, to).map_err(|e| format!("could not replace {}: {e}", to.display()))
}

fn same_contents(a: &Path, b: &Path) -> bool {
    matches!((std::fs::read(a), std::fs::read(b)), (Ok(x), Ok(y)) if x == y)
}

/// One computer's view: its own folder, the shared one, who it is.
struct Sides<'a> {
    local: &'a Path,
    shared: &'a Path,
    by: &'a str,
}

impl Sides<'_> {
    fn log_file(&self, id: &str) -> PathBuf {
        self.shared.join(LOG_DIR).join(format!("{id}.jsonl"))
    }

    fn local_stamp(&self, rel: &str) -> Option<Stamp> {
        std::fs::metadata(self.local.join(safe(rel)?)).ok().filter(|m| m.is_file()).map(|m| stamp_of(&m))
    }

    /// Adds lines to this computer's log.
    fn log(&self, id: &str, changes: &[Change]) -> CmdResult<()> {
        if changes.is_empty() {
            return Ok(());
        }
        let file = self.log_file(id);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&file).map_err(|e| e.to_string())?;
        let mut text = String::new();
        for c in changes {
            text.push_str(&serde_json::to_string(c).map_err(|e| e.to_string())?);
            text.push('\n');
        }
        f.write_all(text.as_bytes()).map_err(|e| e.to_string())
    }

    /// The other computers' logs: name → lines.
    fn other_logs(&self, id: &str) -> BTreeMap<String, Vec<String>> {
        let mut out = BTreeMap::new();
        let Ok(entries) = std::fs::read_dir(self.shared.join(LOG_DIR)) else { return out };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(other) = name.strip_suffix(".jsonl") else { continue };
            if other == id || hidden(&name) {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(e.path()) {
                // A last line still being written (no newline yet) waits.
                let complete = text.rfind('\n').map(|i| &text[..i]).unwrap_or("");
                out.insert(other.to_string(), complete.lines().map(String::from).collect());
            }
        }
        out
    }

    /// The first time: both folders compared file by file.
    fn reconcile(&self, st: &mut State) -> CmdResult<bool> {
        let (theirs, ours) = (walk(self.shared), walk(self.local));
        let mut changed = false;
        let mut sent = Vec::new();
        let paths: Vec<String> =
            theirs.keys().chain(ours.keys()).cloned().collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        for rel in paths {
            let Some(p) = safe(&rel) else { continue };
            let (s, l) = (self.shared.join(&p), self.local.join(&p));
            let take = |changed: &mut bool| -> CmdResult<()> {
                copy(&s, &l)?;
                *changed = true;
                Ok(())
            };
            match (theirs.get(&rel), ours.get(&rel)) {
                (Some(_), None) => take(&mut changed)?,
                (None, Some(stamp)) => {
                    copy(&l, &s)?;
                    sent.push(Change { t: now(), op: Op::Put, path: rel.clone(), size: stamp.1, by: self.by.into() });
                }
                (Some(a), Some(b)) if a.1 != b.1 || !same_contents(&s, &l) => {
                    if a.0 > b.0 {
                        take(&mut changed)?;
                    } else {
                        copy(&l, &s)?;
                        sent.push(Change { t: now(), op: Op::Put, path: rel.clone(), size: b.1, by: self.by.into() });
                    }
                }
                _ => {}
            }
            if let Some(stamp) = self.local_stamp(&rel) {
                st.files.insert(rel, stamp);
            }
        }
        self.log(&st.id, &sent)?;
        // Everything the logs say so far is in the comparison.
        st.read = self.other_logs(&st.id).into_iter().map(|(k, v)| (k, v.len())).collect();
        st.ready = true;
        Ok(changed)
    }

    /// The other computers' changes since last time. Returns whether any
    /// file here changed, and how many files are still on their way.
    fn pull(&self, st: &mut State) -> (bool, usize) {
        let mut changed = false;
        let mut waiting = 0;
        for (log, lines) in self.other_logs(&st.id) {
            let mut done = st.read.get(&log).copied().unwrap_or(0).min(lines.len());
            for line in &lines[done..] {
                let Ok(c) = serde_json::from_str::<Change>(line) else {
                    done += 1;
                    continue;
                };
                let Some(p) = safe(&c.path) else {
                    done += 1;
                    continue;
                };
                let (s, l) = (self.shared.join(&p), self.local.join(&p));
                // Changed here since the last sync, after that change: ours wins
                // (it goes to the shared folder next).
                let ours_newer = match (self.local_stamp(&c.path), st.files.get(&c.path)) {
                    (Some(now), Some(then)) => now != *then && now.0 / 1_000_000_000 > u128::from(c.t),
                    (Some(_), None) => true,
                    _ => false,
                };
                match c.op {
                    Op::Put if ours_newer => {}
                    Op::Put => {
                        // The file itself may still be on its way (Google Drive):
                        // wait for it, and for the rest of this log after it.
                        let arrived = std::fs::metadata(&s).ok().filter(|m| m.is_file()).map(|m| m.len());
                        if arrived != Some(c.size) {
                            waiting += lines.len() - done;
                            break;
                        }
                        if copy(&s, &l).is_err() {
                            break;
                        }
                        changed = true;
                        if let Some(stamp) = self.local_stamp(&c.path) {
                            st.files.insert(c.path.clone(), stamp);
                        }
                    }
                    Op::Del if ours_newer => {}
                    Op::Del => {
                        if l.is_file() {
                            let _ = std::fs::remove_file(&l);
                            changed = true;
                        }
                        st.files.remove(&c.path);
                    }
                }
                done += 1;
            }
            st.read.insert(log, done);
        }
        (changed, waiting)
    }

    /// This computer's changes since last time, to the shared folder.
    fn push(&self, st: &mut State) -> CmdResult<()> {
        let ours = walk(self.local);
        let mut sent = Vec::new();
        for (rel, stamp) in &ours {
            if st.files.get(rel) == Some(stamp) {
                continue;
            }
            let Some(p) = safe(rel) else { continue };
            copy(&self.local.join(&p), &self.shared.join(&p))?;
            sent.push(Change { t: now(), op: Op::Put, path: rel.clone(), size: stamp.1, by: self.by.into() });
            st.files.insert(rel.clone(), *stamp);
        }
        let gone: Vec<String> = st.files.keys().filter(|k| !ours.contains_key(*k)).cloned().collect();
        for rel in gone {
            if let Some(p) = safe(&rel) {
                let s = self.shared.join(p);
                if s.is_file() {
                    let _ = std::fs::remove_file(&s);
                }
            }
            sent.push(Change { t: now(), op: Op::Del, path: rel.clone(), size: 0, by: self.by.into() });
            st.files.remove(&rel);
        }
        self.log(&st.id, &sent)
    }
}

/// A name for this computer's log: random enough, readable enough.
fn new_id(by: &str) -> String {
    let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
        ^ u128::from(std::process::id());
    let who: String = by.chars().filter(|c| c.is_ascii_alphanumeric()).take(16).collect::<String>().to_lowercase();
    let app = if crate::storage::is_presenter() { "presenter" } else { "planner" };
    format!("{}{app}-{:x}", if who.is_empty() { String::new() } else { format!("{who}-") }, n & 0xffff_ffff_ffff)
}

/// Brings this computer's library and the shared folder into step (see the
/// module notes). Returns whether anything here changed.
fn sync_folders(local: &Path, shared: &Path, by: &str, st: &mut State) -> CmdResult<(bool, usize)> {
    if st.id.is_empty() {
        st.id = new_id(by);
    }
    if st.shared.as_deref() != Some(shared) {
        *st = State { id: std::mem::take(&mut st.id), shared: Some(shared.to_path_buf()), ..Default::default() };
    }
    std::fs::create_dir_all(local).map_err(|e| e.to_string())?;
    let sides = Sides { local, shared, by };
    // No mark of a first copy here (a new copy, or one removed): compare the
    // folders again rather than take the missing files as removed.
    let marker = local.join(crate::storage::COPY_MADE);
    if !st.ready || !marker.is_file() {
        st.ready = false;
        st.files.clear();
        let changed = sides.reconcile(st)?;
        let _ = std::fs::write(&marker, "");
        return Ok((changed, 0));
    }
    let pulled = sides.pull(st);
    sides.push(st)?;
    Ok(pulled)
}

/// Emitted when files of the library came from another computer (the
/// Library's lists refresh; nothing else changes on screen).
pub const LIBRARY_EVENT: &str = "episco://library";

/// Only one sync at a time.
static RUNNING: Mutex<()> = Mutex::new(());

/// How the last sync went, for the Library's status line.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncStatus {
    /// A shared folder is chosen.
    pub shared: bool,
    /// When the last sync finished (seconds since 1970), if one has.
    pub at: Option<u64>,
    /// Why it failed (e.g. the shared folder cannot be reached).
    pub error: Option<String>,
    /// Files another computer saved that are still on their way here.
    pub waiting: usize,
}

static STATUS: Mutex<SyncStatus> = Mutex::new(SyncStatus { shared: false, at: None, error: None, waiting: 0 });

/// Syncs now, if a shared folder is chosen and reachable. Tells the windows
/// (and reloads the layouts and parish details) when files here changed.
pub fn run(app: &AppHandle) -> CmdResult<bool> {
    let result = run_once(app);
    let mut status = STATUS.lock().unwrap_or_else(|e| e.into_inner());
    status.shared = crate::storage::shared_root().is_some();
    match &result {
        Ok((_, waiting)) => {
            status.at = Some(now());
            status.error = None;
            status.waiting = *waiting;
        }
        Err(e) => status.error = Some(e.clone()),
    }
    result.map(|(changed, _)| changed)
}

fn run_once(app: &AppHandle) -> CmdResult<(bool, usize)> {
    let Some(shared) = crate::storage::shared_root() else { return Ok((false, 0)) };
    let _one = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    let local = crate::storage::copy_root(app)?;
    let state_file = app.path().app_local_data_dir().map_err(|e| e.to_string())?.join(STATE_FILE);
    let first = !local.join(crate::storage::COPY_MADE).is_file();
    let mut st: State =
        std::fs::read_to_string(&state_file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let by = crate::storage::settings().name;
    let result = sync_folders(&local, &shared, &by, &mut st);
    if let Some(dir) = state_file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&state_file, serde_json::to_string(&st).unwrap_or_default());
    let (changed, waiting) = result?;
    // The first copy is made: work from it from now on.
    if first {
        crate::storage::copy_made(app);
        let _ = app.emit(LIBRARY_EVENT, ());
    }
    if changed {
        crate::templates::init(app);
        crate::bulletin::init(app);
        let _ = app.emit(LIBRARY_EVENT, ());
    }
    Ok((changed, waiting))
}

/// Set by [`soon`]: something in the library was just saved here.
static NUDGE: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());

/// Asks for a sync in a moment (after a library item, layout or the parish
/// details is saved here), so the other computer sees it soon.
pub fn soon() {
    let (flag, wake) = &NUDGE;
    *flag.lock().unwrap_or_else(|e| e.into_inner()) = true;
    wake.notify_one();
}

/// How often the library is synced when nothing was saved here.
const EVERY: Duration = Duration::from_secs(15);

/// Syncs at start-up, every 15 seconds and soon after a save here, apart
/// from the windows.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || loop {
        if let Err(e) = run(&app) {
            tracing::warn!("library sync: {e}");
        }
        let (flag, wake) = &NUDGE;
        let guard = flag.lock().unwrap_or_else(|e| e.into_inner());
        let (mut nudged, _) = wake.wait_timeout_while(guard, EVERY, |n| !*n).unwrap_or_else(|e| e.into_inner());
        let was = *nudged;
        *nudged = false;
        drop(nudged);
        // A save often comes with others: wait a moment and sync them together.
        if was {
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

/// Syncs now (e.g. when the Library is opened).
#[tauri::command]
pub async fn library_sync(app: AppHandle) -> CmdResult<bool> {
    tauri::async_runtime::spawn_blocking(move || run(&app)).await.map_err(|e| e.to_string())?
}

/// How the last sync went.
#[tauri::command]
pub fn library_sync_status() -> SyncStatus {
    let mut s = STATUS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    s.shared = crate::storage::shared_root().is_some();
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("episco-sync-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn put(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    fn read(root: &Path, rel: &str) -> Option<String> {
        std::fs::read_to_string(root.join(rel)).ok()
    }

    #[test]
    fn two_computers_share_a_library_through_the_change_log() {
        let shared = tmp("shared");
        let (a, b) = (tmp("a"), tmp("b"));
        // Before: each has its own hymn; the shared folder has a picture.
        put(&a, "library/hymn-436.ewm", "436");
        put(&b, "library/hymn-450.ewm", "450");
        put(&shared, "media/window.png", "png");
        put(&shared, "services/x/service.toml", "not copied");
        let (mut sa, mut sb) = (State::default(), State::default());

        // First runs: the folders are compared and brought together.
        sync_folders(&a, &shared, "Rector", &mut sa).unwrap();
        sync_folders(&b, &shared, "Tech", &mut sb).unwrap();
        sync_folders(&a, &shared, "Rector", &mut sa).unwrap();
        for side in [&a, &b] {
            assert_eq!(read(side, "library/hymn-436.ewm").as_deref(), Some("436"));
            assert_eq!(read(side, "library/hymn-450.ewm").as_deref(), Some("450"));
            assert_eq!(read(side, "media/window.png").as_deref(), Some("png"));
            assert!(read(side, "services/x/service.toml").is_none(), "services stay in the shared folder");
        }

        // A change on A reaches B through A's log; a removal on B reaches A.
        std::thread::sleep(Duration::from_millis(20));
        put(&a, "library/hymn-436.ewm", "436, new words");
        put(&a, "layouts.json", "{}");
        sync_folders(&a, &shared, "Rector", &mut sa).unwrap();
        std::fs::remove_file(b.join("media/window.png")).unwrap();
        assert!(sync_folders(&b, &shared, "Tech", &mut sb).unwrap().0, "B took A's changes");
        assert_eq!(read(&b, "library/hymn-436.ewm").as_deref(), Some("436, new words"));
        assert_eq!(read(&b, "layouts.json").as_deref(), Some("{}"));
        assert!(read(&shared, "media/window.png").is_none());
        sync_folders(&a, &shared, "Rector", &mut sa).unwrap();
        assert!(read(&a, "media/window.png").is_none(), "B's removal reached A");

        // Nothing new: nothing copied, nothing logged.
        let log = std::fs::read_to_string(shared.join(LOG_DIR).join(format!("{}.jsonl", sa.id))).unwrap();
        assert!(!sync_folders(&a, &shared, "Rector", &mut sa).unwrap().0);
        assert_eq!(std::fs::read_to_string(shared.join(LOG_DIR).join(format!("{}.jsonl", sa.id))).unwrap(), log);
        assert!(log.contains("\"by\":\"Rector\""));

        // A file whose log line arrives before the file waits for it.
        let late = Change { t: now(), op: Op::Put, path: "library/late.ewm".into(), size: 4, by: "Music".into() };
        Sides { local: &a, shared: &shared, by: "Music" }.log("music-planner-1", &[late]).unwrap();
        sync_folders(&a, &shared, "Rector", &mut sa).unwrap();
        assert!(read(&a, "library/late.ewm").is_none());
        put(&shared, "library/late.ewm", "late");
        sync_folders(&a, &shared, "Rector", &mut sa).unwrap();
        assert_eq!(read(&a, "library/late.ewm").as_deref(), Some("late"));

        // A's copy removed (a new folder chosen and then the old one again):
        // compared afresh, so nothing in the shared folder is removed.
        std::fs::remove_dir_all(&a).unwrap();
        sync_folders(&a, &shared, "Rector", &mut sa).unwrap();
        assert_eq!(read(&shared, "library/hymn-450.ewm").as_deref(), Some("450"));
        assert_eq!(read(&a, "library/hymn-450.ewm").as_deref(), Some("450"));

        for d in [shared, a, b] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    #[test]
    fn log_paths_stay_inside_the_library() {
        assert!(safe("library/x.ewm").is_some());
        assert!(safe("parish-logo.png").is_some());
        assert!(safe("services/x/service.toml").is_none());
        assert!(safe("library/../services/x").is_none());
        assert!(safe("/etc/passwd").is_none());
        assert!(safe("media/a\\b.png").is_none());
    }

    #[test]
    fn drive_conflict_copies_are_left_alone() {
        let shared = tmp("conflict-shared");
        let a = tmp("conflict-a");
        put(&shared, "library/hymn-436.ewm", "one");
        put(&shared, "library/hymn-436 (1).ewm", "two");
        put(&shared, "library/Psalm (23).ewm", "psalm");
        let mut sa = State::default();
        sync_folders(&a, &shared, "Rector", &mut sa).unwrap();
        assert!(a.join("library/hymn-436.ewm").is_file());
        assert!(!a.join("library/hymn-436 (1).ewm").exists(), "Drive's conflict copy stays behind");
        assert!(a.join("library/Psalm (23).ewm").is_file(), "a name with a number of its own is kept");
        for d in [shared, a] {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}
