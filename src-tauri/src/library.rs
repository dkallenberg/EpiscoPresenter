//! The item library: reusable prayers, collects, hymns and readings that can
//! be browsed by type and added to any service.
//!
//! * **Built-in** items (Rite II texts from the 1979 BCP) are compiled into
//!   the app from `library/`.
//! * **My library** items are `.ewm` files in the app's data folder
//!   (`<app data>/library/`), written by "Save to Library" or by dropping files
//!   there. A saved item with the same title as a built-in one is listed
//!   alongside it, not instead of it.
//!
//! Adding an item to a service copies its document into a new cue; the cue
//! is saved beside the service plan on the next Save Service, so editing it
//! for one Sunday never changes the library.

use std::path::PathBuf;

use episco_core::ewm;
use episco_core::propers::{self, ProperItem, Propers};
use episco_core::{Cue, CueOverrides, LiturgyDocument};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::commands::slug_file_name;
use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

macro_rules! builtin {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_str!(concat!("../../library/", $name, ".ewm")))),*]
    };
}

/// Built-in items, in the order of the service: Rite II, then Rite I.
const BUILTIN: &[(&str, &str)] = builtin![
    "acclamation-ordinary",
    "acclamation-easter",
    "acclamation-lent",
    "collect-for-purity",
    "gloria-in-excelsis",
    "kyrie",
    "salutation",
    "nicene-creed",
    "confession",
    "absolution",
    "the-peace",
    "sursum-corda",
    "sanctus",
    "lords-prayer",
    "lords-prayer-traditional",
    "fraction-anthem",
    "invitation",
    "postcommunion-eternal-god",
    "postcommunion-almighty",
    "dismissal-go-forth",
    "dismissal-go-in-peace",
    "dismissal-rejoicing",
    "dismissal-bless-the-lord",
    "rite1/acclamation-ordinary",
    "rite1/acclamation-easter",
    "rite1/acclamation-lent",
    "rite1/collect-for-purity",
    "rite1/summary-of-the-law",
    "rite1/kyrie",
    "rite1/gloria-in-excelsis",
    "rite1/salutation",
    "rite1/nicene-creed",
    "rite1/confession",
    "rite1/confession-short",
    "rite1/absolution",
    "rite1/the-peace",
    "rite1/sursum-corda",
    "rite1/sanctus",
    "rite1/lords-prayer",
    "rite1/fraction-anthem",
    "rite1/agnus-dei",
    "rite1/humble-access",
    "rite1/invitation",
    "rite1/postcommunion",
    "rite1/blessing",
];

#[derive(Serialize)]
pub struct LibraryEntry {
    /// `builtin:<name>` or `user:<file name>`.
    id: String,
    title: String,
    category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    citation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    season: Option<String>,
    /// "I" or "II" for texts that differ between the rites.
    #[serde(skip_serializing_if = "Option::is_none")]
    rite: Option<String>,
    builtin: bool,
    /// First words of the text, for the list.
    summary: String,
    slides: usize,
}

fn user_dir(app: &AppHandle) -> CmdResult<PathBuf> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("library");
    Ok(dir)
}

fn entry(id: String, doc: &LiturgyDocument, builtin: bool) -> LibraryEntry {
    let mut summary: String = doc
        .blocks
        .iter()
        .filter(|b| b.role().is_none_or(|r| !r.is_rubric()))
        .map(|b| b.plain_text().replace('\n', " "))
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    if summary.chars().count() > 140 {
        summary = summary.chars().take(140).collect::<String>() + "…";
    }
    LibraryEntry {
        id,
        title: doc.meta.title.clone(),
        category: doc.meta.effective_category().key().to_string(),
        citation: doc.meta.citation.clone(),
        season: doc.meta.season.map(|s| s.label().to_string()),
        rite: doc.meta.extra("rite").map(String::from),
        builtin,
        summary,
        slides: doc.slides().len(),
    }
}

fn user_files(app: &AppHandle) -> Vec<PathBuf> {
    let Ok(dir) = user_dir(app) else { return Vec::new() };
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(ewm::EXTENSION))
        .collect();
    files.sort();
    files
}

/// A built-in item's document by name (e.g. `"collect-for-purity"`).
pub fn builtin_doc(name: &str) -> Option<LiturgyDocument> {
    let (_, src) = BUILTIN.iter().find(|(n, _)| *n == name)?;
    ewm::parse(src).ok().map(|p| p.document)
}

/// Loads a library item's document by id.
fn load(app: &AppHandle, id: &str) -> CmdResult<LiturgyDocument> {
    if let Some(name) = id.strip_prefix("builtin:") {
        let (_, src) = BUILTIN.iter().find(|(n, _)| *n == name).ok_or("no such built-in item")?;
        return ewm::parse(src).map(|p| p.document).map_err(|e| e.to_string());
    }
    let file = id.strip_prefix("user:").ok_or("bad library id")?;
    if file.contains(['/', '\\']) || file.starts_with('.') {
        return Err("bad library id".into());
    }
    let path = user_dir(app)?.join(file);
    ewm::load_file(&path).map(|p| p.document).map_err(|e| e.to_string())
}

/// Every library item: built-in first (service order), then the user's.
#[tauri::command]
pub fn library_list(app: AppHandle) -> Vec<LibraryEntry> {
    let mut out: Vec<LibraryEntry> = BUILTIN
        .iter()
        .filter_map(|(name, src)| ewm::parse(src).ok().map(|p| entry(format!("builtin:{name}"), &p.document, true)))
        .collect();
    for path in user_files(&app) {
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
        match ewm::load_file(&path) {
            Ok(p) => out.push(entry(format!("user:{name}"), &p.document, false)),
            Err(e) => tracing::warn!("skipping library file: {e}"),
        }
    }
    out
}

/// A library item as a cue (resolved against the current service), for previews.
#[tauri::command]
pub fn library_preview(app: AppHandle, engine: State<'_, Engine>, id: String) -> CmdResult<Cue> {
    let doc = load(&app, &id)?;
    let mut cue = Cue::liturgy("preview", doc, None, CueOverrides::default());
    if let Some(service) = engine.service() {
        cue.rebuild(&service.context());
    }
    Ok(cue)
}

fn insert_documents(engine: &Engine, docs: Vec<LiturgyDocument>, after: Option<usize>) -> CmdResult<Vec<String>> {
    engine.edit_service(|s| {
        let start = after.map(|a| a + 1).unwrap_or(s.cues.len());
        let mut ids = Vec::new();
        for (at, doc) in (start..).zip(docs) {
            let mut cue = Cue::liturgy(s.unique_id(&doc.meta.title), doc, None, CueOverrides::default());
            cue.modified = true; // saved beside the plan on Save Service
            ids.push(s.insert_cue(at, cue));
        }
        Ok(ids)
    })
}

/// Adds library items to the service after cue index `after` (else at the end).
#[tauri::command]
pub fn library_add(
    app: AppHandle,
    engine: State<'_, Engine>,
    ids: Vec<String>,
    after: Option<usize>,
) -> CmdResult<Vec<String>> {
    let docs = ids.iter().map(|id| load(&app, id)).collect::<CmdResult<Vec<_>>>()?;
    insert_documents(&engine, docs, after)
}

/// Saves a cue's current text to My Library. With `replace` false, refuses
/// to overwrite an existing item of the same name (the UI then asks).
#[tauri::command]
pub fn library_save_cue(app: AppHandle, engine: State<'_, Engine>, id: String, replace: bool) -> CmdResult<String> {
    let service = engine.service().ok_or("no service is loaded")?;
    let cue = service.cue(&id).ok_or("no such cue")?;
    let doc = cue.document().ok_or("announcements cues cannot be saved to the library")?;
    let dir = user_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let name = slug_file_name(&doc.meta.title, ewm::EXTENSION);
    let path = dir.join(&name);
    if path.exists() && !replace {
        return Err(format!("exists:{}", doc.meta.title));
    }
    std::fs::write(&path, ewm::to_string(doc)).map_err(|e| format!("could not save {}: {e}", path.display()))?;
    Ok(format!("user:{name}"))
}

/// Deletes an item from My Library (built-in items cannot be deleted).
#[tauri::command]
pub fn library_delete(app: AppHandle, id: String) -> CmdResult<()> {
    let file = id.strip_prefix("user:").ok_or("built-in items cannot be deleted")?;
    if file.contains(['/', '\\']) || file.starts_with('.') {
        return Err("bad library id".into());
    }
    let path = user_dir(&app)?.join(file);
    std::fs::remove_file(&path).map_err(|e| format!("could not delete {}: {e}", path.display()))
}

/// The My Library folder, so the operator can open it in Finder.
#[tauri::command]
pub fn library_folder(app: AppHandle) -> CmdResult<PathBuf> {
    let dir = user_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

// ---- propers -------------------------------------------------------------

/// Finds the day, collect and readings in text pasted from the lectionary.
#[tauri::command]
pub fn parse_propers(text: String) -> Propers {
    propers::parse(&text)
}

/// Adds the chosen propers as cues after cue index `after`.
#[tauri::command]
pub fn add_propers(
    engine: State<'_, Engine>,
    items: Vec<ProperItem>,
    day: Option<String>,
    after: Option<usize>,
) -> CmdResult<Vec<String>> {
    let rite = engine.service().and_then(|s| s.rite).unwrap_or_default();
    let docs = items.iter().map(|i| propers::build_document_in(i, day.as_deref(), rite)).collect();
    insert_documents(&engine, docs, after)
}

/// Rebuilds a psalm cue's slides: antiphon (start, end, every N verses),
/// verses per slide. Verses alternate plain (choir) and bold (congregation).
/// With `verses_text`, the verses are replaced by those pasted.
#[tauri::command]
pub fn psalm_arrange(
    engine: State<'_, Engine>,
    id: String,
    options: propers::PsalmOptions,
    verses_text: Option<String>,
) -> CmdResult<usize> {
    engine.edit_service(|s| {
        let doc = s.cue(&id).and_then(|c| c.document()).ok_or("no such item")?.clone();
        let doc = &doc;
        let pasted = verses_text.as_deref().map(str::trim).filter(|t| !t.is_empty());
        let verses = match pasted {
            Some(text) => {
                let v = propers::parse_psalm_text(text);
                if v.is_empty() {
                    return Err(
                        "no numbered verses found in the pasted text (lines should start with 1, 2, 3 …)".to_string()
                    );
                }
                v
            }
            None => propers::psalm_verses(doc),
        };
        if verses.is_empty() {
            return Err("this psalm has no verses yet: paste them into “Paste verses” first".to_string());
        }
        let arranged = propers::rearrange_psalm_with(doc, &options, &verses);
        let count = verses.len();
        s.set_document(&id, arranged).map_err(|e| e.to_string())?;
        Ok(count)
    })
}

// ---- the lectionary --------------------------------------------------------

/// The propers for a date from the lectionary: the observances possible
/// that day and the chosen one's Collect (both rites) and readings (both
/// tracks), with psalm verses from the Prayer Book.
#[tauri::command]
pub fn lectionary_propers(date: String, observance: Option<String>) -> CmdResult<episco_core::lectionary::DayPropers> {
    let date: episco_core::calendar::Date = date.parse()?;
    Ok(episco_core::lectionary::for_date(date, observance.as_deref()))
}

/// Sets (or, with empty text, removes) the text of a reading from pasted
/// NRSV / NRSVue text, laid out about `words` words to a slide.
#[tauri::command]
pub fn reading_set_text(
    engine: State<'_, Engine>,
    id: String,
    text: String,
    version: episco_core::bible::BibleVersion,
    words: Option<usize>,
) -> CmdResult<usize> {
    engine.edit_service(|s| {
        let doc = s.cue(&id).and_then(|c| c.document()).ok_or("no such item")?;
        let new = episco_core::bible::set_reading_text(doc, &text, version, words.unwrap_or(40));
        let slides =
            new.blocks.iter().filter(|b| matches!(b, episco_core::Block::Paragraph { role: None, .. })).count();
        s.set_document(&id, new).map_err(|e| e.to_string())?;
        Ok(slides)
    })
}

/// The web page to copy a reading's text from.
#[tauri::command]
pub fn reading_url(citation: String, version: episco_core::bible::BibleVersion) -> String {
    version.passage_url(&citation)
}

/// Opens a reading's page on Bible Gateway / Oremus in the browser.
#[tauri::command]
pub fn open_passage(citation: String, version: episco_core::bible::BibleVersion) -> CmdResult<()> {
    let url = version.passage_url(&citation);
    let status = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(&url).status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/C", "start", "", &url]).status()
    } else {
        std::process::Command::new("xdg-open").arg(&url).status()
    };
    status.map(|_| ()).map_err(|e| format!("could not open the browser: {e}"))
}
