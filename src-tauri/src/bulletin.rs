//! Bulletins and officiant scripts: the preview, the Word files, and the
//! parish details printed on them (`parish.json` in the library folder).

use std::path::{Path, PathBuf};

use episco_core::bulletin::{self, BulletinStyle, Doc, DocKind, ParishInfo};
use tauri::{AppHandle, Emitter, State};

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

/// Emitted when the parish details change.
pub const EVENT: &str = "episco://parish";

const PARISH_FILE: &str = "parish.json";

fn parish_path(app: &AppHandle) -> CmdResult<PathBuf> {
    Ok(crate::storage::data_root(app)?.join(PARISH_FILE))
}

/// The parish details as saved: the logo relative to the library folder,
/// since the folder's path differs on each computer.
fn load_parish(app: &AppHandle) -> ParishInfo {
    parish_path(app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// The parish details as last read, for the stream's welcome slide.
static PARISH: std::sync::RwLock<Option<ParishInfo>> = std::sync::RwLock::new(None);

/// Reads the parish details (at start-up, and when they or the library
/// folder change).
pub fn init(app: &AppHandle) {
    *PARISH.write().unwrap_or_else(|e| e.into_inner()) = Some(load_parish(app));
}

/// The welcome slide for a service, with this parish's name.
pub fn welcome_for(service: &episco_core::Service) -> episco_core::bulletin::Welcome {
    let parish = PARISH.read().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default();
    episco_core::bulletin::welcome(service, &parish)
}

/// The parish details with the logo as a full path (if the file is there).
fn parish_resolved(app: &AppHandle) -> ParishInfo {
    let mut p = load_parish(app);
    let root = crate::storage::data_root(app).ok();
    p.logo = p.logo.and_then(|l| {
        let full = if l.is_absolute() { l } else { root.as_ref()?.join(l) };
        full.is_file().then_some(full)
    });
    p
}

#[tauri::command]
pub fn parish_get(app: AppHandle) -> ParishInfo {
    parish_resolved(&app)
}

/// Saves the parish details. A logo outside the library folder is copied in
/// (as `parish-logo.png` or `.jpg`) so every computer can find it.
#[tauri::command]
pub fn parish_set(app: AppHandle, engine: State<'_, Engine>, mut info: ParishInfo) -> CmdResult<ParishInfo> {
    let root = crate::storage::data_root(&app)?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    if let Some(logo) = info.logo.take() {
        let rel = match logo.strip_prefix(&root) {
            Ok(rel) => rel.to_path_buf(),
            Err(_) if logo.is_file() => {
                let ext = logo.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
                let name = PathBuf::from(format!("parish-logo.{ext}"));
                std::fs::copy(&logo, root.join(&name)).map_err(|e| format!("could not copy the logo: {e}"))?;
                name
            }
            Err(_) => logo,
        };
        info.logo = Some(rel);
    }
    let path = root.join(PARISH_FILE);
    std::fs::write(&path, serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?)
        .map_err(|e| format!("could not save {}: {e}", path.display()))?;
    let _ = app.emit(EVENT, ());
    init(&app);
    // The open service's welcome slide names the parish.
    if engine.service().is_some() {
        let _ = engine.edit_service(|_| Ok(()));
    }
    Ok(parish_resolved(&app))
}

/// Asks for a logo picture (PNG or JPEG: Word cannot take the others).
#[tauri::command]
pub async fn parish_pick_logo(app: AppHandle) -> Option<PathBuf> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .file()
        .set_title("Choose the parish logo")
        .add_filter("Picture", &["png", "jpg", "jpeg", "PNG", "JPG", "JPEG"])
        .blocking_pick_file()?
        .into_path()
        .ok()
}

fn build(app: &AppHandle, engine: &Engine, kind: DocKind) -> CmdResult<Doc> {
    let service = engine.service().ok_or("no service is open")?;
    let parish = parish_resolved(app);
    Ok(match kind {
        DocKind::Bulletin => bulletin::bulletin(&service, &parish),
        DocKind::Script => bulletin::script(&service, &parish),
    })
}

/// The bulletin or script of the open service, as HTML for the preview.
#[tauri::command]
pub fn bulletin_html(app: AppHandle, engine: State<'_, Engine>, kind: DocKind) -> CmdResult<String> {
    Ok(bulletin::to_html(&build(&app, &engine, kind)?))
}

/// "2025-11-09 0800 Bulletin".
fn file_stem(service: &episco_core::Service, kind: DocKind) -> String {
    let mut parts = Vec::new();
    if let Some(d) = &service.date {
        parts.push(d.clone());
    }
    if let Some(t) = &service.time {
        parts.push(t.replace(':', ""));
    }
    parts.push(match kind {
        DocKind::Bulletin => "Bulletin".into(),
        DocKind::Script => "Script".into(),
    });
    parts.join(" ")
}

fn open_file(path: &Path) {
    let r = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(path).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").arg("/C").arg("start").arg("").arg(path).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(path).spawn()
    };
    if let Err(e) = r {
        tracing::warn!("could not open {}: {e}", path.display());
    }
}

/// Saves the bulletin or script as a Word file (asking where, starting in
/// the library's `bulletins` folder) and opens it. Returns the path, or
/// `None` if cancelled.
#[tauri::command]
pub async fn bulletin_export(app: AppHandle, engine: State<'_, Engine>, kind: DocKind) -> CmdResult<Option<PathBuf>> {
    let doc = build(&app, &engine, kind)?;
    let service = engine.service().ok_or("no service is open")?;
    let bytes = episco_core::docx::to_docx(&doc)?;
    let dir = crate::storage::data_dir(&app, "bulletins")?;
    let _ = std::fs::create_dir_all(&dir);
    let stem = file_stem(&service, kind);
    let Some(path) =
        crate::commands::pick_save_path(&app, "Word document", "docx", &format!("{stem}.docx"), Some(&dir))
    else {
        return Ok(None);
    };
    std::fs::write(&path, bytes).map_err(|e| format!("could not save {}: {e}", path.display()))?;
    open_file(&path);
    Ok(Some(path))
}

/// Sets how an item appears in the bulletin (`None`: its default).
#[tauri::command]
pub fn set_cue_bulletin(engine: State<'_, Engine>, id: String, style: Option<BulletinStyle>) -> CmdResult<()> {
    engine.edit_service(|s| {
        let mut overrides = s.cue(&id).ok_or("no such cue")?.overrides.clone();
        overrides.bulletin = style;
        s.set_overrides(&id, overrides).map_err(|e| e.to_string())
    })
}

/// Sets the service's start time (`"08:00"`), or clears it.
#[tauri::command]
pub fn set_service_time(engine: State<'_, Engine>, time: Option<String>) -> CmdResult<()> {
    let time = time.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    if let Some(t) = &time {
        bulletin::time_phrase(t).ok_or_else(|| format!("“{t}” is not a time like 08:00 or 10:30"))?;
    }
    engine.edit_service(|s| {
        s.time = time;
        s.modified = true;
        Ok(())
    })
}

/// The style each item has by default and now, for the planner's list.
#[derive(serde::Serialize)]
pub struct CueBulletin {
    id: String,
    style: BulletinStyle,
    default: BulletinStyle,
    /// A reading or Gospel (its text can be pasted in).
    reading: bool,
    /// A reading whose text has not been added yet.
    needs_text: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    citation: Option<String>,
    /// The Bible version of the text it has ("NRSVue").
    #[serde(skip_serializing_if = "Option::is_none")]
    bible: Option<String>,
    /// What the item still needs before Sunday, in plain words.
    #[serde(skip_serializing_if = "Option::is_none")]
    todo: Option<String>,
    /// The Prayer Book's own text: it cannot be edited in the preview.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    protected: bool,
}

/// What an item still needs: a reading's text, a hymn chosen, the preacher.
pub fn todo(c: &episco_core::Cue) -> Option<String> {
    use episco_core::{Block, ElementCategory};
    let doc = c.document()?;
    if bulletin::needs_text(c) {
        return Some("Add the reading's text".into());
    }
    match c.category {
        ElementCategory::Hymn => {
            // Only the "Choose this hymn" note: no title, number or verses yet.
            let words = doc
                .blocks
                .iter()
                .any(|b| matches!(b, Block::Paragraph { role, .. } if role.as_ref().is_none_or(|r| !r.is_rubric())));
            (!words && doc.meta.hymn_number.is_none()).then(|| "Choose the hymn".into())
        }
        ElementCategory::Homily => {
            doc.meta.extra("preacher").is_none_or(|p| p.trim().is_empty()).then(|| "Name the preacher".into())
        }
        // An anthem added with no title yet (its title is the piece's own).
        ElementCategory::Anthem => {
            (doc.meta.title.trim().is_empty() && doc.meta.extra("composer").is_none()).then(|| "Name the anthem".into())
        }
        _ => None,
    }
}

#[tauri::command]
pub fn bulletin_styles(engine: State<'_, Engine>) -> Vec<CueBulletin> {
    engine
        .service()
        .map(|s| {
            s.cues
                .iter()
                .map(|c| CueBulletin {
                    id: c.id.clone(),
                    style: bulletin::style_of(c),
                    default: bulletin::default_style(c),
                    reading: matches!(
                        c.category,
                        episco_core::ElementCategory::Reading | episco_core::ElementCategory::Gospel
                    ),
                    needs_text: bulletin::needs_text(c),
                    citation: c.document().and_then(|d| d.meta.citation.clone()).filter(|x| !x.trim().is_empty()),
                    bible: c.document().and_then(|d| d.meta.extra("bible")).map(String::from),
                    todo: todo(c),
                    protected: c.document().is_some_and(|d| d.meta.is_prayer_book_text()),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Writes an edit typed into the bulletin or script preview back to the
/// item (so the slides change too).
#[tauri::command]
pub fn bulletin_edit(
    engine: State<'_, Engine>,
    id: String,
    block: usize,
    line: Option<usize>,
    edit: bulletin::LineEdit,
) -> CmdResult<()> {
    engine.edit_service(|s| {
        let doc = s.cue(&id).and_then(|c| c.document()).ok_or("no such item")?;
        let new = bulletin::edit_line(doc, block, line, edit)?;
        if &new != doc {
            s.set_document(&id, new).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
}

/// Renames an item from the preview (empty: back to the item's own title).
#[tauri::command]
pub fn bulletin_set_title(engine: State<'_, Engine>, id: String, title: String) -> CmdResult<()> {
    engine.edit_service(|s| {
        let cue = s.cue(&id).ok_or("no such item")?;
        let own = cue.document().map(|d| d.meta.title.clone()).unwrap_or_default();
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut overrides = cue.overrides.clone();
        overrides.title = (!title.is_empty() && title != own).then_some(title);
        if overrides != cue.overrides {
            s.set_overrides(&id, overrides).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_left_to_do() {
        let hymn = crate::services::hymn_cue("Entrance Hymn").unwrap();
        assert_eq!(todo(&hymn).as_deref(), Some("Choose the hymn"));
        let mut sermon = crate::services::sermon_cue();
        assert_eq!(todo(&sermon).as_deref(), Some("Name the preacher"));
        if let episco_core::CueContent::Liturgy { document, .. } = &mut sermon.content {
            document.meta.extra.push(("preacher".into(), "Jane Doe".into()));
        }
        assert_eq!(todo(&sermon), None);
        let chosen: episco_core::LiturgyDocument =
            "---\ntitle: Lift Up Your Heads\ntype: hymn\nhymn_number: 436\n---\n[1] Lift up your heads\n"
                .parse()
                .unwrap();
        let chosen = episco_core::Cue::liturgy("h", chosen, None, Default::default());
        assert_eq!(todo(&chosen), None);
    }
}
