//! Saves the open service automatically, shortly after it stops changing.
//!
//! Runs on the master / standalone node. Any service with a plan file is
//! saved about 1.5 s after the last edit (cue edits, reordering, looks,
//! service settings). A service without a plan file (the built-in example)
//! is not saved until it is kept (services.rs `service_keep`).

use std::sync::Arc;
use std::time::Duration;

use episco_sync::{HubUpdate, ServerMessage, SyncHub};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast::error::RecvError;

/// Quiet period before saving.
const DELAY: Duration = Duration::from_millis(1500);

/// Event emitted to the windows after every autosave attempt.
pub const EVENT: &str = "episco://autosave";

#[derive(Clone, Serialize)]
struct Status {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn needs_save(update: &HubUpdate) -> bool {
    matches!(&update.message, ServerMessage::Service { service: Some(s), .. }
        if s.plan_path.is_some() && s.has_unsaved_changes())
}

pub fn spawn(app: AppHandle, hub: Arc<SyncHub>) {
    tokio::spawn(async move {
        let mut updates = hub.subscribe();
        loop {
            // Wait for a change that needs saving.
            match updates.recv().await {
                Ok(u) if needs_save(&u) => {}
                Ok(_) | Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return,
            }
            // Debounce: keep waiting while changes keep coming.
            loop {
                match tokio::time::timeout(DELAY, updates.recv()).await {
                    Ok(Ok(_)) | Ok(Err(RecvError::Lagged(_))) => continue,
                    Ok(Err(RecvError::Closed)) => return,
                    Err(_) => break,
                }
            }
            let result = hub.edit_service(|s| match s.plan_path.clone() {
                Some(path) if s.has_unsaved_changes() => s.save_to(&path).map_err(|e| Some(e.to_string())),
                // Nothing to do: return an error so no update is broadcast.
                _ => Err(None),
            });
            let status = match result {
                Ok(()) => Some(Status { ok: true, error: None }),
                Err(Some(e)) => {
                    tracing::error!("autosave failed: {e}");
                    Some(Status { ok: false, error: Some(e) })
                }
                Err(None) => None,
            };
            if let Some(status) = status {
                let _ = app.emit(EVENT, status);
            }
        }
    });
}
