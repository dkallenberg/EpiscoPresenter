//! The Google Slides announcements deck as a PDF, downloaded from Google and
//! kept in `<app data>/decks/<deck id>.pdf`. Every window turns its pages
//! into slide images (ui/js/deckpdf.js), so the app shows real thumbnails,
//! steps through the deck slide by slide, and knows how many slides it has.
//!
//! Each computer downloads its own copy (paths are not shared over the
//! network). The deck must be shared "Anyone with the link can view".

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

/// Emitted to every window when a deck has been (re)downloaded.
pub const EVENT: &str = "episco://deck";

/// The deck's id in a Google Slides link (not "Publish to web" links, which
/// cannot be downloaded as a PDF).
pub fn deck_id(url: &str) -> Option<String> {
    let url = url.trim();
    if url.contains("/presentation/d/e/") {
        return None;
    }
    let id = if let Some(i) = url.find("/presentation/") {
        let rest = &url[i + "/presentation/".len()..];
        let rest = rest.strip_prefix("u/").map(|r| r.split_once('/').map(|(_, r)| r).unwrap_or("")).unwrap_or(rest);
        rest.strip_prefix("d/")?.split(['/', '?', '#']).next()?.to_string()
    } else {
        url.to_string()
    };
    (id.len() >= 25 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')).then_some(id)
}

fn pdf_path(app: &AppHandle, id: &str) -> CmdResult<PathBuf> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join("decks").join(format!("{id}.pdf")))
}

#[derive(Clone, Serialize)]
pub struct DeckFile {
    /// The downloaded PDF on this computer.
    path: PathBuf,
    id: String,
}

/// Downloads the deck as a PDF (always, with `refresh`; else only if this
/// computer has no copy yet) and tells every window.
#[tauri::command]
pub async fn deck_fetch(app: AppHandle, url: String, refresh: Option<bool>) -> CmdResult<Option<DeckFile>> {
    let Some(id) = deck_id(&url) else { return Ok(None) };
    let path = pdf_path(&app, &id)?;
    if path.is_file() && !refresh.unwrap_or(false) {
        return Ok(Some(DeckFile { path, id }));
    }
    let export = format!("https://docs.google.com/presentation/d/{id}/export/pdf");
    let response = reqwest::get(&export).await.map_err(|e| format!("could not reach Google Slides: {e}"))?;
    let status = response.status();
    let bytes = response.bytes().await.map_err(|e| format!("could not download the deck: {e}"))?;
    if !status.is_success() || !bytes.starts_with(b"%PDF") {
        return Err(
            "Google did not send the slides. In Google Slides, Share the deck as “Anyone with the link can view”, then refresh."
                .into(),
        );
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, &bytes).map_err(|e| format!("could not save the deck: {e}"))?;
    tracing::info!("downloaded the announcements deck ({} KB)", bytes.len() / 1024);
    let file = DeckFile { path, id };
    let _ = app.emit(EVENT, &file);
    Ok(Some(file))
}

/// The downloaded copy of a deck on this computer, if there is one.
#[tauri::command]
pub fn deck_local(app: AppHandle, url: String) -> CmdResult<Option<DeckFile>> {
    let Some(id) = deck_id(&url) else { return Ok(None) };
    let path = pdf_path(&app, &id)?;
    Ok(path.is_file().then_some(DeckFile { path, id }))
}

/// Records how many slides the deck has (counted from the PDF), so a single
/// pass steps through all of them.
#[tauri::command]
pub fn deck_set_slides(engine: State<'_, Engine>, slides: u32) -> CmdResult<()> {
    engine.edit_service(|s| {
        if s.deck_slides != Some(slides) && slides > 0 {
            s.deck_slides = Some(slides);
            s.modified = true;
            s.rebuild_all();
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_ids_from_links() {
        let id = "112f2iF7ZVUGdU25aC_MCR2AUJc-jw4gRaOpqtdlgk5o";
        for url in [
            format!("https://docs.google.com/presentation/d/{id}/edit?usp=sharing"),
            format!("https://docs.google.com/presentation/d/{id}/present#slide=id.p"),
            format!("https://docs.google.com/presentation/u/1/d/{id}/edit"),
            format!("https://docs.google.com/presentation/d/{id}"),
            id.to_string(),
        ] {
            assert_eq!(deck_id(&url).as_deref(), Some(id), "{url}");
        }
        assert_eq!(deck_id("https://docs.google.com/presentation/d/e/2PACX-1vXYZ/pub"), None);
        assert_eq!(deck_id("https://example.com/"), None);
    }
}
