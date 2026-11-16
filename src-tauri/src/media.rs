//! The media library: images and videos copied into the app's data folder
//! (`<app data>/media/`), to show full screen as their own item, to use as
//! a background, or to place in an item (a psalm's music line, …).

use std::path::{Path, PathBuf};

use episco_core::{Cue, CueOverrides, ElementCategory, Fit, Layout, LiturgyDocument, Look, Output};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

pub const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "svg", "webp"];
pub const VIDEO_EXTENSIONS: [&str; 5] = ["mp4", "m4v", "mov", "webm", "ogv"];

#[derive(Serialize)]
pub struct MediaEntry {
    path: PathBuf,
    /// File name without the extension.
    name: String,
    /// `image` or `video`.
    kind: &'static str,
    /// Bytes.
    size: u64,
    /// The parish's words for it ("Advent", "music", "Christmas pageant").
    tags: Vec<String>,
}

// ---- tags -------------------------------------------------------------------

/// File name → tags, kept beside the files in `media/tags.json` (so every
/// computer using the library sees them).
type Tags = std::collections::BTreeMap<String, Vec<String>>;

fn read_tags(dir: &Path) -> Tags {
    std::fs::read_to_string(dir.join("tags.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn write_tags(dir: &Path, tags: &Tags) -> CmdResult<()> {
    let path = dir.join("tags.json");
    let json = serde_json::to_string_pretty(tags).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("could not save {}: {e}", path.display()))
}

/// Tags as typed: trimmed, without repeats (ignoring case), in order.
fn clean_tags(tags: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tags {
        let t = t.trim().trim_start_matches('#').trim().to_string();
        if !t.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(&t)) {
            out.push(t);
        }
    }
    out
}

fn file_key(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Sets a media file's tags (an empty list removes them).
#[tauri::command]
pub fn media_set_tags(app: AppHandle, path: PathBuf, tags: Vec<String>) -> CmdResult<Vec<String>> {
    crate::sync::soon();
    let path = inside_media(&app, &path)?;
    let dir = media_dir(&app)?;
    let mut all = read_tags(&dir);
    let tags = clean_tags(tags);
    if tags.is_empty() {
        all.remove(&file_key(&path));
    } else {
        all.insert(file_key(&path), tags.clone());
    }
    write_tags(&dir, &all)?;
    Ok(tags)
}

fn media_dir(app: &AppHandle) -> CmdResult<PathBuf> {
    crate::storage::data_dir(app, "media")
}

fn ext(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

fn kind(p: &Path) -> Option<&'static str> {
    let e = ext(p);
    if IMAGE_EXTENSIONS.contains(&e.as_str()) {
        Some("image")
    } else if VIDEO_EXTENSIONS.contains(&e.as_str()) {
        Some("video")
    } else {
        None
    }
}

fn entry(path: PathBuf) -> Option<MediaEntry> {
    let kind = kind(&path)?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let name = path.file_stem()?.to_string_lossy().into_owned();
    Some(MediaEntry { path, name, kind, size, tags: Vec::new() })
}

/// Every image and video in the media folder, by name.
#[tauri::command]
pub fn media_list(app: AppHandle) -> Vec<MediaEntry> {
    let Ok(dir) = media_dir(&app) else { return Vec::new() };
    let tags = read_tags(&dir);
    let Ok(read) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut out: Vec<MediaEntry> = read.filter_map(|e| e.ok()).filter_map(|e| entry(e.path())).collect();
    for e in &mut out {
        e.tags = tags.get(&file_key(&e.path)).cloned().unwrap_or_default();
    }
    out.sort_by_key(|e| e.name.to_lowercase());
    out
}

/// A free name in `dir` for `name` ("x.png", "x-2.png", …).
fn unique(dir: &Path, name: &str) -> PathBuf {
    let p = Path::new(name);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "media".into());
    let ext = ext(p);
    let mut candidate = dir.join(name);
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{stem}-{n}.{ext}"));
        n += 1;
    }
    candidate
}

/// Copies a file into the media folder (under a free name), unless it is
/// there already. Returns where it is.
pub fn copy_into_library(app: &AppHandle, src: &Path) -> CmdResult<PathBuf> {
    let dir = media_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    if src.parent() == Some(dir.as_path()) {
        return Ok(src.to_path_buf());
    }
    let name = src.file_name().ok_or("bad file name")?.to_string_lossy().into_owned();
    let dest = unique(&dir, &name);
    std::fs::copy(src, &dest).map_err(|e| format!("could not copy {}: {e}", src.display()))?;
    Ok(dest)
}

// ---- music: antiphons, settings, Gospel Acclamations -------------------------

/// The music folders in the parish library (`music/Antiphon`, …), by kind:
/// pictures of music lines kept apart from the media library.
fn music_folder(kind: &str) -> CmdResult<&'static str> {
    match kind {
        "antiphon" => Ok("Antiphon"),
        "setting" => Ok("Setting"),
        "gospel" => Ok("Gospel Acclamation"),
        _ => Err(format!("no music folder “{kind}”")),
    }
}

fn music_dir(app: &AppHandle, kind: &str) -> CmdResult<PathBuf> {
    Ok(crate::storage::data_dir(app, "music")?.join(music_folder(kind)?))
}

/// The pictures in one music folder (`antiphon`, `setting`, `gospel`), by name.
#[tauri::command]
pub fn music_list(app: AppHandle, kind: String) -> CmdResult<Vec<MediaEntry>> {
    let dir = music_dir(&app, &kind)?;
    let Ok(read) = std::fs::read_dir(&dir) else { return Ok(Vec::new()) };
    let mut out: Vec<MediaEntry> =
        read.filter_map(|e| e.ok()).filter_map(|e| entry(e.path())).filter(|e| e.kind == "image").collect();
    out.sort_by_key(|e| e.name.to_lowercase());
    Ok(out)
}

/// Copies pictures chosen in a file dialog into a music folder.
#[tauri::command]
pub async fn music_import(app: AppHandle, kind: String) -> CmdResult<Vec<MediaEntry>> {
    crate::sync::soon();
    let dir = music_dir(&app, &kind)?;
    let picked = app
        .dialog()
        .file()
        .set_title(format!("Add to the {} folder", music_folder(&kind)?))
        .add_filter("Images", &IMAGE_EXTENSIONS)
        .blocking_pick_files()
        .unwrap_or_default();
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let mut out = Vec::new();
    for file in picked {
        let src = file.into_path().map_err(|e| e.to_string())?;
        if kind_of_image(&src) {
            let dest = if src.parent() == Some(dir.as_path()) {
                src.clone()
            } else {
                let name = src.file_name().ok_or("bad file name")?.to_string_lossy().into_owned();
                let dest = unique(&dir, &name);
                std::fs::copy(&src, &dest).map_err(|e| format!("could not copy {}: {e}", src.display()))?;
                dest
            };
            out.extend(entry(dest));
        }
    }
    Ok(out)
}

fn kind_of_image(p: &Path) -> bool {
    kind(p) == Some("image")
}

/// Copies images and videos chosen in a file dialog into the media folder.
#[tauri::command]
pub async fn media_import(app: AppHandle) -> CmdResult<Vec<MediaEntry>> {
    crate::sync::soon();
    let all: Vec<&str> = IMAGE_EXTENSIONS.iter().chain(VIDEO_EXTENSIONS.iter()).copied().collect();
    let picked = app
        .dialog()
        .file()
        .add_filter("Images and videos", &all)
        .add_filter("Images", &IMAGE_EXTENSIONS)
        .add_filter("Videos", &VIDEO_EXTENSIONS)
        .blocking_pick_files()
        .unwrap_or_default();
    let dir = media_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let mut out = Vec::new();
    for file in picked {
        let src = file.into_path().map_err(|e| e.to_string())?;
        if kind(&src).is_none() {
            continue;
        }
        let dest = if src.parent() == Some(dir.as_path()) {
            src.clone()
        } else {
            let name = src.file_name().ok_or("bad file name")?.to_string_lossy().into_owned();
            let dest = unique(&dir, &name);
            std::fs::copy(&src, &dest).map_err(|e| format!("could not copy {}: {e}", src.display()))?;
            dest
        };
        out.extend(entry(dest));
    }
    Ok(out)
}

fn inside_media(app: &AppHandle, path: &Path) -> CmdResult<PathBuf> {
    let dir = media_dir(app)?;
    if path.parent() != Some(dir.as_path()) || kind(path).is_none() {
        return Err("not a file in the media library".into());
    }
    Ok(path.to_path_buf())
}

/// Deletes a file from the media library.
#[tauri::command]
pub fn media_delete(app: AppHandle, path: PathBuf) -> CmdResult<()> {
    crate::sync::soon();
    let path = inside_media(&app, &path)?;
    std::fs::remove_file(&path).map_err(|e| format!("could not delete {}: {e}", path.display()))?;
    let dir = media_dir(&app)?;
    let mut all = read_tags(&dir);
    if all.remove(&file_key(&path)).is_some() {
        write_tags(&dir, &all)?;
    }
    Ok(())
}

/// Renames a file in the media library (keeps the extension).
#[tauri::command]
pub fn media_rename(app: AppHandle, path: PathBuf, name: String) -> CmdResult<PathBuf> {
    let path = inside_media(&app, &path)?;
    let clean: String = name.trim().chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '\0')).collect();
    if clean.is_empty() || clean.starts_with('.') {
        return Err("enter a name".into());
    }
    let dest = path.with_file_name(format!("{clean}.{}", ext(&path)));
    if dest.exists() {
        return Err(format!("“{clean}” already exists"));
    }
    std::fs::rename(&path, &dest).map_err(|e| e.to_string())?;
    // The tags go with the file.
    let dir = media_dir(&app)?;
    let mut all = read_tags(&dir);
    if let Some(t) = all.remove(&file_key(&path)) {
        all.insert(file_key(&dest), t);
        write_tags(&dir, &all)?;
    }
    Ok(dest)
}

/// The media folder, for opening in Finder / Explorer.
#[tauri::command]
pub fn media_folder(app: AppHandle) -> CmdResult<PathBuf> {
    let dir = media_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// The look that shows a media file full screen.
pub fn full_screen(path: &str) -> Look {
    Look {
        background: Some(path.to_string()),
        layout: Some(Layout::Centered),
        fit: Some(Fit::Fit),
        tone: None,
        blank: None,
    }
}

/// Adds each file as its own item after cue index `after` (else at the
/// end), shown full screen on both outputs, or with `sanctuary_only` on the
/// sanctuary screen alone (the stream shows just the camera).
#[tauri::command]
pub fn media_add(
    engine: State<'_, Engine>,
    paths: Vec<PathBuf>,
    after: Option<usize>,
    sanctuary_only: Option<bool>,
) -> CmdResult<Vec<String>> {
    let sanctuary_only = sanctuary_only.unwrap_or(false);
    engine.edit_service(|s| {
        let start = after.map(|a| a + 1).unwrap_or(s.cues.len());
        let mut ids = Vec::new();
        for (at, path) in (start..).zip(paths) {
            let value = path.to_string_lossy().replace('\\', "/");
            let title = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Media".into());
            let mut doc = LiturgyDocument::default();
            doc.meta.title = title.clone();
            doc.meta.category = Some(ElementCategory::Other);
            doc.meta.extra.push(("media".into(), value.clone()));
            let mut overrides = CueOverrides::default();
            *overrides.looks.get_mut(Output::Sanctuary) = full_screen(&value);
            *overrides.looks.get_mut(Output::Stream) = if sanctuary_only { camera_only() } else { full_screen(&value) };
            let mut cue = Cue::liturgy(s.unique_id(&title), doc, None, overrides);
            cue.modified = true;
            ids.push(s.insert_cue(at, cue));
        }
        Ok(ids)
    })
}

/// The stream showing only the key colour (the camera through the keyer).
pub fn camera_only() -> Look {
    Look {
        background: Some(episco_core::look::KEY_BACKGROUND.into()),
        layout: Some(Layout::Centered),
        tone: None,
        fit: None,
        blank: Some(true),
    }
}

/// A planned blank: the sanctuary screen goes black while the stream shows
/// the camera. Added after cue index `after` (else at the end).
#[tauri::command]
pub fn blank_add(engine: State<'_, Engine>, after: Option<usize>, title: Option<String>) -> CmdResult<String> {
    engine.edit_service(|s| {
        let title = title.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).unwrap_or_else(|| "Blank".into());
        let mut doc = LiturgyDocument::default();
        doc.meta.title = title.clone();
        doc.meta.category = Some(ElementCategory::Other);
        doc.meta.extra.push(("blank".into(), "sanctuary".into()));
        let mut overrides = CueOverrides::default();
        *overrides.looks.get_mut(Output::Sanctuary) = Look {
            background: Some("#000000".into()),
            layout: Some(Layout::Centered),
            tone: None,
            fit: None,
            blank: Some(true),
        };
        *overrides.looks.get_mut(Output::Stream) = camera_only();
        let at = after.map(|a| a + 1).unwrap_or(s.cues.len());
        let mut cue = Cue::liturgy(s.unique_id(&title), doc, None, overrides);
        cue.modified = true;
        Ok(s.insert_cue(at, cue))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_tidied_and_kept_beside_the_files() {
        assert_eq!(
            clean_tags(vec![" Advent ".into(), "#music".into(), "advent".into(), "".into()]),
            ["Advent", "music"]
        );
        let dir = std::env::temp_dir().join(format!("episco-tags-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut t = Tags::new();
        t.insert("alleluia.png".into(), vec!["music".into()]);
        write_tags(&dir, &t).unwrap();
        assert_eq!(read_tags(&dir), t);
        let _ = std::fs::remove_dir_all(dir);
    }
}
