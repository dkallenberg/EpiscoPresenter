//! Undo and Redo for the open service.
//!
//! Every change to the open service goes through `Engine::edit_service`,
//! which hands the service as it was before to [`record`]. Undo puts that
//! back (and keeps the newer one for Redo). Changes coming in quick
//! succession (typing in the Editor sends one every third of a second) are
//! one step. Opening another service starts afresh. The history is this
//! computer's and lasts while the app is open.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use episco_core::Service;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

/// Emitted to the windows when what can be undone changes.
pub const EVENT: &str = "episco://undo";
/// Changes closer together than this are one step.
const BURST: Duration = Duration::from_millis(1500);
/// Steps kept.
const DEPTH: usize = 100;

#[derive(Default)]
struct History {
    undo: Vec<Arc<Service>>,
    redo: Vec<Arc<Service>>,
    last: Option<Instant>,
}

/// How many steps there are each way, for the buttons.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Status {
    pub undo: usize,
    pub redo: usize,
}

static HISTORY: Mutex<Option<History>> = Mutex::new(None);
static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

fn with<T>(f: impl FnOnce(&mut History) -> T) -> T {
    let mut guard = HISTORY.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(Default::default))
}

fn status() -> Status {
    with(|h| Status { undo: h.undo.len(), redo: h.redo.len() })
}

fn tell() {
    if let Some(app) = APP.get() {
        let _ = app.emit(EVENT, status());
    }
}

/// A change was made: `before` is the service as it was.
pub fn record(before: Arc<Service>, after: Option<&Service>) {
    if after.is_some_and(|a| *a == *before) {
        return;
    }
    with(|h| {
        let now = Instant::now();
        // Part of the same burst: the step already holds the service as it
        // was before the burst began.
        let same_burst = h.redo.is_empty() && h.last.is_some_and(|t| now.duration_since(t) < BURST);
        if !same_burst || h.undo.is_empty() {
            h.undo.push(before);
            if h.undo.len() > DEPTH {
                h.undo.remove(0);
            }
        }
        h.redo.clear();
        h.last = Some(now);
    });
    tell();
}

/// Another service was opened (or none): start afresh.
pub fn clear() {
    with(|h| *h = History::default());
    tell();
}

/// Puts back a service from the history (in place of the one open now).
fn restore(engine: &Engine, to: Arc<Service>) -> CmdResult<()> {
    engine.replace_service(|s| {
        *s = (*to).clone();
        // Saved again, every item: one may have been saved since.
        for cue in &mut s.cues {
            cue.modified = true;
        }
        s.modified = true;
    })
}

/// Undoes the last change to the open service.
#[tauri::command]
pub fn undo(engine: State<'_, Engine>) -> CmdResult<Status> {
    let now = engine.service().ok_or("no service is open")?;
    let Some(prev) = with(|h| h.undo.pop()) else { return Ok(status()) };
    if let Err(e) = restore(&engine, prev.clone()) {
        with(|h| h.undo.push(prev));
        return Err(e);
    }
    with(|h| {
        h.redo.push(now);
        h.last = None;
    });
    tell();
    Ok(status())
}

/// Does again what was undone.
#[tauri::command]
pub fn redo(engine: State<'_, Engine>) -> CmdResult<Status> {
    let now = engine.service().ok_or("no service is open")?;
    let Some(next) = with(|h| h.redo.pop()) else { return Ok(status()) };
    if let Err(e) = restore(&engine, next.clone()) {
        with(|h| h.redo.push(next));
        return Err(e);
    }
    with(|h| {
        h.undo.push(now);
        h.last = None;
    });
    tell();
    Ok(status())
}

/// What can be undone and redone now.
#[tauri::command]
pub fn undo_status() -> Status {
    status()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc(title: &str) -> Arc<Service> {
        Arc::new(Service { title: title.into(), ..Default::default() })
    }

    #[test]
    fn bursts_are_one_step_and_a_new_change_clears_redo() {
        clear();
        record(svc("a"), Some(&svc("b")));
        record(svc("b"), Some(&svc("c"))); // typing on: same step
        assert_eq!(status().undo, 1);
        assert_eq!(with(|h| h.undo[0].title.clone()), "a", "the step goes back to before the burst");
        // Unchanged: nothing recorded.
        record(svc("c"), Some(&svc("c")));
        assert_eq!(status().undo, 1);
        // After a pause, a new step.
        with(|h| h.last = Some(Instant::now() - BURST * 2));
        record(svc("c"), Some(&svc("d")));
        assert_eq!(status().undo, 2);
        with(|h| h.redo.push(svc("x")));
        with(|h| h.last = Some(Instant::now() - BURST * 2));
        record(svc("d"), Some(&svc("e")));
        assert_eq!(status().redo, 0, "a new change clears Redo");
        clear();
        assert_eq!(status().undo, 0);
    }
}
