//! Editing a Library item on its own (Library → Edit, or ＋ New): the item is
//! opened in the Editor as a service of one item, the way a template is
//! edited, so every panel (hymn words, verses, a picture of the hymn, the
//! psalm layout, speaker rows) works on it. Save to Library writes it back
//! to its file; Done returns to the service that was open before, if any.
//!
//! Services already made keep their own copy of an item. After saving, the
//! upcoming services that use it are listed, and the ones chosen are given
//! the new version.

use std::path::PathBuf;
use std::sync::Mutex;

use episco_core::document::FrontMatter;
use episco_core::service::Loaded;
use episco_core::{Block, Cue, CueOverrides, ElementCategory, LiturgyDocument, Service, Span};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::engine::Engine;
use crate::library::{library_copy_of, load, today, user_dir, user_file, BASED_ON};

type CmdResult<T> = Result<T, String>;

/// The id of the one item in the editing "service".
pub const EDIT_CUE: &str = "library-item";

/// The item being edited: its file in the Library (none until a new item
/// is first saved) and the service set aside meanwhile.
struct Editing {
    file: Option<String>,
    previous: Option<PathBuf>,
}

static EDITING: Mutex<Option<Editing>> = Mutex::new(None);

/// A new item, from ＋ New in the Library.
#[derive(Debug, Deserialize)]
pub struct NewItem {
    /// `hymn`, `prayer`, `collect` … (an item type's key).
    category: String,
    title: String,
    #[serde(default)]
    hymn_number: Option<String>,
    /// The hymn's book (`wlp` …); none: the Hymnal 1982.
    #[serde(default)]
    hymnal: Option<String>,
    /// "I" or "II" for a text that differs between the rites.
    #[serde(default)]
    rite: Option<String>,
}

fn new_document(item: &NewItem) -> LiturgyDocument {
    let category = item.category.parse::<ElementCategory>().unwrap_or(ElementCategory::Prayer);
    let mut meta = FrontMatter { title: item.title.trim().to_string(), category: Some(category), ..Default::default() };
    let some = |v: &Option<String>| v.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(String::from);
    if category == ElementCategory::Hymn {
        meta.hymn_number = some(&item.hymn_number);
        if let Some(book) = some(&item.hymnal).filter(|b| b != "h82") {
            meta.extra.push((episco_core::hymn::HYMNAL.into(), book));
        }
        // Empty: the Editor's Hymn words box opens for the words.
        return LiturgyDocument { meta, blocks: Vec::new() };
    }
    if let Some(rite) = some(&item.rite) {
        meta.extra.push(("rite".into(), rite));
    }
    LiturgyDocument { meta, blocks: vec![Block::Paragraph { role: None, lines: vec![vec![Span::plain("")]] }] }
}

/// Opens a Library item (`id`, one of the parish's) or a `new` one in the
/// Editor, setting aside the open service (saved first).
#[tauri::command]
pub fn library_edit_open(
    app: AppHandle,
    engine: State<'_, Engine>,
    id: Option<String>,
    new: Option<NewItem>,
) -> CmdResult<()> {
    let (doc, file) = match (&id, &new) {
        (Some(id), _) => {
            if id.starts_with("builtin:") {
                return Err("The Prayer Book's own text can't be changed: make a parish version of it first.".into());
            }
            (load(&app, id)?, Some(user_file(id)?.to_string()))
        }
        (None, Some(item)) => {
            if item.title.trim().is_empty() && item.hymn_number.as_deref().is_none_or(|n| n.trim().is_empty()) {
                return Err("Give the new item a title.".into());
            }
            (new_document(item), None)
        }
        (None, None) => return Err("nothing to edit".into()),
    };
    // The open service is saved and set aside (unless it is already an item
    // being edited).
    let mut editing = EDITING.lock().unwrap_or_else(|e| e.into_inner());
    // Still editing an item (not left by ☰ Services): keep what was set aside.
    let still = engine.service().is_some_and(|s| s.plan_path.is_none() && s.cue(EDIT_CUE).is_some());
    let previous = match editing.take().filter(|_| still) {
        Some(e) => e.previous,
        None => match engine.service().and_then(|s| s.plan_path.clone()) {
            Some(path) => {
                engine.edit_service(|s| {
                    if s.has_unsaved_changes() {
                        s.save_to(&path).map_err(|e| e.to_string())
                    } else {
                        Ok(())
                    }
                })?;
                Some(path)
            }
            None => None,
        },
    };
    let title = if doc.meta.title.trim().is_empty() {
        episco_core::hymn::number_label(&doc.meta).unwrap_or_else(|| "New item".into())
    } else {
        doc.meta.title.clone()
    };
    let mut service = Service { title, ..Default::default() };
    service.insert_cue(0, Cue::liturgy(EDIT_CUE, doc, None, CueOverrides::default()));
    service.modified = false;
    engine.load_service(Some(service))?;
    *editing = Some(Editing { file, previous });
    Ok(())
}

/// A service that uses a Library item.
#[derive(Debug, Serialize)]
pub struct ServiceUse {
    path: PathBuf,
    title: String,
    date: Option<String>,
    time: Option<String>,
    /// The items in it that are this one ("Entrance Hymn").
    items: Vec<String>,
    /// Open on someone else's computer: it can't be changed now.
    #[serde(skip_serializing_if = "Option::is_none")]
    open_on: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Saved {
    /// The item's Library id (`user:…`).
    id: String,
    /// Upcoming services that use it.
    uses: Vec<ServiceUse>,
}

/// Saves the item being edited to the Library, and lists the upcoming
/// services that use it.
#[tauri::command]
pub fn library_edit_save(app: AppHandle, engine: State<'_, Engine>) -> CmdResult<Saved> {
    crate::sync::soon();
    let service = engine.service().ok_or("nothing is being edited")?;
    let doc = service.cue(EDIT_CUE).and_then(|c| c.document()).ok_or("nothing is being edited")?.clone();
    let dir = user_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let mut editing = EDITING.lock().unwrap_or_else(|e| e.into_inner());
    let state = editing.as_mut().ok_or("nothing is being edited")?;
    let (doc, file) = match &state.file {
        Some(file) => (doc, file.clone()),
        None => {
            // A new item: named for its number and title, never over another.
            let (doc, name) = library_copy_of(&doc)?;
            let stem = name.trim_end_matches(&format!(".{}", episco_core::ewm::EXTENSION)).to_string();
            let mut name = name;
            let mut n = 2;
            while dir.join(&name).exists() {
                name = format!("{stem}-{n}.{}", episco_core::ewm::EXTENSION);
                n += 1;
            }
            (doc, name)
        }
    };
    let path = dir.join(&file);
    std::fs::write(&path, episco_core::ewm::to_string(&doc))
        .map_err(|e| format!("could not save {}: {e}", path.display()))?;
    state.file = Some(file.clone());
    engine.edit_service(|s| {
        s.title = doc.meta.title.clone();
        s.modified = false;
        s.cues.iter_mut().for_each(|c| c.modified = false);
        Ok(())
    })?;
    let id = format!("user:{file}");
    let uses = uses_of(&app, &doc, state.previous.as_deref())?;
    Ok(Saved { id, uses })
}

/// Back to the service set aside (or none).
#[tauri::command]
pub fn library_edit_done(engine: State<'_, Engine>) -> CmdResult<()> {
    let previous = EDITING.lock().unwrap_or_else(|e| e.into_inner()).take().and_then(|e| e.previous);
    match previous {
        Some(path) => match Service::load_plan(&path) {
            Ok(Loaded { service, .. }) => engine.load_service(Some(service)),
            Err(_) => engine.load_service(None),
        },
        None => engine.load_service(None),
    }
}

/// The upcoming services that use a Library item.
#[tauri::command]
pub fn library_item_uses(app: AppHandle, id: String) -> CmdResult<Vec<ServiceUse>> {
    let doc = load(&app, &id)?;
    uses_of(&app, &doc, None)
}

/// Gives the services chosen (their plans) the Library's version of an item.
/// Returns how many were changed.
#[tauri::command]
pub fn library_update_services(app: AppHandle, id: String, paths: Vec<PathBuf>) -> CmdResult<usize> {
    let doc = load(&app, &id)?;
    let known: Vec<PathBuf> = crate::services::list_services(&app)?.into_iter().map(|s| s.path).collect();
    let mut changed = 0;
    for path in paths {
        // Only the app's own services, and none open on another computer.
        if !known.contains(&path) || open_elsewhere(&path).is_some() {
            continue;
        }
        let Ok(Loaded { mut service, .. }) = Service::load_plan(&path) else { continue };
        let ids: Vec<String> = service
            .cues
            .iter()
            .filter(|c| c.document().is_some_and(|d| same_item(d, &doc)))
            .map(|c| c.id.clone())
            .collect();
        if ids.is_empty() {
            continue;
        }
        for cue in &ids {
            let _ = service.set_document(cue, doc.clone());
        }
        service.save_to(&path).map_err(|e| e.to_string())?;
        changed += 1;
    }
    Ok(changed)
}

fn open_elsewhere(plan: &std::path::Path) -> Option<String> {
    crate::inuse::holder(plan).filter(|m| m.session != crate::inuse::session()).map(|m| m.name)
}

fn uses_of(app: &AppHandle, doc: &LiturgyDocument, also: Option<&std::path::Path>) -> CmdResult<Vec<ServiceUse>> {
    let today = today();
    let mut out = Vec::new();
    for s in crate::services::list_services(app)? {
        let upcoming = s.date.as_deref().is_some_and(|d| d >= today.as_str());
        if !upcoming && also != Some(s.path.as_path()) {
            continue;
        }
        let Ok(Loaded { service, .. }) = Service::load_plan(&s.path) else { continue };
        let items: Vec<String> = service
            .cues
            .iter()
            .filter(|c| c.document().is_some_and(|d| same_item(d, doc)))
            .map(|c| c.title.clone())
            .collect();
        if items.is_empty() {
            continue;
        }
        out.push(ServiceUse {
            open_on: open_elsewhere(&s.path),
            path: s.path,
            title: s.title,
            date: s.date,
            time: s.time,
            items,
        });
    }
    Ok(out)
}

/// The same item: a hymn by its number and book (else its title); another
/// item by its kind and title (ignoring case, punctuation and "The"), and
/// for a parish version, the Prayer Book item it is a version of.
pub fn same_item(a: &LiturgyDocument, b: &LiturgyDocument) -> bool {
    let (ca, cb) = (a.meta.effective_category(), b.meta.effective_category());
    if ca != cb {
        return false;
    }
    let norm = |t: &str| {
        let t: String = t.to_lowercase().chars().filter(|c| c.is_alphanumeric() || c.is_whitespace()).collect();
        let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
        t.strip_prefix("the ").map(String::from).unwrap_or(t)
    };
    if ca == ElementCategory::Hymn {
        let (na, nb) = (episco_core::hymn::number_label(&a.meta), episco_core::hymn::number_label(&b.meta));
        if na.is_some() || nb.is_some() {
            return na == nb;
        }
    }
    let (ta, tb) = (norm(&a.meta.title), norm(&b.meta.title));
    if ta.is_empty() || ta != tb {
        return false;
    }
    match (a.meta.extra(BASED_ON), b.meta.extra(BASED_ON)) {
        (Some(x), Some(y)) => x == y,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> LiturgyDocument {
        text.parse().unwrap()
    }

    #[test]
    fn the_same_item_in_a_service() {
        let hymn = doc("---\ntitle: Lift Up Your Heads\ntype: hymn\nhymn_number: 436\n---\n[1] Lift up your heads\n");
        // A hymn by its number and book, whatever its title or place.
        assert!(same_item(&doc("---\ntitle: Entrance Hymn\ntype: hymn\nhymn_number: 436\n---\n"), &hymn));
        assert!(!same_item(
            &doc("---\ntitle: Lift Up Your Heads\ntype: hymn\nhymn_number: 436\nhymnal: wlp\n---\n"),
            &hymn
        ));
        assert!(!same_item(&doc("---\ntitle: Entrance Hymn\ntype: hymn\n---\n_Choose this hymn_\n"), &hymn));
        // Another item by its kind and title.
        let collect = doc("---\ntitle: The Collect for Purity\ntype: collect\n---\n[Celebrant] Almighty God\n");
        assert!(same_item(&doc("---\ntitle: Collect for purity\ntype: collect\n---\n"), &collect));
        assert!(!same_item(&doc("---\ntitle: Collect for Purity\ntype: prayer\n---\n"), &collect));
    }

    #[test]
    fn a_new_hymn_or_text() {
        let hymn = new_document(&NewItem {
            category: "hymn".into(),
            title: "Lift Up Your Heads".into(),
            hymn_number: Some("436".into()),
            hymnal: Some("h82".into()),
            rite: None,
        });
        assert_eq!(hymn.meta.hymn_number.as_deref(), Some("436"));
        assert_eq!(hymn.meta.extra(episco_core::hymn::HYMNAL), None, "the Hymnal 1982 is the usual book");
        assert!(hymn.blocks.is_empty());
        let wlp = new_document(&NewItem {
            category: "hymn".into(),
            title: "You are mine".into(),
            hymn_number: Some("758".into()),
            hymnal: Some("wlp".into()),
            rite: None,
        });
        assert_eq!(episco_core::hymn::number_label(&wlp.meta).as_deref(), Some("WLP 758"));
        let prayer = new_document(&NewItem {
            category: "prayer".into(),
            title: "A Prayer for the Parish".into(),
            hymn_number: None,
            hymnal: None,
            rite: Some("II".into()),
        });
        assert_eq!(prayer.meta.extra("rite"), Some("II"));
        assert_eq!(prayer.blocks.len(), 1);
    }
}
