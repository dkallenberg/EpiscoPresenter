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
//!
//! **Versions.** A built-in item is the Prayer Book's text ("BCP"). The
//! parish can make copies of it (each labelled with the day it was made),
//! edit them, and set one as the **default**: new services, and adding the
//! item from the Library, then use the parish's text. A copy is a My Library
//! file whose front matter says what it is a copy of (`based_on`) and when it
//! was made (`created`); the defaults are in `defaults.json` in the library
//! folder.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use episco_core::ewm;
use episco_core::propers::{self, ProperItem, Propers};
use episco_core::{Cue, CueOverrides, LiturgyDocument};
use serde::Serialize;
use tauri::{AppHandle, State};

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
    "eucharistic-prayer-a",
    "eucharistic-prayer-b",
    "eucharistic-prayer-c",
    "eucharistic-prayer-d",
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
    "prayers-of-the-people-1",
    "prayers-of-the-people-2",
    "prayers-of-the-people-3",
    "prayers-of-the-people-4",
    "prayers-of-the-people-5",
    "prayers-of-the-people-6",
    "prayers-of-the-people-collects",
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
    "rite1/eucharistic-prayer-1",
    "rite1/eucharistic-prayer-2",
    "rite1/lords-prayer",
    "rite1/fraction-anthem",
    "rite1/agnus-dei",
    "rite1/humble-access",
    "rite1/invitation",
    "rite1/postcommunion",
    "rite1/blessing",
    "rite1/prayers-of-the-people",
    // Holy Baptism, Confirmation, the bishop's parts, Holy Week and
    // ordinations (BCP 264-311, 413-419, 512-551).
    "occasions/ash-wednesday-ashes",
    "occasions/ash-wednesday-invitation",
    "occasions/baptism-acclamation",
    "occasions/baptism-chrism",
    "occasions/baptism-confirmation",
    "occasions/baptism-prayers",
    "occasions/baptism-presentation",
    "occasions/baptism-the-baptism",
    "occasions/baptism-water",
    "occasions/baptismal-covenant",
    "occasions/confirmation",
    "occasions/confirmation-prayers",
    "occasions/confirmation-presentation",
    "occasions/episcopal-blessing",
    "occasions/exsultet",
    "occasions/foot-washing",
    "occasions/good-friday-anthems",
    "occasions/good-friday-final-prayer",
    "occasions/good-friday-opening",
    "occasions/good-friday-passion",
    "occasions/in-the-midst-of-life",
    "occasions/litany-for-ordinations",
    "occasions/litany-of-penitence",
    "occasions/liturgy-of-the-palms",
    "occasions/ordination-bishop-after-communion",
    "occasions/ordination-bishop-consecration",
    "occasions/ordination-bishop-examination",
    "occasions/ordination-bishop-opening",
    "occasions/ordination-bishop-presentation",
    "occasions/ordination-collect",
    "occasions/ordination-deacon-after-communion",
    "occasions/ordination-deacon-consecration",
    "occasions/ordination-deacon-examination",
    "occasions/ordination-deacon-opening",
    "occasions/ordination-deacon-presentation",
    "occasions/ordination-priest-after-communion",
    "occasions/ordination-priest-consecration",
    "occasions/ordination-priest-examination",
    "occasions/ordination-priest-opening",
    "occasions/ordination-priest-presentation",
    "occasions/passion-announcement",
    "occasions/renewal-of-vows",
    "occasions/solemn-collects",
    "occasions/vigil-first-eucharist",
    "occasions/vigil-liturgy-of-the-word",
    "occasions/vigil-service-of-light",
];

#[derive(Clone, Serialize, serde::Deserialize)]
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
    /// For a parish copy: the built-in item it is a version of.
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<String>,
    /// For a parish copy: the day it was made (YYYY-MM-DD).
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<String>,
    /// This version is what services use (for a built-in item: no parish
    /// copy is the default).
    default: bool,
    /// First words of the text, for the list.
    summary: String,
    slides: usize,
    /// What the parish uses the item for in a service (`prayers_of_the_people`):
    /// it is then offered with the Prayer Book's forms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    uses: Option<String>,
    /// The parish's other items that are the same hymn (same number) or
    /// the same item (same kind and title): copies to merge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    dupes: Vec<String>,
}

fn user_dir(app: &AppHandle) -> CmdResult<PathBuf> {
    let dir = crate::storage::data_dir(app, "library")?;
    Ok(dir)
}

fn entry(id: String, doc: &LiturgyDocument, builtin: bool) -> LibraryEntry {
    let base = if builtin { None } else { doc.meta.extra(BASED_ON).map(String::from) };
    let created = base.as_ref().and(doc.meta.extra(CREATED)).map(String::from);
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
        citation: doc.meta.citation.clone().or_else(|| doc.meta.hymn_number.as_ref().map(|n| format!("Hymn {n}"))),
        season: doc.meta.season.map(|s| s.label().to_string()),
        rite: doc.meta.extra("rite").map(String::from),
        builtin,
        base,
        created,
        default: false,
        summary,
        slides: doc.slides().len(),
        uses: doc.meta.extra(USES).map(String::from),
        dupes: Vec::new(),
    }
}

/// What makes two of the parish's items the same: a hymn's number, else
/// the kind and title (ignoring case, punctuation and "The"). Versions of a
/// Prayer Book item are meant to be several, so they have none.
fn same_key(e: &LibraryEntry) -> Option<String> {
    if e.builtin || e.base.is_some() {
        return None;
    }
    let norm = |t: &str| {
        let t = t.trim().to_lowercase();
        let t = t.strip_prefix("the ").unwrap_or(&t).to_string();
        t.chars().filter(|c| c.is_alphanumeric()).collect::<String>()
    };
    let number = (e.category == "hymn")
        .then(|| e.citation.as_deref()?.strip_prefix("Hymn ").map(norm))
        .flatten()
        .filter(|n| !n.is_empty());
    Some(match number {
        Some(n) => format!("hymn#{n}"),
        None => format!("{}:{}", e.category, norm(&e.title)),
    })
}

/// Marks the parish's items that are in the library more than once.
fn mark_dupes(entries: &mut [LibraryEntry]) {
    let keys: Vec<Option<String>> = entries.iter().map(same_key).collect();
    let ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
    for (i, e) in entries.iter_mut().enumerate() {
        let Some(k) = &keys[i] else { continue };
        e.dupes = keys
            .iter()
            .enumerate()
            .filter(|(j, o)| *j != i && o.as_ref() == Some(k))
            .map(|(j, _)| ids[j].clone())
            .collect();
    }
}

// ---- versions -----------------------------------------------------------

/// Front matter: the built-in item a parish copy is a version of.
pub const BASED_ON: &str = "based_on";
/// Front matter: the day a parish copy was made.
pub const CREATED: &str = "created";
const DEFAULTS: &str = "defaults.json";

/// Built-in name → the My Library file that is its default version.
fn read_defaults(dir: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(dir.join(DEFAULTS)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn write_defaults(dir: &Path, map: &BTreeMap<String, String>) -> CmdResult<()> {
    let path = dir.join(DEFAULTS);
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("could not save {}: {e}", path.display()))
}

/// The Prayer Book's text of a built-in item, marked with what it is.
fn bcp_doc(name: &str) -> Option<LiturgyDocument> {
    let (_, src) = BUILTIN.iter().find(|(n, _)| *n == name)?;
    let mut doc = ewm::parse(src).ok()?.document;
    doc.meta.extra.push((BASED_ON.into(), name.into()));
    Some(doc)
}

/// The parish's default version of a built-in item, if it set one.
fn default_doc(dir: &Path, name: &str) -> Option<LiturgyDocument> {
    let file = read_defaults(dir).remove(name)?;
    match ewm::load_file(dir.join(&file)) {
        Ok(p) => Some(p.document),
        Err(e) => {
            tracing::warn!("default version of {name} ({file}) could not be read: {e}");
            None
        }
    }
}

fn today() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    episco_core::calendar::Date::from_days((secs / 86_400) as i64).to_string()
}

/// A file name for a new copy of `name` made on `date`.
fn version_file(dir: &Path, name: &str, date: &str) -> String {
    let stem = format!("{}--{date}", name.replace('/', "_"));
    (1..)
        .map(|n| if n == 1 { format!("{stem}.{}", ewm::EXTENSION) } else { format!("{stem}-{n}.{}", ewm::EXTENSION) })
        .find(|f| !dir.join(f).exists())
        .expect("unbounded")
}

/// Saves `doc` as a new parish copy of built-in item `name`.
fn save_version(dir: &Path, name: &str, mut doc: LiturgyDocument) -> CmdResult<String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let date = today();
    doc.meta.extra.retain(|(k, _)| k != BASED_ON && k != CREATED);
    doc.meta.extra.push((BASED_ON.into(), name.into()));
    doc.meta.extra.push((CREATED.into(), date.clone()));
    let file = version_file(dir, name, &date);
    let path = dir.join(&file);
    std::fs::write(&path, ewm::to_string(&doc)).map_err(|e| format!("could not save {}: {e}", path.display()))?;
    Ok(format!("user:{file}"))
}

fn user_file(id: &str) -> CmdResult<&str> {
    let file = id.strip_prefix("user:").ok_or("bad library id")?;
    if file.contains(['/', '\\']) || file.starts_with('.') {
        return Err("bad library id".into());
    }
    Ok(file)
}

/// Makes a copy of a built-in item (or of one of its copies) for the
/// parish to edit. Returns the copy's id.
#[tauri::command]
pub fn library_copy(app: AppHandle, id: String) -> CmdResult<String> {
    crate::sync::soon();
    let doc = load(&app, &id)?;
    let name = doc.meta.extra(BASED_ON).map(String::from).ok_or("only Prayer Book items have versions")?;
    save_version(&user_dir(&app)?, &name, doc)
}

/// Turns an item that is the Prayer Book's text into a new dated parish
/// version (saved in the library), which can then be edited. Returns the
/// version's library id.
#[tauri::command]
pub fn cue_make_version(app: AppHandle, engine: State<'_, Engine>, id: String) -> CmdResult<String> {
    crate::sync::soon();
    let service = engine.service().ok_or("no service is loaded")?;
    let doc = service.cue(&id).and_then(|c| c.document()).ok_or("no such item")?.clone();
    if !doc.meta.is_prayer_book_text() {
        return Err("this item can already be edited".into());
    }
    let name = doc.meta.extra(BASED_ON).unwrap_or_default().to_string();
    let dir = user_dir(&app)?;
    let lib_id = save_version(&dir, &name, doc)?;
    let version = ewm::load_file(dir.join(user_file(&lib_id)?)).map_err(|e| e.to_string())?.document;
    engine.edit_service(|s| s.set_document(&id, version).map_err(|e| e.to_string()))?;
    Ok(lib_id)
}

/// Makes a version the default for its built-in item; `None` (or the
/// built-in item itself) goes back to the Prayer Book's text.
#[tauri::command]
pub fn library_set_default(app: AppHandle, base: String, id: Option<String>) -> CmdResult<()> {
    crate::sync::soon();
    if !BUILTIN.iter().any(|(n, _)| *n == base) {
        return Err("no such built-in item".into());
    }
    let dir = user_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let mut map = read_defaults(&dir);
    match id.as_deref().filter(|i| i.starts_with("user:")) {
        Some(id) => {
            let file = user_file(id)?;
            let doc = ewm::load_file(dir.join(file)).map_err(|e| e.to_string())?.document;
            if doc.meta.extra(BASED_ON) != Some(base.as_str()) {
                return Err("that item is not a version of this one".into());
            }
            map.insert(base, file.to_string());
        }
        None => {
            map.remove(&base);
        }
    }
    write_defaults(&dir, &map)
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

/// A built-in item's document by name (e.g. `"collect-for-purity"`): the
/// parish's default version if it set one, else the Prayer Book's text.
pub fn builtin_doc(name: &str) -> Option<LiturgyDocument> {
    if let Some(dir) = crate::storage::root().map(|r| r.join("library")) {
        if let Some(doc) = default_doc(&dir, name) {
            return Some(doc);
        }
    }
    bcp_doc(name)
}

/// Front matter: what the parish uses an item for in a service.
pub const USES: &str = "uses";
/// [`USES`]: a form of the Prayers of the People.
pub const PRAYERS_OF_THE_PEOPLE: &str = "prayers_of_the_people";

/// Marks a library item as used for something in a service (or unmarks
/// it), e.g. a parish's own Prayers of the People.
#[tauri::command]
pub fn library_set_uses(app: AppHandle, id: String, uses: Option<String>) -> CmdResult<()> {
    crate::sync::soon();
    let path = user_dir(&app)?.join(user_file(&id)?);
    let mut doc = ewm::load_file(&path).map_err(|e| e.to_string())?.document;
    doc.meta.extra.retain(|(k, _)| k != USES);
    if let Some(u) = uses.map(|u| u.trim().to_string()).filter(|u| !u.is_empty()) {
        doc.meta.extra.push((USES.into(), u));
    }
    std::fs::write(&path, ewm::to_string(&doc)).map_err(|e| format!("could not save {}: {e}", path.display()))
}

/// Loads a library item's document by id.
pub fn load(app: &AppHandle, id: &str) -> CmdResult<LiturgyDocument> {
    if let Some(name) = id.strip_prefix("builtin:") {
        return bcp_doc(name).ok_or_else(|| "no such built-in item".into());
    }
    let path = user_dir(app)?.join(user_file(id)?);
    ewm::load_file(&path).map(|p| p.document).map_err(|e| e.to_string())
}

/// Every library item: built-in first (service order), then the user's.
/// Run apart from the window, so it opens at once from a slow shared folder.
#[tauri::command]
pub async fn library_list(app: AppHandle) -> Vec<LibraryEntry> {
    tauri::async_runtime::spawn_blocking(move || list_library(&app)).await.unwrap_or_default()
}

/// [`library_list`]. The parish's items are read again only when their
/// file has changed since last time (see [`crate::cache`]).
fn list_library(app: &AppHandle) -> Vec<LibraryEntry> {
    let mut out: Vec<LibraryEntry> = BUILTIN
        .iter()
        .filter_map(|(name, src)| ewm::parse(src).ok().map(|p| entry(format!("builtin:{name}"), &p.document, true)))
        .collect();
    // ("-2": entries now say what an item is used for.)
    let mut cache = crate::cache::Cache::open(app, "library-2");
    for path in user_files(app) {
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
        let Some(stamp) = crate::cache::file_stamp(&path) else { continue };
        let read = || match ewm::load_file(&path) {
            Ok(p) => Some(entry(format!("user:{name}"), &p.document, false)),
            Err(e) => {
                tracing::warn!("skipping library file: {e}");
                None
            }
        };
        if let Some(e) = cache.get(&path, &stamp, read) {
            out.push(e);
        }
    }
    cache.save();
    // Which version of each built-in item services use.
    let defaults = user_dir(app).map(|d| read_defaults(&d)).unwrap_or_default();
    let ids: Vec<(String, bool)> = out.iter().map(|e| (e.id.clone(), e.builtin)).collect();
    for e in &mut out {
        e.default = match (&e.base, e.builtin) {
            (Some(base), false) => defaults.get(base).is_some_and(|f| e.id == format!("user:{f}")),
            (None, true) => {
                let name = e.id.trim_start_matches("builtin:");
                // A default whose file is gone falls back to the BCP.
                defaults.get(name).is_none_or(|f| !ids.iter().any(|(i, _)| *i == format!("user:{f}")))
            }
            _ => false,
        };
    }
    mark_dupes(&mut out);
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
pub fn library_save_cue(
    app: AppHandle,
    engine: State<'_, Engine>,
    id: String,
    replace: bool,
    version: Option<VersionSave>,
) -> CmdResult<String> {
    crate::sync::soon();
    let service = engine.service().ok_or("no service is loaded")?;
    let cue = service.cue(&id).ok_or("no such cue")?;
    let doc = cue.document().ok_or("announcements cues cannot be saved to the library")?;
    let dir = user_dir(&app)?;
    // A Prayer Book item (or a copy of one): saved as a version of it.
    if let Some(name) = doc.meta.extra(BASED_ON).filter(|n| BUILTIN.iter().any(|(b, _)| b == n)) {
        let existing = doc.meta.extra(CREATED).and_then(|_| find_version(&dir, doc));
        return match (version, existing) {
            (None, Some(file)) => {
                Err(format!("version:{}", doc.meta.extra(CREATED).unwrap_or_default().to_owned() + "|" + &file))
            }
            (Some(VersionSave::Update), Some(file)) => {
                let path = dir.join(&file);
                std::fs::write(&path, ewm::to_string(doc))
                    .map_err(|e| format!("could not save {}: {e}", path.display()))?;
                Ok(format!("user:{file}"))
            }
            _ => save_version(&dir, name, doc.clone()),
        };
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let (doc, name) = library_copy_of(doc)?;
    let path = dir.join(&name);
    if path.exists() && !replace {
        return Err(format!("exists:{}", doc.meta.title));
    }
    std::fs::write(&path, ewm::to_string(&doc)).map_err(|e| format!("could not save {}: {e}", path.display()))?;
    Ok(format!("user:{name}"))
}

/// The library's copy of an item and its file name. A hymn is kept under
/// its own number and title ("hymn-436-lift-up-your-heads"), never under its
/// place in the service: this week's entrance hymn may close the service
/// next time.
fn library_copy_of(doc: &LiturgyDocument) -> CmdResult<(LiturgyDocument, String)> {
    if doc.meta.effective_category() != episco_core::ElementCategory::Hymn {
        return Ok((doc.clone(), slug_file_name(&doc.meta.title, ewm::EXTENSION)));
    }
    let title = episco_core::hymn::library_title(doc).ok_or_else(|| {
        format!(
            "“{}” is the hymn's place in the service, not the hymn. Give it the hymn's number or title in the Editor, then save it to the library.",
            doc.meta.title
        )
    })?;
    let mut doc = doc.clone();
    doc.meta.title = title.clone();
    let number = doc.meta.hymn_number.clone().unwrap_or_default();
    let stem = if title == format!("Hymn {number}") { title } else { format!("hymn {number} {title}") };
    Ok((doc, slug_file_name(&stem, ewm::EXTENSION)))
}

/// Puts a library item into an existing cue in place of its text, keeping
/// the cue's name: a hymn from the library fills the "Entrance Hymn" slot
/// and stays the Entrance Hymn.
#[tauri::command]
pub fn library_fill_cue(app: AppHandle, engine: State<'_, Engine>, id: String, cue: String) -> CmdResult<()> {
    let doc = load(&app, &id)?;
    engine.edit_service(|s| {
        let c = s.cue(&cue).ok_or("no such cue")?;
        let mut overrides = c.overrides.clone();
        overrides.title = Some(c.title.clone());
        // A hymnal setting (the Sanctus as S 125, say) is sung: shown as a hymn.
        if doc.meta.effective_category() == episco_core::ElementCategory::Hymn && doc.meta.extra(BASED_ON).is_none() {
            overrides.category = None;
        }
        s.set_document(&cue, doc).map_err(|e| e.to_string())?;
        s.set_overrides(&cue, overrides).map_err(|e| e.to_string())
    })
}

/// How "Save to Library" saves a copy of a Prayer Book item that came
/// from one of the parish's versions.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionSave {
    /// Overwrite the version it came from.
    Update,
    /// Save another, dated version.
    New,
}

/// The version file a document came from (same item, same day made).
fn find_version(dir: &Path, doc: &LiturgyDocument) -> Option<String> {
    let (base, created) = (doc.meta.extra(BASED_ON)?, doc.meta.extra(CREATED)?);
    let entries = std::fs::read_dir(dir).ok()?;
    let mut files: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    files.sort();
    files.into_iter().find_map(|p| {
        let d = ewm::load_file(&p).ok()?.document;
        (d.meta.extra(BASED_ON) == Some(base)
            && d.meta.extra(CREATED) == Some(created)
            && d.meta.title == doc.meta.title)
            .then(|| p.file_name()?.to_str().map(String::from))
            .flatten()
    })
}

/// Makes a library item from pasted text (e.g. a prayer from the parish's
/// own copy of a supplemental liturgy) and saves it to the parish library.
/// With `replace` false, refuses to overwrite an item of the same name.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn library_create_from_text(
    app: AppHandle,
    title: String,
    category: episco_core::ElementCategory,
    citation: Option<String>,
    text: String,
    replace: bool,
    uses: Option<String>,
    rite: Option<String>,
) -> CmdResult<String> {
    crate::sync::soon();
    if title.trim().is_empty() {
        return Err("give the item a title".into());
    }
    let mut doc = episco_core::paste::document_from_text(&title, category, citation.as_deref(), &text, 70);
    if let Some(u) = uses.filter(|u| !u.trim().is_empty()) {
        doc.meta.extra.push((USES.into(), u));
    }
    // "I" or "II": offered for services in that rite only.
    if let Some(r) = rite.filter(|r| matches!(r.as_str(), "I" | "II")) {
        doc.meta.extra.retain(|(k, _)| k != "rite");
        doc.meta.extra.push(("rite".into(), r));
    }
    if !doc.blocks.iter().any(|b| matches!(b, episco_core::Block::Paragraph { .. })) {
        return Err("paste the text first".into());
    }
    let dir = user_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let name = slug_file_name(&doc.meta.title, ewm::EXTENSION);
    let path = dir.join(&name);
    if path.exists() && !replace {
        return Err(format!("exists:{}", doc.meta.title));
    }
    std::fs::write(&path, ewm::to_string(&doc)).map_err(|e| format!("could not save {}: {e}", path.display()))?;
    Ok(format!("user:{name}"))
}

/// Deletes an item from My Library (built-in items cannot be deleted).
#[tauri::command]
pub fn library_delete(app: AppHandle, id: String) -> CmdResult<()> {
    crate::sync::soon();
    if id.starts_with("builtin:") {
        return Err("built-in items cannot be deleted".into());
    }
    let file = user_file(&id)?;
    let dir = user_dir(&app)?;
    let path = dir.join(file);
    std::fs::remove_file(&path).map_err(|e| format!("could not delete {}: {e}", path.display()))?;
    // A deleted default: back to the Prayer Book's text.
    let mut map = read_defaults(&dir);
    let before = map.len();
    map.retain(|_, f| f != file);
    if map.len() != before {
        write_defaults(&dir, &map)?;
    }
    Ok(())
}

/// Keeps one of the parish's copies of an item and removes the others
/// (services keep their own text, so nothing else changes).
#[tauri::command]
pub fn library_merge(app: AppHandle, keep: String, remove: Vec<String>) -> CmdResult<usize> {
    crate::sync::soon();
    let dir = user_dir(&app)?;
    let mut removed = 0;
    for id in remove.iter().filter(|id| **id != keep) {
        let path = dir.join(user_file(id)?);
        if path.is_file() {
            std::fs::remove_file(&path).map_err(|e| format!("could not delete {}: {e}", path.display()))?;
            removed += 1;
        }
    }
    Ok(removed)
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
    app: AppHandle,
    engine: State<'_, Engine>,
    items: Vec<ProperItem>,
    day: Option<String>,
    after: Option<usize>,
) -> CmdResult<Vec<String>> {
    let service = engine.service();
    let rite = service.as_ref().and_then(|s| s.rite).unwrap_or_default();
    let mut docs: Vec<_> = items.iter().map(|i| propers::build_document_in(i, day.as_deref(), rite)).collect();
    // Reading texts from the parish's Bible file, if it has one.
    crate::scripture::fill_documents(&app, &mut docs, service.and_then(|s| s.bible).unwrap_or_default());
    insert_documents(&engine, docs, after)
}

/// The Gospel's music line (a picture from the media library, sung before
/// it): `src` sets it (`None` removes it). The slides are put in order:
/// music, the announcement, the title, the text, the response.
#[tauri::command]
pub fn gospel_set_music(
    app: AppHandle,
    engine: State<'_, Engine>,
    id: String,
    src: Option<String>,
    alt: Option<String>,
) -> CmdResult<()> {
    crate::sync::soon();
    let music = src.filter(|p| !p.trim().is_empty()).map(|p| (p, alt.unwrap_or_default()));
    // The music chosen becomes the usual one: new services start with it.
    if let Some(m) = &music {
        remember_gospel_music(&app, m);
    }
    engine.edit_service(|s| {
        let doc = s.cue(&id).and_then(|c| c.document()).ok_or("no such item")?;
        let new = propers::arrange_gospel(doc, Some(music));
        s.set_document(&id, new).map_err(|e| e.to_string())
    })
}

/// The parish's usual music line before the Gospel (the last one chosen),
/// kept in the library folder as `gospel-music.json`.
fn gospel_music_file(app: &AppHandle) -> CmdResult<PathBuf> {
    crate::storage::data_dir(app, "gospel-music.json")
}

fn remember_gospel_music(app: &AppHandle, (src, alt): &(String, String)) {
    // Inside the library folder it is saved as `library:…`, which every
    // computer can find.
    let src = crate::storage::root()
        .and_then(|root| episco_core::portable::to_portable(src, &root))
        .unwrap_or_else(|| src.clone());
    if let Ok(path) = gospel_music_file(app) {
        let json = serde_json::json!({ "src": src, "alt": alt });
        if let Err(e) = std::fs::write(&path, json.to_string()) {
            tracing::warn!("could not save {}: {e}", path.display());
        }
    }
}

/// The music line new services put before the Gospel, if the parish has one.
pub fn usual_gospel_music(app: &AppHandle) -> Option<(String, String)> {
    let text = std::fs::read_to_string(gospel_music_file(app).ok()?).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let src = v.get("src")?.as_str()?.to_string();
    let alt = v.get("alt").and_then(|a| a.as_str()).unwrap_or_default().to_string();
    (!src.is_empty()).then_some((src, alt))
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

// ---- hymns ------------------------------------------------------------------

/// Replaces a hymn's words with pasted text (from the parish's licensed
/// source), one stanza per slide. Returns the number of slides.
#[tauri::command]
pub fn hymn_set_text(
    engine: State<'_, Engine>,
    id: String,
    text: String,
    options: Option<episco_core::hymn::HymnOptions>,
) -> CmdResult<usize> {
    let opts = options.unwrap_or_default();
    engine.edit_service(|s| {
        let doc = s.cue(&id).and_then(|c| c.document()).ok_or("no such item")?;
        let new = episco_core::hymn::set_hymn_text(doc, &text, &opts);
        if new.blocks.is_empty() {
            return Err("no words found in the pasted text".to_string());
        }
        let n = new.slides().len();
        s.set_document(&id, new).map_err(|e| e.to_string())?;
        Ok(n)
    })
}

// ---- anthems -----------------------------------------------------------------

/// Adds a choir anthem after cue index `after`: an item named for its place
/// ("Offertory Anthem") with the piece's title and composer, shown as a
/// lower third on the stream and listed in the bulletin.
#[tauri::command]
pub fn anthem_add(
    engine: State<'_, Engine>,
    after: Option<usize>,
    name: String,
    title: String,
    composer: String,
) -> CmdResult<String> {
    let name = match name.trim() {
        "" => "Anthem".to_string(),
        n => n.to_string(),
    };
    let doc = episco_core::music::anthem_document(&title, &composer);
    engine.edit_service(|s| {
        let at = after.map(|a| a + 1).unwrap_or(s.cues.len());
        let overrides = CueOverrides { title: Some(name.clone()), ..Default::default() };
        let mut cue = Cue::liturgy(s.unique_id(&name), doc, None, overrides);
        cue.modified = true;
        Ok(s.insert_cue(at, cue))
    })
}

/// Changes an anthem's title and composer; the item keeps its name.
#[tauri::command]
pub fn anthem_set(engine: State<'_, Engine>, id: String, title: String, composer: String) -> CmdResult<()> {
    engine.edit_service(|s| {
        let cue = s.cue(&id).ok_or("no such item")?;
        let doc = cue.document().ok_or("not an anthem")?;
        let new = episco_core::music::set_anthem(doc, &title, &composer);
        let mut overrides = cue.overrides.clone();
        overrides.title.get_or_insert_with(|| cue.title.clone());
        s.set_document(&id, new).map_err(|e| e.to_string())?;
        s.set_overrides(&id, overrides).map_err(|e| e.to_string())
    })
}

#[derive(Serialize)]
pub struct HymnImport {
    added: usize,
    skipped: Vec<String>,
}

/// Imports hymns from text files into My Library: one file per hymn, named
/// with its number and title ("436 Lift Up Your Heads.txt"), holding the
/// words as copied from the parish's licensed source.
#[tauri::command]
pub async fn library_import_hymns(app: AppHandle) -> CmdResult<HymnImport> {
    crate::sync::soon();
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().add_filter("Hymn words", &["txt"]).blocking_pick_files().unwrap_or_default();
    let dir = user_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let mut added = 0;
    let mut skipped = Vec::new();
    for file in picked {
        let path = file.into_path().map_err(|e| e.to_string())?;
        let name = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let Ok(text) = std::fs::read_to_string(&path) else {
            skipped.push(format!("{name} (not UTF-8 text)"));
            continue;
        };
        let (number, title) = episco_core::hymn::split_file_name(&name);
        let doc = episco_core::hymn::hymn_document(&title, number.as_deref(), &text, &Default::default());
        if doc.blocks.is_empty() {
            skipped.push(format!("{name} (no words)"));
            continue;
        }
        let Ok((doc, file_name)) = library_copy_of(&doc) else {
            skipped.push(format!("{name} (no hymn number or title)"));
            continue;
        };
        std::fs::write(dir.join(&file_name), ewm::to_string(&doc))
            .map_err(|e| format!("could not save {file_name}: {e}"))?;
        added += 1;
    }
    Ok(HymnImport { added, skipped })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_of_an_item_are_marked() {
        let doc = |src: &str| -> LiturgyDocument { src.parse().unwrap() };
        let mut list = vec![
            entry(
                "user:a.ewm".into(),
                &doc("---\ntitle: Lift Up Your Heads\ntype: hymn\nhymn: \"436\"\n---\n1 Words\n"),
                false,
            ),
            entry(
                "user:b.ewm".into(),
                &doc("---\ntitle: Lift up your heads, ye mighty gates\ntype: hymn\nhymn: \"436\"\n---\n1 Words\n"),
                false,
            ),
            entry("user:c.ewm".into(), &doc("---\ntitle: The Parish Prayer\ntype: prayer\n---\nWords\n"), false),
            entry("user:d.ewm".into(), &doc("---\ntitle: Parish prayer\ntype: prayer\n---\nOther words\n"), false),
            entry(
                "user:e.ewm".into(),
                &doc("---\ntitle: Amazing Grace\ntype: hymn\nhymn: \"671\"\n---\n1 Words\n"),
                false,
            ),
        ];
        mark_dupes(&mut list);
        assert_eq!(list[0].dupes, vec!["user:b.ewm"]);
        assert_eq!(list[1].dupes, vec!["user:a.ewm"]);
        assert_eq!(list[2].dupes, vec!["user:d.ewm"]);
        assert!(list[4].dupes.is_empty());
    }

    #[test]
    fn hymns_are_saved_by_their_own_name() {
        let slot: LiturgyDocument = "---\ntitle: Entrance Hymn\ntype: hymn\n---\n1 Words\n".parse().unwrap();
        assert!(library_copy_of(&slot).is_err(), "only the slot's name");
        let mut hymn = slot.clone();
        hymn.meta.hymn_number = Some("436".into());
        let (doc, file) = library_copy_of(&hymn).unwrap();
        assert_eq!((doc.meta.title.as_str(), file.as_str()), ("Hymn 436", "hymn-436.ewm"));
        hymn.meta.title = "Lift Up Your Heads".into();
        let (doc, file) = library_copy_of(&hymn).unwrap();
        assert_eq!((doc.meta.title.as_str(), file.as_str()), ("Lift Up Your Heads", "hymn-436-lift-up-your-heads.ewm"));
        let prayer: LiturgyDocument = "---\ntitle: A Prayer\n---\nWords\n".parse().unwrap();
        assert_eq!(library_copy_of(&prayer).unwrap().1, "a-prayer.ewm");
    }

    #[test]
    fn parish_versions_of_prayer_book_items() {
        let dir = std::env::temp_dir().join(format!("episco-versions-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let bcp = bcp_doc("nicene-creed").unwrap();
        assert_eq!(bcp.meta.extra(BASED_ON), Some("nicene-creed"));
        assert!(default_doc(&dir, "nicene-creed").is_none(), "no default: the BCP");
        // Two copies made today: dated, numbered apart.
        let a = save_version(&dir, "nicene-creed", bcp.clone()).unwrap();
        let b = save_version(&dir, "nicene-creed", bcp.clone()).unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with(&format!("user:nicene-creed--{}", today())), "{a}");
        let file_a = a.trim_start_matches("user:");
        let mut edited = ewm::load_file(dir.join(file_a)).unwrap().document;
        assert_eq!(edited.meta.extra(CREATED), Some(today().as_str()));
        // The copy it came from is found again for "Update".
        assert!(find_version(&dir, &edited).is_some());
        edited.meta.title = "The Nicene Creed (parish)".into();
        std::fs::write(dir.join(file_a), ewm::to_string(&edited)).unwrap();
        // Set as the default: services get it.
        let mut map = BTreeMap::new();
        map.insert("nicene-creed".to_string(), file_a.to_string());
        write_defaults(&dir, &map).unwrap();
        assert_eq!(default_doc(&dir, "nicene-creed").unwrap().meta.title, "The Nicene Creed (parish)");
        // A missing file falls back.
        std::fs::remove_file(dir.join(file_a)).unwrap();
        assert!(default_doc(&dir, "nicene-creed").is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn every_builtin_item_parses_cleanly() {
        for (name, src) in BUILTIN {
            let parsed = ewm::parse(src).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(parsed.warnings.is_empty(), "{name}: {:?}", parsed.warnings);
            assert!(!parsed.document.slides().is_empty(), "{name} has no slides");
        }
        let names: Vec<&str> = BUILTIN.iter().map(|(n, _)| *n).collect();
        for ep in [
            "eucharistic-prayer-a",
            "eucharistic-prayer-b",
            "eucharistic-prayer-c",
            "eucharistic-prayer-d",
            "rite1/eucharistic-prayer-1",
            "rite1/eucharistic-prayer-2",
        ] {
            assert!(names.contains(&ep), "{ep}");
        }
        // Prayer C's people's lines are the people's, in bold.
        let c = builtin_doc("eucharistic-prayer-c").unwrap();
        let people: Vec<String> =
            c.blocks.iter().filter(|b| b.role() == Some(&episco_core::Role::People)).map(|b| b.plain_text()).collect();
        assert!(people.contains(&"Glory to you for ever and ever.".to_string()), "{people:?}");
        assert!(people.contains(&"Risen Lord, be known to us in the breaking of the Bread.".to_string()));
        // Texts said by everyone together are marked All.
        for name in ["nicene-creed", "rite1/nicene-creed", "lords-prayer", "confession", "rite1/humble-access"] {
            let d = builtin_doc(name).unwrap();
            assert!(d.blocks.iter().any(|b| b.role() == Some(&episco_core::Role::All)), "{name}");
            assert!(!d.blocks.iter().any(|b| b.role() == Some(&episco_core::Role::People)), "{name}");
        }
        let one = builtin_doc("rite1/eucharistic-prayer-1").unwrap();
        assert!(one.blocks.iter().any(|b| b.plain_text().starts_with("All glory be to thee, Almighty God")));
    }
}
