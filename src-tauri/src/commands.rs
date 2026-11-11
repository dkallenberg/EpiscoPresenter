//! Tauri commands invoked from the web front-end.

use std::path::{Path, PathBuf};

use episco_core::ewm::{self, Parsed};
use episco_core::role::RoleStyle;
use episco_core::service::Loaded;
use episco_core::{
    AnnouncementMode, Command, Cue, CueOverrides, ElementCategory, LiturgicalColor, LiturgyDocument, Role,
    SanctuaryLayout, Season, Service, StreamZone,
};
use episco_sync::NodeConfig;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::engine::{Engine, Snapshot};

type CmdResult<T> = Result<T, String>;

// ---- node & presentation -------------------------------------------------

#[tauri::command]
pub fn get_node_config(config: State<'_, NodeConfig>) -> NodeConfig {
    config.inner().clone()
}

#[tauri::command]
pub fn get_snapshot(engine: State<'_, Engine>) -> Snapshot {
    engine.snapshot()
}

#[tauri::command]
pub fn send_command(engine: State<'_, Engine>, command: Command) -> CmdResult<()> {
    engine.send(command)
}

// ---- vocabulary ----------------------------------------------------------

#[derive(Serialize)]
pub struct RoleInfo {
    role: Role,
    congregational: bool,
    style: RoleStyle,
}

#[derive(Serialize)]
pub struct CategoryInfo {
    key: &'static str,
    stream_zone: StreamZone,
    sanctuary_layout: SanctuaryLayout,
}

#[derive(Serialize)]
pub struct ColorInfo {
    key: &'static str,
    swatch: &'static str,
    background: String,
}

#[derive(Serialize)]
pub struct SeasonInfo {
    /// Serialized value as used in documents (e.g. `HolyWeek`).
    key: Season,
    label: &'static str,
}

#[derive(Serialize)]
pub struct Vocabulary {
    roles: Vec<RoleInfo>,
    categories: Vec<CategoryInfo>,
    seasons: Vec<SeasonInfo>,
    colors: Vec<ColorInfo>,
}

/// Option lists for the editor's selectors, with their formatting.
#[tauri::command]
pub fn vocabulary() -> Vocabulary {
    Vocabulary {
        roles: Role::STANDARD
            .into_iter()
            .map(|role| RoleInfo { congregational: role.is_congregational(), style: role.style(), role })
            .collect(),
        categories: ElementCategory::ALL
            .into_iter()
            .map(|c| CategoryInfo {
                key: c.key(),
                stream_zone: c.stream_zone(),
                sanctuary_layout: c.sanctuary_layout(),
            })
            .collect(),
        seasons: Season::ALL.into_iter().map(|s| SeasonInfo { key: s, label: s.label() }).collect(),
        colors: LiturgicalColor::ALL
            .into_iter()
            .map(|c| ColorInfo { key: c.label(), swatch: c.swatch(), background: c.default_background() })
            .collect(),
    }
}

// ---- .ewm text -----------------------------------------------------------

/// Parses `.ewm` source from the editor.
#[tauri::command]
pub fn parse_ewm(source: String) -> CmdResult<Parsed> {
    ewm::parse(&source).map_err(|e| e.to_string())
}

/// Serialises a document to `.ewm` source.
#[tauri::command]
pub fn format_ewm(document: LiturgyDocument) -> String {
    ewm::to_string(&document)
}

// ---- dialogs -------------------------------------------------------------

/// Asks a yes/no question with a native dialog. Returns `true` for the
/// confirming button.
#[tauri::command]
pub async fn confirm(app: AppHandle, title: String, message: String, ok_label: String) -> bool {
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(ok_label, "Cancel".into()))
        .blocking_show()
}

fn pick_paths(app: &AppHandle, name: &str, ext: &str, multiple: bool) -> CmdResult<Vec<PathBuf>> {
    let dialog = app.dialog().file().add_filter(name, &[ext]);
    let picked = if multiple {
        dialog.blocking_pick_files().unwrap_or_default()
    } else {
        dialog.blocking_pick_file().into_iter().collect()
    };
    picked.into_iter().map(|p| p.into_path().map_err(|e| e.to_string())).collect()
}

fn pick_save_path(app: &AppHandle, name: &str, ext: &str, suggested: &str, dir: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = app.dialog().file().add_filter(name, &[ext]).set_file_name(suggested);
    if let Some(dir) = dir {
        dialog = dialog.set_directory(dir);
    }
    let path = dialog.blocking_save_file()?.into_path().ok()?;
    // Some platforms do not append the extension themselves.
    Some(if path.extension().is_none() { path.with_extension(ext) } else { path })
}

/// Picks an image for a sheet-music block. The path is made relative to the
/// item's `.ewm` file when the image lives beside or below it.
#[tauri::command]
pub async fn pick_image(app: AppHandle, relative_to: Option<PathBuf>) -> Option<String> {
    let mut dialog = app.dialog().file().add_filter("Image", &["png", "jpg", "jpeg", "gif", "svg", "webp"]);
    let base = relative_to.as_deref().and_then(Path::parent);
    if let Some(dir) = base {
        dialog = dialog.set_directory(dir);
    }
    let path = dialog.blocking_pick_file()?.into_path().ok()?;
    let shown = base.and_then(|b| path.strip_prefix(b).ok()).unwrap_or(&path);
    Some(shown.to_string_lossy().replace('\\', "/"))
}

// ---- service: load / new -------------------------------------------------

#[derive(Serialize)]
pub struct OpenResult {
    /// False if the user cancelled the file dialog.
    opened: bool,
    warnings: Vec<String>,
    /// Ids of cues added (for `add_items`).
    ids: Vec<String>,
}

impl OpenResult {
    fn cancelled() -> Self {
        OpenResult { opened: false, warnings: Vec::new(), ids: Vec::new() }
    }
}

/// Opens a service plan (`.toml`), from `path` or a file dialog, and makes it live.
#[tauri::command]
pub async fn open_service(app: AppHandle, engine: State<'_, Engine>, path: Option<PathBuf>) -> CmdResult<OpenResult> {
    let Some(path) = path.or_else(|| pick_paths(&app, "Service plan", "toml", false).ok()?.pop()) else {
        return Ok(OpenResult::cancelled());
    };
    let Loaded { service, warnings } = Service::load_plan(&path).map_err(|e| e.to_string())?;
    engine.load_service(Some(service))?;
    Ok(OpenResult { opened: true, warnings, ids: Vec::new() })
}

/// Starts an empty service.
#[tauri::command]
pub fn new_service(engine: State<'_, Engine>) -> CmdResult<()> {
    engine.load_service(Some(Service { title: "New Service".into(), ..Default::default() }))
}

const EXAMPLE_PLAN: &str = include_str!("../../examples/service.toml");
const EXAMPLE_FILES: &[(&str, &str)] = &[
    ("liturgies/hymn-436.ewm", include_str!("../../examples/liturgies/hymn-436.ewm")),
    ("liturgies/opening-acclamation.ewm", include_str!("../../examples/liturgies/opening-acclamation.ewm")),
    ("liturgies/first-reading.ewm", include_str!("../../examples/liturgies/first-reading.ewm")),
    ("liturgies/psalm-78.ewm", include_str!("../../examples/liturgies/psalm-78.ewm")),
    ("liturgies/sursum-corda.ewm", include_str!("../../examples/liturgies/sursum-corda.ewm")),
];

/// Loads the example service compiled into the app (works on any machine).
#[tauri::command]
pub fn load_example_service(engine: State<'_, Engine>) -> CmdResult<Vec<String>> {
    let Loaded { service, warnings } = Service::from_plan_with(EXAMPLE_PLAN, |file| {
        let key = file.to_string_lossy().replace('\\', "/");
        EXAMPLE_FILES
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, src)| (src.to_string(), None))
            .ok_or_else(|| format!("example file {key} is not embedded"))
    })
    .map_err(|e| e.to_string())?;
    engine.load_service(Some(service))?;
    Ok(warnings)
}

// ---- service: cues -------------------------------------------------------

/// Adds `.ewm` files (from `paths`, or a multi-select dialog) after cue index `after`.
#[tauri::command]
pub async fn add_items(
    app: AppHandle,
    engine: State<'_, Engine>,
    after: Option<usize>,
    paths: Option<Vec<PathBuf>>,
) -> CmdResult<OpenResult> {
    let paths = match paths {
        Some(p) => p,
        None => pick_paths(&app, "Liturgy (.ewm)", ewm::EXTENSION, true)?,
    };
    if paths.is_empty() {
        return Ok(OpenResult::cancelled());
    }
    let mut parsed = Vec::with_capacity(paths.len());
    let mut warnings = Vec::new();
    for path in paths {
        let p = ewm::load_file(&path).map_err(|e| e.to_string())?;
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        warnings.extend(p.warnings.iter().map(|w| format!("{name}:{}: {}", w.line, w.message)));
        parsed.push((path, p.document));
    }
    let ids = engine.edit_service(|s| {
        let start = after.map(|a| a + 1).unwrap_or(s.cues.len());
        let mut ids = Vec::new();
        for (at, (path, doc)) in (start..).zip(parsed) {
            let stem = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let cue = Cue::liturgy(s.unique_id(&stem), doc, Some(path), CueOverrides::default(), s.color);
            ids.push(s.insert_cue(at, cue));
        }
        Ok(ids)
    })?;
    Ok(OpenResult { opened: true, warnings, ids })
}

/// Starter text for a new item of each type.
fn template(category: ElementCategory) -> &'static str {
    match category {
        ElementCategory::Hymn => "---\ntitle: New Hymn\ntype: hymn\nhymn_number: \n---\n\n# Verse 1\nFirst line of the verse\n",
        ElementCategory::Reading | ElementCategory::Gospel => {
            "---\ntitle: New Reading\ntype: reading\ncitation: \n---\n\n[Reader] A Reading from …\n\n[Reader] The Word of the Lord.\n[People] **Thanks be to God.**\n"
        }
        ElementCategory::Psalm => {
            "---\ntitle: Psalm\ntype: psalm\ncitation: \n---\n\n[Reader] First half-verse; *\nsecond half-verse.\n[People] **First half-verse;** *\n**second half-verse.**\n"
        }
        ElementCategory::Collect => {
            "---\ntitle: The Collect of the Day\ntype: collect\n---\n\n[Celebrant] The Lord be with you.\n[People] **And also with you.**\n[Celebrant] Let us pray.\n\n[Celebrant] Almighty God, …\n[People] **Amen.**\n"
        }
        _ => "---\ntitle: New Item\n---\n\n[Celebrant] \n[People] ****\n",
    }
}

/// Inserts a new, unsaved item after cue index `after` and returns its id.
#[tauri::command]
pub fn new_item(engine: State<'_, Engine>, after: Option<usize>, category: ElementCategory) -> CmdResult<String> {
    engine.edit_service(|s| {
        let at = after.map(|a| a + 1).unwrap_or(s.cues.len());
        let cue = if category == ElementCategory::Announcements {
            Cue::announcements("announcements", AnnouncementMode::SinglePass, CueOverrides::default(), s.color)
        } else {
            let mut doc: LiturgyDocument = template(category).parse().map_err(|e| format!("{e}"))?;
            doc.meta.category = Some(category);
            let mut cue = Cue::liturgy("", doc, None, CueOverrides::default(), s.color);
            cue.modified = true;
            cue
        };
        Ok(s.insert_cue(at, cue))
    })
}

/// Replaces a cue's document (live: outputs update immediately).
#[tauri::command]
pub fn update_cue_document(engine: State<'_, Engine>, id: String, document: LiturgyDocument) -> CmdResult<()> {
    engine.edit_service(|s| s.set_document(&id, document).map_err(|e| e.to_string()))
}

#[tauri::command]
pub fn set_cue_overrides(engine: State<'_, Engine>, id: String, overrides: CueOverrides) -> CmdResult<()> {
    engine.edit_service(|s| s.set_overrides(&id, overrides).map_err(|e| e.to_string()))
}

#[tauri::command]
pub fn set_announcement_mode(engine: State<'_, Engine>, id: String, mode: AnnouncementMode) -> CmdResult<()> {
    engine.edit_service(|s| {
        let i = s.index_of(&id).ok_or("no such cue")?;
        match &mut s.cues[i].content {
            episco_core::CueContent::Announcements { mode: m } => *m = mode,
            _ => return Err("not an announcements cue".to_string()),
        }
        s.modified = true;
        Ok(())
    })
}

#[tauri::command]
pub fn move_cue(engine: State<'_, Engine>, id: String, to: usize) -> CmdResult<()> {
    engine.edit_service(|s| s.move_cue(&id, to).map_err(|e| e.to_string()))
}

#[tauri::command]
pub fn remove_cue(engine: State<'_, Engine>, id: String) -> CmdResult<()> {
    engine.edit_service(|s| s.remove_cue(&id).map(drop).map_err(|e| e.to_string()))
}

// ---- saving --------------------------------------------------------------

fn slug_file_name(title: &str, ext: &str) -> String {
    let stem: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!("{}.{ext}", if stem.is_empty() { "untitled" } else { &stem })
}

fn write_cue(path: &Path, document: &LiturgyDocument) -> CmdResult<()> {
    std::fs::write(path, ewm::to_string(document)).map_err(|e| format!("could not save {}: {e}", path.display()))
}

/// Saves a cue's document to its `.ewm` file (asking for a path if it has
/// none, or if `save_as`). Returns the path, or `None` if cancelled.
#[tauri::command]
pub async fn save_cue(
    app: AppHandle,
    engine: State<'_, Engine>,
    id: String,
    save_as: bool,
) -> CmdResult<Option<PathBuf>> {
    let service = engine.service().ok_or("no service is loaded")?;
    let cue = service.cue(&id).ok_or("no such cue")?;
    let document = cue.document().ok_or("announcements cues have no file")?;
    let plan_dir = service.plan_path.as_deref().and_then(Path::parent);
    let path = match cue.source() {
        Some(p) if !save_as => p.to_path_buf(),
        existing => {
            let dir = existing.and_then(Path::parent).or(plan_dir);
            match pick_save_path(
                &app,
                "Liturgy (.ewm)",
                ewm::EXTENSION,
                &slug_file_name(&document.meta.title, "ewm"),
                dir,
            ) {
                Some(p) => p,
                None => return Ok(None),
            }
        }
    };
    write_cue(&path, document)?;
    let saved = path.clone();
    engine.edit_service(|s| s.mark_cue_saved(&id, saved).map_err(|e| e.to_string()))?;
    Ok(Some(path))
}

/// Saves every modified item that already has a file, then the plan itself
/// (asking for a path if it has none, or if `save_as`). Items that have never
/// been saved must be saved individually first. Returns the plan path, or
/// `None` if cancelled.
#[tauri::command]
pub async fn save_service(app: AppHandle, engine: State<'_, Engine>, save_as: bool) -> CmdResult<Option<PathBuf>> {
    let service = engine.service().ok_or("no service is loaded")?;
    if let Some(unsaved) = service.cues.iter().find(|c| c.document().is_some() && c.source().is_none()) {
        return Err(format!("“{}” has never been saved; save it to an .ewm file first", unsaved.title));
    }
    let path = match &service.plan_path {
        Some(p) if !save_as => p.clone(),
        existing => {
            let dir = existing.as_deref().and_then(Path::parent);
            match pick_save_path(&app, "Service plan", "toml", &slug_file_name(&service.title, "toml"), dir) {
                Some(p) => p,
                None => return Ok(None),
            }
        }
    };
    let toml = service.to_plan_toml(&path).map_err(|e| e.to_string())?;
    for cue in service.cues.iter().filter(|c| c.modified) {
        if let (Some(src), Some(doc)) = (cue.source(), cue.document()) {
            write_cue(src, doc)?;
        }
    }
    std::fs::write(&path, toml).map_err(|e| format!("could not save {}: {e}", path.display()))?;
    let saved = path.clone();
    engine.edit_service(|s| {
        s.plan_path = Some(saved);
        s.modified = false;
        for c in &mut s.cues {
            c.modified = false;
        }
        Ok(())
    })?;
    Ok(Some(path))
}
