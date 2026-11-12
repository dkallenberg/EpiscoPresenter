//! Reading texts from a Bible file the parish provides.
//!
//! The NRSV and NRSVue are copyright, so the app cannot include them. A
//! parish that has the text as a JSON file (see
//! [`episco_core::bible::BibleText`]) imports it once; it is copied into the
//! library folder (`bible/NRSVue.json` or `bible/NRSV.json`) so every
//! computer has it, and readings are then filled in from their citations:
//! automatically when propers are added or a service is created, and on
//! request from the Reading texts dialog.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use episco_core::bible::{self, BibleText, BibleVersion};
use episco_core::{Block, ElementCategory, LiturgyDocument, Service};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

/// Words to a slide for filled-in readings (as the paste dialog's default).
const WORDS: usize = 40;

type Cached = (PathBuf, Option<SystemTime>, Arc<BibleText>);
static CACHE: Mutex<Option<HashMap<BibleVersion, Cached>>> = Mutex::new(None);

fn file_for(app: &AppHandle, version: BibleVersion) -> CmdResult<PathBuf> {
    Ok(crate::storage::data_dir(app, "bible")?.join(format!("{}.json", version.initials())))
}

/// The Bible file for `version`, if the parish has imported one (read once
/// and kept until the file changes).
pub fn bible(app: &AppHandle, version: BibleVersion) -> Option<Arc<BibleText>> {
    let path = file_for(app, version).ok()?;
    let modified = std::fs::metadata(&path).ok()?.modified().ok();
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = cache.get_or_insert_with(HashMap::new);
    if let Some((p, m, b)) = cache.get(&version) {
        if *p == path && *m == modified {
            return Some(b.clone());
        }
    }
    let text = std::fs::read_to_string(&path).ok()?;
    match BibleText::from_json(&text) {
        Ok(b) => {
            let b = Arc::new(b);
            cache.insert(version, (path, modified, b.clone()));
            Some(b)
        }
        Err(e) => {
            tracing::warn!("{}: {e}", path.display());
            None
        }
    }
}

fn is_reading(doc: &LiturgyDocument) -> bool {
    matches!(doc.meta.effective_category(), ElementCategory::Reading | ElementCategory::Gospel)
}

fn has_text(doc: &LiturgyDocument) -> bool {
    doc.blocks.iter().any(|b| matches!(b, Block::Paragraph { role: None, .. }))
}

/// Fills a reading without text from the Bible file. `Ok(false)`: nothing
/// to do (not a reading, already has text, or no citation).
fn fill(doc: &mut LiturgyDocument, bible: &BibleText, version: BibleVersion) -> Result<bool, String> {
    if !is_reading(doc) || has_text(doc) {
        return Ok(false);
    }
    let Some(citation) = doc.meta.citation.clone().filter(|c| !c.trim().is_empty()) else { return Ok(false) };
    let text = bible.passage(&citation, true)?;
    *doc = bible::set_reading_passage(doc, &text, version, WORDS);
    Ok(true)
}

/// Fills the readings among `docs` (about to be added to a service) when
/// the parish has a Bible file for `version`. Failures leave a reading as it
/// was, to be pasted by hand.
pub fn fill_documents(app: &AppHandle, docs: &mut [LiturgyDocument], version: BibleVersion) {
    let Some(bible) = bible(app, version) else { return };
    for doc in docs {
        if let Err(e) = fill(doc, &bible, version) {
            tracing::warn!("{e}");
        }
    }
}

/// Fills every reading of a new service that has no text yet.
pub fn fill_service(app: &AppHandle, service: &mut Service) {
    let version = service.bible.unwrap_or_default();
    let Some(bible) = bible(app, version) else { return };
    let ids: Vec<String> = service.cues.iter().map(|c| c.id.clone()).collect();
    for id in ids {
        let Some(mut doc) = service.cue(&id).and_then(|c| c.document()).cloned() else { continue };
        if let Ok(true) = fill(&mut doc, &bible, version) {
            let _ = service.set_document(&id, doc);
        }
    }
}

#[derive(Serialize)]
pub struct BibleInfo {
    version: BibleVersion,
    books: usize,
}

/// The Bible files the parish has imported.
#[tauri::command]
pub fn bible_status(app: AppHandle) -> Vec<BibleInfo> {
    BibleVersion::ALL
        .iter()
        .filter_map(|&v| bible(&app, v).map(|b| BibleInfo { version: v, books: b.book_count() }))
        .collect()
}

/// Asks for a Bible JSON file.
#[tauri::command]
pub async fn bible_pick_file(app: AppHandle) -> Option<PathBuf> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .set_title("Choose the Bible text file (JSON)")
        .add_filter("Bible (JSON)", &["json", "JSON"])
        .blocking_pick_file()?
        .into_path()
        .ok()
}

/// Imports a Bible JSON file as `version`: checks it, then copies it into
/// the library folder. Returns the number of books.
#[tauri::command]
pub fn bible_import(app: AppHandle, path: PathBuf, version: BibleVersion) -> CmdResult<usize> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let books = BibleText::from_json(&text)?.book_count();
    let dest = file_for(&app, version)?;
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    if dest != path {
        std::fs::write(&dest, text).map_err(|e| format!("could not save {}: {e}", dest.display()))?;
    }
    Ok(books)
}

/// Removes an imported Bible file.
#[tauri::command]
pub fn bible_remove(app: AppHandle, version: BibleVersion) -> CmdResult<()> {
    let path = file_for(&app, version)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Serialize)]
pub struct FillResult {
    filled: usize,
    /// "First Reading: no book called …"
    failed: Vec<String>,
}

/// Fills the open service's readings from the Bible file for `version`
/// (else the service's). `ids`: only these; `replace`: also readings that
/// already have text.
#[tauri::command]
pub fn reading_fill(
    app: AppHandle,
    engine: State<'_, Engine>,
    ids: Option<Vec<String>>,
    version: Option<BibleVersion>,
    replace: bool,
) -> CmdResult<FillResult> {
    let service = engine.service().ok_or("no service is open")?;
    let version = version.or(service.bible).unwrap_or_default();
    let bible = bible(&app, version).ok_or_else(|| {
        format!("no {} Bible file has been imported: use Import Bible file… first", version.initials())
    })?;
    let mut changed = Vec::new();
    let mut failed = Vec::new();
    for cue in &service.cues {
        if ids.as_ref().is_some_and(|ids| !ids.contains(&cue.id)) {
            continue;
        }
        let Some(doc) = cue.document() else { continue };
        let mut doc = doc.clone();
        if replace && has_text(&doc) {
            doc = bible::set_reading_text(&doc, "", version, WORDS);
        }
        match fill(&mut doc, &bible, version) {
            Ok(true) => changed.push((cue.id.clone(), doc)),
            Ok(false) => {}
            Err(e) => failed.push(format!("{}: {e}", cue.title)),
        }
    }
    let filled = changed.len();
    if filled > 0 {
        engine.edit_service(|s| {
            for (id, doc) in changed {
                s.set_document(&id, doc).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?;
    }
    Ok(FillResult { filled, failed })
}
