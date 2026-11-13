//! Slides made in the app, imported presentations and web videos.
//!
//! * **Made slides**: an item of simple slides, each a *Title* slide (`#`
//!   heading, with an optional line under it), *Information with title*
//!   (`##` heading, then text or a picture) or *Information without title*
//!   (text or a picture alone), between `---` breaks. Pictures come from the
//!   media library and are sized `center`, `width`, `height` or `full`.
//! * **Imported slides**: a PDF (saved from PowerPoint, Keynote or Google
//!   Slides) copied into the media library, or a Google Slides link, one
//!   full-screen picture per page.
//! * **Web video**: a YouTube, Facebook or Instagram link played full screen.
//!
//! All three are ordinary items that can go anywhere in a service; they are
//! not printed in the bulletin.

use std::path::PathBuf;

use episco_core::document::{SLIDE_MAKER, WEB_VIDEO};
use episco_core::{Block, Cue, CueOverrides, ElementCategory, Layout, LiturgyDocument, Look, Output, TextTone};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::engine::Engine;

type CmdResult<T> = Result<T, String>;

/// The look made slides start with on both screens (changeable in Slides).
fn slides_look() -> Look {
    Look {
        background: Some("builtin:glass-dark".into()),
        layout: Some(Layout::Centered),
        tone: Some(TextTone::Light),
        fit: None,
        blank: None,
    }
}

/// Made slides as an item, with their look.
pub fn slides_cue(doc: LiturgyDocument) -> Cue {
    let mut overrides = CueOverrides::default();
    *overrides.looks.get_mut(Output::Sanctuary) = slides_look();
    *overrides.looks.get_mut(Output::Stream) = slides_look();
    let mut cue = Cue::liturgy("", doc, None, overrides);
    cue.modified = true;
    cue
}

fn insert(engine: &Engine, after: Option<usize>, doc: LiturgyDocument) -> CmdResult<String> {
    engine.edit_service(|s| Ok(s.insert_cue(after.map(|a| a + 1).unwrap_or(s.cues.len()), slides_cue(doc))))
}

/// A new set of made slides: `document` (built by the slide maker or an
/// import), else one title slide called `title`.
pub fn slides_document(title: Option<String>, document: Option<LiturgyDocument>) -> LiturgyDocument {
    let title = title.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    let mut doc = document.unwrap_or_else(|| {
        let mut d = LiturgyDocument::default();
        d.blocks.push(Block::Heading { level: 1, text: title.clone().unwrap_or_else(|| "Title".into()) });
        d
    });
    if let Some(t) = title {
        doc.meta.title = t;
    }
    if doc.meta.title.is_empty() {
        doc.meta.title = "Slides".into();
    }
    doc.meta.category.get_or_insert(ElementCategory::Other);
    doc.meta.extra.retain(|(k, _)| k != SLIDE_MAKER);
    doc.meta.extra.push((SLIDE_MAKER.into(), "true".into()));
    doc
}

/// Adds made slides after cue index `after` (else at the end).
#[tauri::command]
pub fn slides_add(
    engine: State<'_, Engine>,
    after: Option<usize>,
    title: Option<String>,
    document: Option<LiturgyDocument>,
) -> CmdResult<String> {
    insert(&engine, after, slides_document(title, document))
}

/// Chooses a PDF (a presentation saved as PDF) and copies it into the media
/// library; the slides are made from its pages by the caller.
#[tauri::command]
pub async fn slides_pick_pdf(app: AppHandle) -> CmdResult<Option<PathBuf>> {
    let Some(file) = app.dialog().file().add_filter("PDF (a presentation saved as PDF)", &["pdf"]).blocking_pick_file()
    else {
        return Ok(None);
    };
    let src = file.into_path().map_err(|e| e.to_string())?;
    crate::media::copy_into_library(&app, &src).map(Some)
}

/// Which site a web video is on, if it is one the app can play.
pub fn video_site(url: &str) -> Option<&'static str> {
    let rest = url.trim().strip_prefix("https://").or_else(|| url.trim().strip_prefix("http://"))?;
    let host = rest.split(['/', '?', '#']).next()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").or_else(|| host.strip_prefix("m.")).unwrap_or(&host);
    match host {
        "youtube.com" | "youtu.be" | "youtube-nocookie.com" | "music.youtube.com" => Some("YouTube"),
        "facebook.com" | "fb.watch" | "web.facebook.com" => Some("Facebook"),
        "instagram.com" => Some("Instagram"),
        _ => None,
    }
}

/// Adds a YouTube, Facebook or Instagram video as its own item.
#[tauri::command]
pub fn web_video_add(
    engine: State<'_, Engine>,
    after: Option<usize>,
    url: String,
    title: Option<String>,
) -> CmdResult<String> {
    let url = url.trim().to_string();
    let site = video_site(&url).ok_or("paste a YouTube, Facebook or Instagram link")?;
    let mut doc = LiturgyDocument::default();
    doc.meta.title =
        title.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).unwrap_or_else(|| format!("{site} video"));
    doc.meta.category = Some(ElementCategory::Other);
    doc.meta.extra.push((WEB_VIDEO.into(), url));
    engine.edit_service(|s| {
        let at = after.map(|a| a + 1).unwrap_or(s.cues.len());
        let mut overrides = CueOverrides::default();
        let black = Look {
            background: Some("builtin:glass-charcoal".into()),
            layout: Some(Layout::Centered),
            tone: Some(TextTone::Light),
            fit: None,
            blank: None,
        };
        *overrides.looks.get_mut(Output::Sanctuary) = black.clone();
        *overrides.looks.get_mut(Output::Stream) = black;
        let mut cue = Cue::liturgy(s.unique_id(&doc.meta.title), doc, None, overrides);
        cue.modified = true;
        Ok(s.insert_cue(at, cue))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_sites() {
        assert_eq!(video_site("https://www.youtube.com/watch?v=abc"), Some("YouTube"));
        assert_eq!(video_site("https://youtu.be/abc"), Some("YouTube"));
        assert_eq!(video_site("https://www.facebook.com/stx/videos/123/"), Some("Facebook"));
        assert_eq!(video_site("https://fb.watch/xyz/"), Some("Facebook"));
        assert_eq!(video_site("https://www.instagram.com/reel/ABC/"), Some("Instagram"));
        assert_eq!(video_site("https://vimeo.com/1"), None);
        assert_eq!(video_site("youtube.com/watch?v=abc"), None);
    }

    #[test]
    fn made_slides_stay_as_made() {
        let doc: LiturgyDocument = "---\ntitle: Parish Picnic\n---\n# Parish Picnic\nSunday after the 10:30\n\n---\n\n## Bring\nA dish to share\n\n---\n\n![Map](/m/map.png){width}\n".parse().unwrap();
        let doc = slides_document(None, Some(doc));
        assert!(doc.meta.is_made_slides() && doc.meta.screen_only());
        let mut s = episco_core::Service::default();
        let mut cue = Cue::liturgy("p", doc, None, CueOverrides::default());
        cue.rebuild(&s.context());
        s.insert_cue(0, cue);
        let slides: Vec<_> = (0..s.cues[0].slide_count()).map(|i| s.cues[0].slide(i).unwrap().clone()).collect();
        assert_eq!(slides.len(), 3);
        assert_eq!((slides[0].level, slides[1].level, slides[2].level), (Some(1), Some(2), None));
        assert!(!slides[2].section_start);
        assert!(matches!(&slides[2].blocks[0], Block::Image { fit: Some(f), .. } if f == "width"));
        // The same on the stream: not run together.
        if let episco_core::CueContent::Liturgy { stream_slides, .. } = &s.cues[0].content {
            assert!(stream_slides.is_empty());
        }
        // Written back the same way.
        let text = episco_core::ewm::to_string(s.cues[0].document().unwrap());
        assert!(text.contains("![Map](/m/map.png){width}"), "{text}");
        assert!(text.contains("slide_maker: true"));
    }
}
