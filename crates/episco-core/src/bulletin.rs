//! Bulletins and officiant scripts, built from a service.
//!
//! [`bulletin`] and [`script`] turn a [`Service`] into a [`Doc`]: a cover, a
//! list of entries (one per item, each a few [`Piece`]s) and, for the
//! bulletin, a back page. The same `Doc` becomes the preview (HTML,
//! [`to_html`]) and the Word file ([`crate::docx`]), so they always agree.
//!
//! Each item appears in the bulletin in one of four [`BulletinStyle`]s:
//! a *reference line* (title, opening words, "BCP 323" on the right), *full
//! text*, *title only*, or *left out*. The defaults follow the parish's
//! bulletins: Prayer Book texts as reference lines, the propers and the
//! Prayers of the People in full, hymns as "title · Hymn 414".

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::calendar::{liturgical_day, Date};
use crate::category::ElementCategory;
use crate::document::{Block, LiturgyDocument, Span};
use crate::role::Role;
use crate::service::{Cue, CueContent, Service};

/// How an item appears in the bulletin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BulletinStyle {
    /// Title, opening words in italics, page or hymn number on the right.
    Reference,
    /// Every word, with speaker labels and the people's parts in bold.
    Full,
    /// The title and only what the people say, each with the short line
    /// that cues it ("The Word of the Lord." / "Thanks be to God.").
    Responses,
    /// Just the title (and reference).
    Title,
    /// Left out of the bulletin.
    Hidden,
}

impl BulletinStyle {
    pub fn parse(s: &str) -> Option<BulletinStyle> {
        match s {
            "reference" => Some(BulletinStyle::Reference),
            "full" => Some(BulletinStyle::Full),
            "responses" => Some(BulletinStyle::Responses),
            "title" => Some(BulletinStyle::Title),
            "hidden" => Some(BulletinStyle::Hidden),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            BulletinStyle::Reference => "reference",
            BulletinStyle::Full => "full",
            BulletinStyle::Responses => "responses",
            BulletinStyle::Title => "title",
            BulletinStyle::Hidden => "hidden",
        }
    }
}

/// The parish's details for covers and back pages (`parish.json` in the
/// parish library folder).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParishInfo {
    /// e.g. "The Cathedral of Saint James".
    pub name: String,
    /// A second line under the name, e.g. the city.
    pub place: String,
    /// Street address (lines separated by newlines).
    pub address: String,
    pub phone: String,
    pub website: String,
    pub email: String,
    /// Music licence notice, e.g. "All music in this bulletin is printed under CCLI #… or OneLicense #…".
    pub licence: String,
    /// "To our visitors and guests" text for the back page (paragraphs separated by blank lines).
    pub welcome: String,
    /// A logo image (PNG or JPEG) for the cover and back page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<PathBuf>,
    /// The cover title; default "The Holy Eucharist".
    pub liturgy_title: String,
    /// When the bulletin labels the people's parts "People" (their words
    /// are in bold either way).
    pub people_label: PeopleLabel,
}

/// When the bulletin prints the "People" label.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeopleLabel {
    /// Never: bold type shows what the people say.
    #[default]
    Never,
    /// On the first of the people's parts in each item.
    Once,
    /// Every time.
    Always,
}

/// Drops "People" labels as `mode` asks.
fn people_labels(pieces: &mut [Piece], mode: PeopleLabel) {
    let mut seen = false;
    for p in pieces {
        if let Piece::Speech { label, .. } = p {
            if label.as_deref() == Some(Role::People.label()) {
                if mode == PeopleLabel::Never || (mode == PeopleLabel::Once && seen) {
                    *label = None;
                }
                seen = true;
            }
        }
    }
}

/// A run of text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Run {
    pub text: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    /// Where the text comes from in the item, so an edit in the preview can
    /// be written back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<Src>,
}

/// A line of an item's document: block `block`, line `line` of it (`None`:
/// the whole block as one line, e.g. a rubric).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Src {
    pub block: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

impl Run {
    fn plain(t: impl Into<String>) -> Run {
        Run { text: t.into(), bold: false, italic: false, src: None }
    }
}

/// A line's runs, marked as coming from block `block`, line `line`. An empty
/// line still gets one (empty) run, so it can be typed into.
fn runs_at(line: &[Span], force_bold: bool, block: usize, line_no: usize) -> Vec<Run> {
    let src = Some(Src { block, line: Some(line_no) });
    let mut out: Vec<Run> =
        line.iter().map(|s| Run { text: s.text.clone(), bold: s.bold || force_bold, italic: s.italic, src }).collect();
    if out.is_empty() {
        out.push(Run { text: String::new(), bold: force_bold, italic: false, src });
    }
    out
}

fn lines_at(lines: &[Vec<Span>], force_bold: bool, block: usize) -> Vec<Vec<Run>> {
    lines.iter().enumerate().map(|(j, l)| runs_at(l, force_bold, block, j)).collect()
}

/// One printed piece of an entry.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Piece {
    /// A section of the service, e.g. "Holy Communion".
    Section { text: String },
    /// Title, opening words and reference on one line.
    Line {
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        incipit: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reference: Option<String>,
    },
    /// An instruction, in italics (red in the script).
    Rubric {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        src: Option<Src>,
    },
    /// Spoken text: a speaker label (if any) and lines.
    Speech {
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        lines: Vec<Vec<Run>>,
    },
    /// A psalm verse.
    Verse { number: String, lines: Vec<Vec<Run>> },
    /// Running text (a reading), one paragraph.
    Prose { runs: Vec<Run> },
    /// A picture, e.g. a line of music.
    Image { path: PathBuf, alt: String },
    /// Where a reading's text belongs but has not been added yet.
    MissingText { citation: String },
    /// Start a new page (script only).
    PageBreak,
}

/// One item of the service in the document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    /// The cue it comes from (to select it from the preview).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cue: Option<String>,
    pub style: BulletinStyle,
    pub pieces: Vec<Piece>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocKind {
    Bulletin,
    Script,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cover {
    pub title: String,
    /// "on the Twenty-Second Sunday after Pentecost".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day: Option<String>,
    /// "November 9, 2025 at 8 o'clock in the morning".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Doc {
    pub kind: DocKind,
    pub cover: Cover,
    pub entries: Vec<Entry>,
    /// The bulletin's back page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub back: Option<ParishInfo>,
}

// ---------------------------------------------------------------------------
// Dates and times
// ---------------------------------------------------------------------------

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// "08:00" → "8 o'clock in the morning"; "10:30" → "10:30 am"; "18:00" → "6 o'clock in the evening".
pub fn time_phrase(time: &str) -> Option<String> {
    let (h, m) = time.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.trim().parse().ok()?, m.trim().get(..2)?.parse().ok()?);
    if h > 23 || m > 59 {
        return None;
    }
    let h12 = match h % 12 {
        0 => 12,
        x => x,
    };
    if m == 0 {
        let part = match h {
            0..=11 => "in the morning",
            12 => "noon",
            13..=16 => "in the afternoon",
            _ => "in the evening",
        };
        if h == 12 {
            return Some("noon".into());
        }
        return Some(format!("{h12} o\u{2019}clock {part}"));
    }
    Some(format!("{h12}:{m:02} {}", if h < 12 { "am" } else { "pm" }))
}

fn long_date(d: Date) -> String {
    format!("{} {}, {}", MONTHS[(d.month - 1) as usize], d.day, d.year)
}

fn cover(service: &Service, parish: &ParishInfo) -> Cover {
    let date = service.date.as_deref().and_then(|d| d.parse::<Date>().ok());
    let day = date.map(|d| {
        let name = liturgical_day(d).name;
        let name = name.strip_prefix("The ").unwrap_or(&name).to_string();
        format!("on the {name}")
    });
    let when = date.map(|d| match service.time.as_deref().and_then(time_phrase) {
        Some(t) => format!("{} at {t}", long_date(d)),
        None => long_date(d),
    });
    let title = if parish.liturgy_title.trim().is_empty() {
        "The Holy Eucharist".to_string()
    } else {
        parish.liturgy_title.trim().to_string()
    };
    Cover { title, day, when, logo: parish.logo.clone() }
}

// ---------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------

fn is_bcp(reference: &str) -> bool {
    let r = reference.trim();
    r.starts_with("BCP") || r.starts_with("Book of Common Prayer")
}

/// The style an item has unless the plan says otherwise.
pub fn default_style(cue: &Cue) -> BulletinStyle {
    let Some(doc) = cue.document() else {
        return BulletinStyle::Title; // announcements
    };
    let meta = &doc.meta;
    if meta.extra("media").is_some() || meta.extra("blank").is_some() {
        return BulletinStyle::Hidden;
    }
    match cue.category {
        ElementCategory::Hymn | ElementCategory::Anthem => BulletinStyle::Reference,
        ElementCategory::Homily | ElementCategory::Announcements => BulletinStyle::Title,
        ElementCategory::EucharisticPrayer => BulletinStyle::Reference,
        _ if meta.citation.as_deref().is_some_and(is_bcp) => BulletinStyle::Reference,
        _ => BulletinStyle::Full,
    }
}

/// The style an item has in the bulletin.
pub fn style_of(cue: &Cue) -> BulletinStyle {
    cue.overrides.bulletin.unwrap_or_else(|| default_style(cue))
}

/// The opening words of an item: "Almighty God, unto whom all hearts are open…".
pub fn incipit(cue: &Cue) -> Option<String> {
    let doc = cue.document()?;
    let text = doc.blocks.iter().find_map(|b| match b {
        Block::Paragraph { role, .. } if !role.as_ref().is_some_and(Role::is_rubric) => {
            let t = b.plain_text().replace('\n', " ");
            (!t.trim().is_empty()).then_some(t)
        }
        _ => None,
    })?;
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut out = Vec::new();
    for (i, w) in words.iter().enumerate() {
        let clean = w.trim_matches(|c: char| c == '*' || c == '_');
        out.push(clean);
        let ends = clean.ends_with([',', ':', ';', '.', '?', '!']);
        if (ends && i >= 2) || out.len() >= 8 {
            break;
        }
    }
    let mut s = out.join(" ");
    while s.ends_with([',', ':', ';', '.', '?', '!']) {
        s.pop();
    }
    (!s.is_empty()).then(|| format!("{s}\u{2026}"))
}

/// What goes on the right of an item's line: "BCP 323", "Hymn 414", a composer, a preacher.
fn reference(cue: &Cue) -> Option<String> {
    let meta = &cue.document()?.meta;
    if let Some(n) = meta.hymn_number.as_deref().filter(|n| !n.is_empty()) {
        return Some(format!("Hymn {n}"));
    }
    for key in ["composer", "preacher"] {
        if let Some(v) = meta.extra(key).filter(|v| !v.is_empty()) {
            return Some(v.to_string());
        }
    }
    meta.citation.as_deref().filter(|c| is_bcp(c)).map(String::from)
}

/// Scripture (not a prayer book page) shown after the title: "Job 19:23-27a".
fn scripture(cue: &Cue) -> Option<String> {
    let c = cue.document()?.meta.citation.as_deref()?;
    (!is_bcp(c) && !c.is_empty() && !cue.title.contains(c)).then(|| c.to_string())
}

/// For a hymn or anthem, its own title in italics after the slot's name.
fn work_title(cue: &Cue) -> Option<String> {
    let meta = &cue.document()?.meta;
    (!meta.title.is_empty() && meta.title != cue.title).then(|| meta.title.clone())
}

fn resolve(src: &str, cue: &Cue, root: Option<&Path>) -> Option<PathBuf> {
    let p = crate::portable::resolve(src, root)?;
    let full = if p.is_absolute() { p } else { cue.source()?.parent()?.join(p) };
    full.is_file().then_some(full)
}

/// The full text of an item as pieces. `script`: rubrics are kept and
/// slide headings become section lines.
fn body(cue: &Cue, script: bool, root: Option<&Path>) -> Vec<Piece> {
    let Some(doc) = cue.document() else { return Vec::new() };
    let mut out = Vec::new();
    // Where a reading's text goes: the start of its last slide.
    let last_break = doc.blocks.iter().rposition(|b| matches!(b, Block::SlideBreak));
    let mut text_at = 0;
    // Reading text: slides that carry a paragraph on join the one before.
    let continues: Vec<usize> = doc
        .meta
        .extra(crate::bible::TEXT_CONTINUES)
        .map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_default();
    let mut text_slide = 0;
    for (i, b) in doc.blocks.iter().enumerate() {
        if Some(i) == last_break {
            text_at = out.len();
        }
        match b {
            Block::Heading { .. } | Block::SlideBreak => {}
            Block::Image { src, alt } => {
                if let Some(path) = resolve(src, cue, root) {
                    out.push(Piece::Image { path, alt: alt.clone() });
                }
            }
            Block::Paragraph { role, lines } => match role {
                Some(r) if r.is_rubric() => {
                    if script || cue.category != ElementCategory::Psalm {
                        out.push(Piece::Rubric {
                            text: b.plain_text().replace('\n', " "),
                            src: Some(Src { block: i, line: None }),
                        });
                    }
                }
                Some(Role::Custom(n)) if n.chars().next().is_some_and(|c| c.is_ascii_digit()) => {
                    out.push(Piece::Verse { number: n.clone(), lines: lines_at(lines, false, i) });
                }
                Some(Role::Custom(a)) if a.eq_ignore_ascii_case("antiphon") || a.eq_ignore_ascii_case("refrain") => {
                    out.push(Piece::Speech { label: None, lines: lines_at(lines, true, i) });
                }
                Some(r) => out.push(Piece::Speech {
                    label: Some(r.label().to_string()),
                    lines: lines_at(lines, r.is_congregational(), i),
                }),
                None => {
                    if cue.category == ElementCategory::Reading || cue.category == ElementCategory::Gospel {
                        // Reading text: each line a paragraph, except where a
                        // slide carries the one before on.
                        text_slide += 1;
                        for (j, l) in lines.iter().enumerate() {
                            let r = runs_at(l, false, i, j);
                            match out.last_mut() {
                                Some(Piece::Prose { runs: prev }) if j == 0 && continues.contains(&text_slide) => {
                                    prev.push(Run::plain(" "));
                                    prev.extend(r);
                                }
                                _ => out.push(Piece::Prose { runs: r }),
                            }
                        }
                    } else {
                        out.push(Piece::Speech { label: None, lines: lines_at(lines, false, i) });
                    }
                }
            },
        }
    }
    if needs_text(cue) {
        let citation = doc.meta.citation.clone().unwrap_or_default();
        out.insert(text_at.min(out.len()), Piece::MissingText { citation });
    }
    out
}

fn is_reading(cue: &Cue) -> bool {
    matches!(cue.category, ElementCategory::Reading | ElementCategory::Gospel)
}

/// A reading whose scripture text has not been pasted in yet.
pub fn needs_text(cue: &Cue) -> bool {
    is_reading(cue)
        && cue.document().is_some_and(|d| {
            d.meta.citation.as_deref().is_some_and(|c| !c.trim().is_empty())
                && !d.blocks.iter().any(|b| matches!(b, Block::Paragraph { role: None, .. }))
        })
}

fn is_people(p: &Piece) -> bool {
    match p {
        Piece::Speech { label: Some(l), .. } => l == Role::People.label(),
        Piece::Speech { label: None, lines } => lines.iter().flatten().all(|r| r.bold || r.text.trim().is_empty()),
        _ => false,
    }
}

fn words(p: &Piece) -> usize {
    match p {
        Piece::Speech { lines, .. } => lines.iter().flatten().map(|r| r.text.split_whitespace().count()).sum(),
        _ => usize::MAX,
    }
}

/// Only what the people say, each with the leader's line before it when that
/// line is short enough to be a cue ("The Lord be with you.").
fn responses(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    let mut prev: Option<Piece> = None;
    for p in pieces {
        if is_people(&p) {
            if let Some(cue) = prev.take().filter(|c| matches!(c, Piece::Speech { .. }) && words(c) <= 16) {
                out.push(cue);
            }
            out.push(p);
        } else {
            prev = Some(p);
        }
    }
    out
}

fn title_line(cue: &Cue, style: BulletinStyle) -> Piece {
    let work = work_title(cue);
    Piece::Line {
        title: cue.title.clone(),
        detail: scripture(cue),
        incipit: match style {
            BulletinStyle::Reference => work.or_else(|| incipit(cue)),
            BulletinStyle::Title => work,
            _ => work,
        },
        reference: reference(cue),
    }
}

fn entry(cue: &Cue, style: BulletinStyle, script: bool, root: Option<&Path>) -> Entry {
    let mut pieces = Vec::new();
    match style {
        BulletinStyle::Hidden => {}
        BulletinStyle::Reference | BulletinStyle::Title => pieces.push(title_line(cue, style)),
        BulletinStyle::Full => {
            pieces.push(Piece::Line {
                title: cue.title.clone(),
                detail: scripture(cue),
                incipit: work_title(cue),
                reference: reference(cue),
            });
            pieces.extend(body(cue, script, root));
        }
        BulletinStyle::Responses => {
            pieces.push(Piece::Line {
                title: cue.title.clone(),
                detail: scripture(cue),
                incipit: work_title(cue),
                reference: reference(cue),
            });
            pieces.extend(responses(body(cue, script, root)));
        }
    }
    Entry { cue: Some(cue.id.clone()), style, pieces }
}

/// The congregation's bulletin for a service.
pub fn bulletin(service: &Service, parish: &ParishInfo) -> Doc {
    let entries = service
        .cues
        .iter()
        .map(|cue| {
            let mut e = entry(cue, style_of(cue), false, service.library_root.as_deref());
            people_labels(&mut e.pieces, parish.people_label);
            e
        })
        .filter(|e| e.style != BulletinStyle::Hidden)
        .collect();
    Doc { kind: DocKind::Bulletin, cover: cover(service, parish), entries, back: Some(parish.clone()) }
}

/// The officiant's script: every word of every item, rubrics included, the
/// Great Thanksgiving starting a new page.
pub fn script(service: &Service, parish: &ParishInfo) -> Doc {
    let mut entries = Vec::new();
    for cue in &service.cues {
        let doc = cue.document();
        let media = doc.is_some_and(|d| d.meta.extra("media").is_some() || d.meta.extra("blank").is_some());
        let style = match &cue.content {
            CueContent::Announcements { .. } => BulletinStyle::Title,
            _ if media => BulletinStyle::Title,
            _ if cue.category == ElementCategory::Hymn && doc.is_some_and(|d| d.blocks.len() <= 1) => {
                BulletinStyle::Title
            }
            _ => BulletinStyle::Full,
        };
        let mut e = entry(cue, style, true, service.library_root.as_deref());
        if cue.category == ElementCategory::EucharisticPrayer
            && !entries.iter().any(|x: &Entry| x.pieces.contains(&Piece::PageBreak))
        {
            e.pieces.insert(0, Piece::PageBreak);
        }
        entries.push(e);
    }
    Doc { kind: DocKind::Script, cover: cover(service, parish), entries, back: None }
}

// ---------------------------------------------------------------------------
// Editing in the preview
// ---------------------------------------------------------------------------

/// A change typed into the bulletin or script preview, to one line of an item.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum LineEdit {
    /// Replace the line's text.
    Set { spans: Vec<Span> },
    /// Enter: the line keeps `spans`, and `rest` becomes a new line after it.
    Split { spans: Vec<Span>, rest: Vec<Span> },
    /// Backspace at the start of a line: join it onto the line before.
    JoinPrevious,
    /// Remove the line (its block too, if it was the last line).
    Remove,
}

/// Joins neighbouring spans with the same formatting; turns non-breaking
/// spaces (from typing in the browser) into spaces.
fn tidy(spans: Vec<Span>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    for mut s in spans {
        s.text = s.text.replace('\u{a0}', " ").replace(['\n', '\r'], " ");
        if s.text.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.bold == s.bold && last.italic == s.italic => last.text.push_str(&s.text),
            _ => out.push(s),
        }
    }
    out
}

/// Applies an edit from the preview to an item's document.
pub fn edit_line(
    doc: &LiturgyDocument,
    block: usize,
    line: Option<usize>,
    edit: LineEdit,
) -> Result<LiturgyDocument, String> {
    let mut doc = doc.clone();
    let stale = || "the item changed while you were typing: try again".to_string();
    let Some(Block::Paragraph { lines, .. }) = doc.blocks.get_mut(block) else { return Err(stale()) };
    let mut remove_block = false;
    match (line, edit) {
        // A rubric: the whole block as one line.
        (None, LineEdit::Set { spans } | LineEdit::Split { spans, .. }) => *lines = vec![tidy(spans)],
        (None, LineEdit::Remove) => remove_block = true,
        (None, LineEdit::JoinPrevious) => {}
        (Some(j), edit) => {
            if j >= lines.len() {
                return Err(stale());
            }
            match edit {
                LineEdit::Set { spans } => lines[j] = tidy(spans),
                LineEdit::Split { spans, rest } => {
                    lines[j] = tidy(spans);
                    lines.insert(j + 1, tidy(rest));
                }
                LineEdit::JoinPrevious if j > 0 => {
                    let cur = lines.remove(j);
                    let mut joined = std::mem::take(&mut lines[j - 1]);
                    joined.extend(cur);
                    lines[j - 1] = tidy(joined);
                }
                LineEdit::JoinPrevious => {}
                LineEdit::Remove => {
                    lines.remove(j);
                    remove_block = lines.is_empty();
                }
            }
        }
    }
    if remove_block {
        doc.blocks.remove(block);
        doc.blocks.dedup_by(|a, b| matches!(a, Block::SlideBreak) && matches!(b, Block::SlideBreak));
    }
    Ok(doc)
}

// ---------------------------------------------------------------------------
// HTML (the preview, and later the web bulletin)
// ---------------------------------------------------------------------------

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Runs as HTML; consecutive runs from the same line of the item go in one
/// `<span data-src="block:line">` that the preview makes editable.
fn html_runs(rs: &[Run]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < rs.len() {
        let src = rs[i].src;
        let mut j = i;
        while j < rs.len() && rs[j].src == src {
            j += 1;
        }
        let inner = html_plain_runs(&rs[i..j]);
        match src {
            Some(s) => out.push_str(&format!("<span class=\"ed\" data-src=\"{}\">{inner}</span>", src_attr(s))),
            None => out.push_str(&inner),
        }
        i = j;
    }
    out
}

fn src_attr(s: Src) -> String {
    match s.line {
        Some(l) => format!("{}:{l}", s.block),
        None => s.block.to_string(),
    }
}

fn html_plain_runs(rs: &[Run]) -> String {
    rs.iter()
        .map(|r| {
            let mut t = escape(&r.text);
            if r.italic {
                t = format!("<i>{t}</i>");
            }
            if r.bold {
                t = format!("<b>{t}</b>");
            }
            t
        })
        .collect()
}

/// The document as HTML (body content only; styles in ui/css/bulletin.css).
/// Pictures carry their file path in `data-path` for the app to load.
pub fn to_html(doc: &Doc) -> String {
    let mut h = String::new();
    let kind = match doc.kind {
        DocKind::Bulletin => "bulletin",
        DocKind::Script => "script",
    };
    h.push_str(&format!("<article class=\"doc {kind}\">"));
    // Cover
    h.push_str("<header class=\"cover\">");
    if let Some(logo) = &doc.cover.logo {
        h.push_str(&format!("<img class=\"logo\" data-path=\"{}\" alt=\"\">", escape(&logo.to_string_lossy())));
    }
    h.push_str(&format!("<h1>{}</h1>", escape(&doc.cover.title)));
    if let Some(d) = &doc.cover.day {
        h.push_str(&format!("<p class=\"day\">{}</p>", escape(d)));
    }
    if let Some(w) = &doc.cover.when {
        h.push_str(&format!("<p class=\"when\">{}</p>", escape(w)));
    }
    h.push_str("</header>");
    for e in &doc.entries {
        let cue = e.cue.as_deref().map(|c| format!(" data-cue=\"{}\"", escape(c))).unwrap_or_default();
        h.push_str(&format!("<section class=\"entry {}\"{cue}>", e.style.key()));
        for p in &e.pieces {
            match p {
                Piece::Section { text } => h.push_str(&format!("<h2>{}</h2>", escape(text))),
                Piece::Line { title, detail, incipit, reference } => {
                    h.push_str("<p class=\"line\"><span class=\"lead\">");
                    h.push_str(&format!("<b class=\"ed-title\">{}</b>", escape(title)));
                    if let Some(d) = detail {
                        h.push_str(&format!(" <span class=\"detail\">{}</span>", escape(d)));
                    }
                    if let Some(i) = incipit {
                        h.push_str(&format!(" <i>{}</i>", escape(i)));
                    }
                    h.push_str("</span>");
                    if let Some(r) = reference {
                        h.push_str(&format!("<span class=\"ref\">{}</span>", escape(r)));
                    }
                    h.push_str("</p>");
                }
                Piece::Rubric { text, src } => match src {
                    Some(s) => h.push_str(&format!(
                        "<p class=\"rubric\"><span class=\"ed\" data-src=\"{}\">{}</span></p>",
                        src_attr(*s),
                        escape(text)
                    )),
                    None => h.push_str(&format!("<p class=\"rubric\">{}</p>", escape(text))),
                },
                Piece::Speech { label, lines } => {
                    h.push_str("<div class=\"speech\">");
                    h.push_str(&format!("<span class=\"who\">{}</span>", escape(label.as_deref().unwrap_or(""))));
                    let body: Vec<String> = lines.iter().map(|l| html_runs(l)).collect();
                    h.push_str(&format!("<span class=\"said\">{}</span></div>", body.join("<br>")));
                }
                Piece::Verse { number, lines } => {
                    let body: Vec<String> = lines.iter().map(|l| html_runs(l)).collect();
                    h.push_str(&format!(
                        "<div class=\"verse\"><span class=\"num\">{}</span><span class=\"said\">{}</span></div>",
                        escape(number),
                        body.join("<br>")
                    ));
                }
                Piece::Prose { runs } => h.push_str(&format!("<p class=\"prose\">{}</p>", html_runs(runs))),
                Piece::Image { path, alt } => h.push_str(&format!(
                    "<p class=\"music\"><img data-path=\"{}\" alt=\"{}\"></p>",
                    escape(&path.to_string_lossy()),
                    escape(alt)
                )),
                Piece::MissingText { citation } => h.push_str(&format!(
                    "<p class=\"missing\" data-missing=\"{}\">[The text of {} goes here. Add it with Reading texts\u{2026}]</p>",
                    escape(citation),
                    escape(citation)
                )),
                Piece::PageBreak => h.push_str("<hr class=\"page-break\">"),
            }
        }
        h.push_str("</section>");
    }
    if let Some(p) = &doc.back {
        h.push_str("<footer class=\"back\">");
        if !p.welcome.trim().is_empty() {
            h.push_str("<h2>To Our Visitors and Guests\u{2026}</h2>");
            for para in p.welcome.split("\n\n").filter(|x| !x.trim().is_empty()) {
                h.push_str(&format!("<p>{}</p>", escape(para.trim())));
            }
        }
        h.push_str("<div class=\"contact\">");
        if let Some(logo) = &p.logo {
            h.push_str(&format!("<img class=\"logo\" data-path=\"{}\" alt=\"\">", escape(&logo.to_string_lossy())));
        }
        for (cls, v) in [("name", &p.name), ("place", &p.place)] {
            if !v.trim().is_empty() {
                h.push_str(&format!("<p class=\"{cls}\">{}</p>", escape(v.trim())));
            }
        }
        for v in [&p.address, &p.phone, &p.website, &p.email] {
            for line in v.lines().filter(|l| !l.trim().is_empty()) {
                h.push_str(&format!("<p>{}</p>", escape(line.trim())));
            }
        }
        if !p.licence.trim().is_empty() {
            h.push_str(&format!("<p class=\"licence\">{}</p>", escape(p.licence.trim())));
        }
        h.push_str("</div></footer>");
    }
    h.push_str("</article>");
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::CueOverrides;
    use crate::LiturgyDocument;

    fn cue(src: &str) -> Cue {
        let doc: LiturgyDocument = src.parse().unwrap();
        Cue::liturgy("", doc, None, CueOverrides::default())
    }

    #[test]
    fn times_read_like_the_parish_bulletins() {
        assert_eq!(time_phrase("08:00").as_deref(), Some("8 o\u{2019}clock in the morning"));
        assert_eq!(time_phrase("10:30").as_deref(), Some("10:30 am"));
        assert_eq!(time_phrase("17:00").as_deref(), Some("5 o\u{2019}clock in the evening"));
        assert_eq!(time_phrase("12:00").as_deref(), Some("noon"));
        assert_eq!(time_phrase("nonsense"), None);
    }

    #[test]
    fn styles_and_lines() {
        let purity = cue("---\ntitle: Collect for Purity\ntype: collect\ncitation: BCP 323\n---\n[Celebrant] Almighty God, unto whom all hearts are open, all desires known.\n[People] **Amen.**\n");
        assert_eq!(default_style(&purity), BulletinStyle::Reference);
        assert_eq!(incipit(&purity).as_deref(), Some("Almighty God, unto whom all hearts are open\u{2026}"));
        let acclamation = cue("---\ntitle: Opening Acclamation\ncitation: BCP 323\n---\n[Celebrant] Blessed be God: Father, Son, and Holy Spirit.\n");
        assert_eq!(incipit(&acclamation).as_deref(), Some("Blessed be God\u{2026}"));
        let line = title_line(&purity, BulletinStyle::Reference);
        assert_eq!(
            line,
            Piece::Line {
                title: "Collect for Purity".into(),
                detail: None,
                incipit: Some("Almighty God, unto whom all hearts are open\u{2026}".into()),
                reference: Some("BCP 323".into()),
            }
        );

        let reading = cue("---\ntitle: First Reading\ntype: reading\ncitation: Job 19:23-27a\n---\n# The First Reading\n\n---\n\nO that my words were written down!\n\n---\n\n[Reader] The Word of the Lord.\n[People] **Thanks be to God.**\n");
        assert_eq!(default_style(&reading), BulletinStyle::Full);
        let e = entry(&reading, BulletinStyle::Full, false, None);
        assert!(matches!(&e.pieces[0], Piece::Line { detail: Some(d), .. } if d == "Job 19:23-27a"));
        assert!(matches!(&e.pieces[1], Piece::Prose { .. }));
        assert!(matches!(&e.pieces[3], Piece::Speech { label: Some(l), lines } if l == "People" && lines[0][0].bold));

        let hymn = cue("---\ntitle: God, My King, Thy Might Confessing\ntype: hymn\nhymn_number: \"414\"\n---\n[1] (words from the parish's licensed source)\n");
        let mut hymn = hymn;
        hymn.overrides.title = Some("Opening Hymn".into());
        hymn.rebuild(&Default::default());
        assert_eq!(default_style(&hymn), BulletinStyle::Reference);
        assert_eq!(
            title_line(&hymn, BulletinStyle::Reference),
            Piece::Line {
                title: "Opening Hymn".into(),
                detail: None,
                incipit: Some("God, My King, Thy Might Confessing".into()),
                reference: Some("Hymn 414".into()),
            }
        );
    }

    #[test]
    fn responses_only_and_missing_reading_text() {
        let gospel = cue("---\ntitle: The Holy Gospel\ntype: gospel\ncitation: Matthew 25:1-13\n---\n# The Holy Gospel\n\n---\n\n[Deacon] The Holy Gospel of our Lord Jesus Christ according to Matthew.\n[People] **Glory to you, Lord Christ.**\n\n---\n\n[Deacon] The Gospel of the Lord.\n[People] **Praise to you, Lord Christ.**\n");
        assert!(needs_text(&gospel));
        let full = entry(&gospel, BulletinStyle::Full, false, None);
        // The placeholder sits where the text goes: before the closing response.
        assert!(
            matches!(&full.pieces[3], Piece::MissingText { citation } if citation == "Matthew 25:1-13"),
            "{:?}",
            full.pieces
        );
        assert!(matches!(&full.pieces[4], Piece::Speech { label: Some(l), .. } if l == "Deacon"));
        assert!(to_html(&Doc {
            kind: DocKind::Bulletin,
            cover: Cover { title: String::new(), day: None, when: None, logo: None },
            entries: vec![full],
            back: None
        })
        .contains("The text of Matthew 25:1-13 goes here"));

        let r = entry(&gospel, BulletinStyle::Responses, false, None);
        let said: Vec<String> = r.pieces[1..]
            .iter()
            .map(|p| match p {
                Piece::Speech { lines, .. } => lines[0].iter().map(|x| x.text.as_str()).collect(),
                other => format!("{other:?}"),
            })
            .collect();
        assert_eq!(
            said,
            [
                "The Holy Gospel of our Lord Jesus Christ according to Matthew.",
                "Glory to you, Lord Christ.",
                "The Gospel of the Lord.",
                "Praise to you, Lord Christ."
            ]
        );

        // A long prayer is left out; only the people's Amen remains.
        let collect = cue("---\ntitle: Collect of the Day\ntype: collect\n---\n[Priest] Almighty God, you have knit together your elect in one communion and fellowship in the mystical body of your Son Christ our Lord: Give us grace so to follow your blessed saints.\n[People] **Amen.**\n");
        let r = entry(&collect, BulletinStyle::Responses, false, None);
        assert_eq!(r.pieces.len(), 2);
        assert!(is_people(&r.pieces[1]));
        assert!(!needs_text(&collect));
    }

    #[test]
    fn edits_from_the_preview() {
        let doc: LiturgyDocument =
            "---\ntitle: Salutation\n---\n[Rubric] *The people respond.*\n[Celebrant] The Lord be with you.\nLet us pray.\n[People] **And also with you.**\n"
                .parse()
                .unwrap();
        let c = Cue::liturgy("", doc.clone(), None, CueOverrides::default());
        // Every printed line knows where it came from.
        let html = to_html(&Doc {
            kind: DocKind::Script,
            cover: cover(&Service::default(), &ParishInfo::default()),
            entries: vec![entry(&c, BulletinStyle::Full, true, None)],
            back: None,
        });
        assert!(html.contains("data-src=\"0\">The people respond."), "{html}");
        assert!(
            html.contains(
                "data-src=\"1:0\">The Lord be with you.</span><br><span class=\"ed\" data-src=\"1:1\">Let us pray."
            ),
            "{html}"
        );
        assert!(html.contains("data-src=\"2:0\"><b>And also with you.</b>"), "{html}");

        let text = |d: &LiturgyDocument, b: usize| d.blocks[b].plain_text();
        let set = edit_line(
            &doc,
            1,
            Some(0),
            LineEdit::Set { spans: vec![Span::plain("The Lord\u{a0}be "), Span::plain("with you all.")] },
        )
        .unwrap();
        assert_eq!(text(&set, 1), "The Lord be with you all.\nLet us pray.");
        assert_eq!(set.blocks[1], Block::speech(Role::Celebrant, ["The Lord be with you all.", "Let us pray."]));
        let split = edit_line(
            &doc,
            1,
            Some(1),
            LineEdit::Split { spans: vec![Span::plain("Let us")], rest: vec![Span::bold("pray.")] },
        )
        .unwrap();
        assert_eq!(text(&split, 1), "The Lord be with you.\nLet us\npray.");
        let joined = edit_line(&split, 1, Some(2), LineEdit::JoinPrevious).unwrap();
        assert_eq!(text(&joined, 1), "The Lord be with you.\nLet uspray.");
        let rubric = edit_line(&doc, 0, None, LineEdit::Set { spans: vec![Span::italic("All stand.")] }).unwrap();
        assert_eq!(text(&rubric, 0), "All stand.");
        let gone = edit_line(&doc, 2, Some(0), LineEdit::Remove).unwrap();
        assert_eq!(gone.blocks.len(), 2);
        assert!(edit_line(&doc, 9, Some(0), LineEdit::Remove).is_err());
        assert!(edit_line(&doc, 1, Some(5), LineEdit::Remove).is_err());
    }

    #[test]
    fn people_label_never_once_or_always() {
        let mut s = Service::default();
        s.insert_cue(0, cue("---\ntitle: Salutation\n---\n[Rubric] *The people respond.*\n[Celebrant] The Lord be with you.\n[People] **And also with you.**\n[Celebrant] Let us pray.\n[People] **Amen.**\n"));
        let labels = |mode| {
            let parish = ParishInfo { people_label: mode, ..Default::default() };
            let b = bulletin(&s, &parish);
            b.entries[0]
                .pieces
                .iter()
                .filter_map(|p| match p {
                    Piece::Speech { label, .. } => Some(label.clone().unwrap_or_default()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(labels(PeopleLabel::Never), ["Celebrant", "", "Celebrant", ""]);
        assert_eq!(labels(PeopleLabel::Once), ["Celebrant", "People", "Celebrant", ""]);
        assert_eq!(labels(PeopleLabel::Always), ["Celebrant", "People", "Celebrant", "People"]);
        // The script keeps every label.
        let sc = script(&s, &ParishInfo::default());
        assert!(
            sc.entries[0]
                .pieces
                .iter()
                .filter(|p| matches!(p, Piece::Speech { label: Some(l), .. } if l == "People"))
                .count()
                == 2
        );
    }

    #[test]
    fn bulletin_and_script_from_a_service() {
        let mut s = Service { date: Some("2025-11-09".into()), time: Some("08:00".into()), ..Default::default() };
        s.insert_cue(0, cue("---\ntitle: Collect for Purity\ncitation: BCP 323\n---\n[Celebrant] Almighty God, unto whom all hearts are open.\n[Rubric] *Then the Gloria.*\n"));
        s.insert_cue(1, cue("---\ntitle: Collect of the Day\ntype: collect\n---\n[Priest] O God, whose blessed Son came into the world.\n[People] **Amen.**\n"));
        s.insert_cue(2, cue("---\ntitle: The Great Thanksgiving\ntype: eucharistic_prayer\ncitation: BCP 333\n---\n[Celebrant] The Lord be with you.\n"));
        s.insert_cue(3, cue("---\ntitle: Clip\nmedia: /x.png\n---\n"));
        let parish = ParishInfo { name: "St. Example".into(), ..Default::default() };

        let b = bulletin(&s, &parish);
        assert_eq!(b.cover.day.as_deref(), Some("on the Twenty-Second Sunday after Pentecost"));
        assert_eq!(b.cover.when.as_deref(), Some("November 9, 2025 at 8 o\u{2019}clock in the morning"));
        assert_eq!(b.entries.len(), 3, "the media item is left out");
        assert_eq!(b.entries[0].style, BulletinStyle::Reference);
        assert_eq!(b.entries[1].style, BulletinStyle::Full);

        // An override from the plan wins.
        s.cues[0].overrides.bulletin = Some(BulletinStyle::Hidden);
        assert_eq!(bulletin(&s, &parish).entries.len(), 2);

        let sc = script(&s, &parish);
        assert!(sc.entries[0].pieces.iter().any(|p| matches!(p, Piece::Rubric { .. })), "the script keeps rubrics");
        assert!(sc.entries[2].pieces[0] == Piece::PageBreak, "the Great Thanksgiving starts a page");
        assert!(sc.back.is_none());

        let html = to_html(&b);
        assert!(html.contains("<span class=\"ref\">BCP 323</span>") || html.contains("Collect of the Day"));
        assert!(html.contains("data-cue=\"collect-of-the-day\""));
        assert!(html.contains("St. Example"));
    }
}
