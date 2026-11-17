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
use crate::document::{split_indent, with_indent, Block, LiturgyDocument, Span};
use crate::role::Role;
use crate::service::{Cue, CueContent, Service};
use crate::shows::Place;

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
    /// How often a speaker's label is printed in the bulletin and script.
    pub speaker_labels: SpeakerLabels,
    /// The sheet the bulletin is printed on and folded.
    pub paper: BulletinPaper,
    /// The parish's usual service times on a Sunday, "08:00", "10:30".
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub service_times: Vec<String>,
    /// The clergy, offered as preachers in a sermon's details.
    pub rector: Clergy,
    pub assistant: Clergy,
    pub bishop: Clergy,
}

/// The sheet a bulletin is printed on, folded in half into a booklet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BulletinPaper {
    /// 11 × 17 in, folded: 8½ × 11 in pages.
    #[default]
    Tabloid,
    /// 8½ × 11 in, folded: 5½ × 8½ in pages.
    Letter,
    /// 8½ × 14 in (legal), folded: 7 × 8½ in pages.
    Legal,
}

impl BulletinPaper {
    pub fn key(self) -> &'static str {
        match self {
            BulletinPaper::Tabloid => "tabloid",
            BulletinPaper::Letter => "letter",
            BulletinPaper::Legal => "legal",
        }
    }
}

/// One of the parish's clergy: "The Very Rev.", "Jane Doe", "Rector".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Clergy {
    /// Their style or title: "The Rev.", "The Very Rev.", "The Rt. Rev.".
    pub honorific: String,
    pub name: String,
    /// Their position, printed after the name ("Rector").
    pub title: String,
}

/// "The Very Rev. Jane Doe, Rector" from a sermon's details.
pub fn preacher_line(honorific: Option<&str>, name: &str, position: Option<&str>) -> String {
    let mut s = String::new();
    if let Some(h) = honorific.map(str::trim).filter(|h| !h.is_empty()) {
        s.push_str(h);
        s.push(' ');
    }
    s.push_str(name.trim());
    if let Some(p) = position.map(str::trim).filter(|p| !p.is_empty()) {
        s.push_str(", ");
        s.push_str(p);
    }
    s
}

/// How often the bulletin and script print a speaker's label.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeakerLabels {
    /// Once at the start of each block by the same speaker: the Creed's
    /// paragraphs share one "All", a run of "People" responses one label.
    #[default]
    Once,
    /// On every paragraph.
    Every,
}

/// Drops a speaker's label where the same speaker carries on: the label
/// prints at the start of each block of text by one speaker. Rubrics and
/// pictures between do not break the block.
/// With a bishop presiding, the Celebrant's and Priest's parts are the
/// Bishop's; with no deacon, the Deacon's are a Priest's.
fn presider_labels(pieces: &mut [Piece], service: &Service) {
    if !service.bishop && !service.no_deacon {
        return;
    }
    for p in pieces {
        if let Piece::Speech { label: Some(l), .. } = p {
            let role = Role::parse(l);
            let said_by = role.at_service(service.bishop, service.no_deacon);
            if said_by != role {
                *l = said_by.label().to_string();
            }
        }
    }
}

fn speaker_labels(pieces: &mut [Piece], mode: SpeakerLabels) {
    if mode == SpeakerLabels::Every {
        return;
    }
    let mut last: Option<String> = None;
    for p in pieces {
        match p {
            Piece::Speech { label: Some(l), .. } => {
                if last.as_deref() == Some(l.as_str()) {
                    let Piece::Speech { label, .. } = p else { unreachable!() };
                    *label = None;
                } else {
                    last = Some(l.clone());
                }
            }
            Piece::Rubric { .. } | Piece::Image { .. } => {}
            _ => last = None,
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
    // The indent is printed as an indent, not as tabs (see `indents_of`).
    let (_, line) = split_indent(line);
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

/// Each line's indent as written (its leading tabs); empty when none is.
fn indents_of(lines: &[Vec<Span>]) -> Vec<usize> {
    let v: Vec<usize> = lines.iter().map(|l| split_indent(l).0).collect();
    if v.iter().all(|&n| n == 0) {
        Vec::new()
    } else {
        v
    }
}

/// A psalm verse's indents: both halves start level (a half too long for
/// the column carries on indented under itself), and every other verse is
/// a step in.
fn verse_indents(number: &str, lines: &[Vec<Span>]) -> Vec<usize> {
    let even = number.trim_end_matches(|c: char| c.is_ascii_alphabetic()).parse::<u32>().is_ok_and(|n| n % 2 == 0);
    lines.iter().map(|l| split_indent(l).0 + usize::from(even)).collect()
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
        /// Lines that run on from the line before (a paragraph the slides
        /// split), printed after a space rather than on a new line.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        joins: Vec<usize>,
        /// Each line's indent in steps (empty: none). Lines too long for the
        /// column hang under themselves.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        indents: Vec<usize>,
    },
    /// A psalm verse, with each line's indent, and what is set at the right
    /// margin at the end of its last line ("Antiphon": sung next).
    Verse {
        number: String,
        lines: Vec<Vec<Run>>,
        indents: Vec<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tail: Option<String>,
    },
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

/// A clock time as on a sign: "10:30 a.m.", "8:00 a.m.", "12:00 p.m.".
pub fn clock_time(time: &str) -> Option<String> {
    let (h, m) = time.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.trim().parse().ok()?, m.trim().get(..2)?.parse().ok()?);
    if h > 23 || m > 59 {
        return None;
    }
    let h12 = match h % 12 {
        0 => 12,
        x => x,
    };
    Some(format!("{h12}:{m:02} {}", if h < 12 { "a.m." } else { "p.m." }))
}

/// The stream's welcome slide (before the service).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Welcome {
    /// "The Holy Eucharist on the Twenty-Fourth Sunday after Pentecost,
    /// November 8, 2026, 10:30 a.m."
    pub line: String,
    /// "Welcome to the Cathedral of St. James" (or just "Welcome").
    pub greeting: String,
}

/// The welcome slide's words for a service and parish.
pub fn welcome(service: &Service, parish: &ParishInfo) -> Welcome {
    let c = cover(service, parish);
    let date = service.date.as_deref().and_then(|d| d.parse::<Date>().ok()).map(long_date);
    let time = service.time.as_deref().and_then(clock_time);
    // A service named for its day reads like the bulletin's cover; an
    // event (or a service given its own title) goes by that title.
    let day_name = service.date.as_deref().and_then(|d| d.parse::<Date>().ok()).map(|d| liturgical_day(d).name);
    let title = service.title.trim();
    let own_title = service.liturgy.is_none() && !title.is_empty() && day_name.as_deref() != Some(title);
    let head = match c.day {
        _ if own_title => title.to_string(),
        Some(day) => format!("{} {day}", c.title),
        None => c.title,
    };
    let line = [Some(head), date, time].into_iter().flatten().collect::<Vec<_>>().join(", ");
    let name = parish.name.trim();
    let greeting = match name {
        "" => "Welcome".to_string(),
        // "The Cathedral of St. James" → "Welcome to the Cathedral of St. James".
        n => match n.strip_prefix("The ") {
            Some(rest) => format!("Welcome to the {rest}"),
            None => format!("Welcome to {n}"),
        },
    };
    Welcome { line, greeting }
}

/// A Eucharistic Prayer with the service's Proper Preface in its place (as
/// on the slides); any other item as it is.
fn with_preface<'a>(cue: &'a Cue, service: &Service) -> std::borrow::Cow<'a, Cue> {
    if cue.category == ElementCategory::EucharisticPrayer {
        if let Some(doc) = cue.document().and_then(|d| crate::service::preface_in(d, service.preface_entry())) {
            let mut c = cue.clone();
            if let CueContent::Liturgy { document, .. } = &mut c.content {
                **document = doc;
            }
            return std::borrow::Cow::Owned(c);
        }
    }
    std::borrow::Cow::Borrowed(cue)
}

/// The prelude's or postlude's line (at the head or the end of the
/// bulletin and script): its title and composer, or "Prelude" alone, or
/// nothing (left out, or not named yet). The script names the piece
/// whatever the bulletin shows.
fn music_entry(label: &str, music: Option<&crate::music::Music>, script: bool) -> Option<Entry> {
    let m = music?;
    let style = if script { BulletinStyle::Reference } else { m.style() };
    let named = !m.is_empty();
    let line = |incipit: Option<String>, reference: Option<String>| Piece::Line {
        title: label.into(),
        detail: None,
        incipit,
        reference,
    };
    let piece = match style {
        BulletinStyle::Hidden => return None,
        BulletinStyle::Title => line(None, None),
        _ if !named => return None,
        _ => line(
            (!m.title.trim().is_empty()).then(|| m.title.trim().to_string()),
            (!m.composer.trim().is_empty()).then(|| m.composer.trim().to_string()),
        ),
    };
    Some(Entry { cue: None, style, pieces: vec![piece] })
}

fn prelude_entry(service: &Service, script: bool) -> Option<Entry> {
    music_entry("Prelude", service.prelude.as_ref(), script)
}

fn postlude_entry(service: &Service, script: bool) -> Option<Entry> {
    music_entry("Postlude", service.postlude.as_ref(), script)
}

fn cover(service: &Service, parish: &ParishInfo) -> Cover {
    let date = service.date.as_deref().and_then(|d| d.parse::<Date>().ok());
    // A special service names itself ("Good Friday"); else the parish's
    // title for the Eucharist.
    let title = match service.liturgy.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => t.to_string(),
        None if parish.liturgy_title.trim().is_empty() => "The Holy Eucharist".to_string(),
        None => parish.liturgy_title.trim().to_string(),
    };
    let day = date.and_then(|d| {
        let name = liturgical_day(d).name;
        if name.eq_ignore_ascii_case(&title) {
            return None; // "Good Friday" on Good Friday
        }
        Some(match name.strip_prefix("The ") {
            Some(rest) => format!("on the {rest}"),
            None => format!("on {name}"),
        })
    });
    let when = date.map(|d| match service.time.as_deref().and_then(time_phrase) {
        Some(t) => format!("{} at {t}", long_date(d)),
        None => long_date(d),
    });
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
    if meta.screen_only() {
        return BulletinStyle::Hidden;
    }
    // An item can say how it is usually printed (`bulletin: full`), e.g.
    // the occasional rites, whose words the congregation does not know.
    if let Some(style) = meta.extra("bulletin").and_then(BulletinStyle::parse) {
        return style;
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
    if let Some(r) = meta.hymn_reference() {
        return Some(r);
    }
    // The preacher and their position: "The Rev. Jane Doe, Rector".
    if let Some(p) = meta.extra("preacher").filter(|v| !v.trim().is_empty()) {
        return Some(preacher_line(meta.extra("preacher_honorific"), p, meta.extra("preacher_title")));
    }
    if let Some(v) = meta.extra("composer").filter(|v| !v.is_empty()) {
        return Some(v.to_string());
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
    // A sermon's own title.
    if let Some(t) = meta.extra("sermon_title").map(str::trim).filter(|t| !t.is_empty()) {
        return Some(t.to_string());
    }
    (!meta.title.is_empty() && meta.title != cue.title).then(|| meta.title.clone())
}

fn resolve(src: &str, cue: &Cue, root: Option<&Path>) -> Option<PathBuf> {
    let p = crate::portable::resolve(src, root)?;
    let full = if p.is_absolute() { p } else { cue.source()?.parent()?.join(p) };
    full.is_file().then_some(full)
}

/// The congregation's directions for a psalm with an antiphon.
pub const PSALM_ANTIPHON_RUBRIC: &str = "The Cantor introduces the antiphon, which is then repeated by the People.";
pub const PSALM_VERSES_RUBRIC: &str =
    "The People are invited to join the Choir singing the verses printed in boldface.";

/// A psalm with an antiphon, as the bulletin and script print it: how it is
/// sung, the antiphon's music and the setting's (or the antiphon's words,
/// without music), then the verses, each verse before an antiphon marked
/// "Antiphon" at the right margin. The antiphon's words are on the
/// sanctuary's slides; here the music says it.
fn psalm_body(cue: &Cue, doc: &LiturgyDocument, root: Option<&Path>) -> Option<Vec<Piece>> {
    use crate::propers::{is_antiphon_role, ANTIPHON};
    let is_antiphon_head = |b: &Block| matches!(b, Block::Heading { text, .. } if text.eq_ignore_ascii_case(ANTIPHON));
    if !doc.blocks.iter().any(is_antiphon_head) {
        return None;
    }
    let mut out = vec![Piece::Rubric { text: PSALM_ANTIPHON_RUBRIC.into(), src: None }];
    // The antiphon's music (the first in the psalm), else its words once.
    let music = doc.blocks.iter().find_map(|b| match b {
        Block::Image { src, alt, .. } => resolve(src, cue, root).map(|path| Piece::Image { path, alt: alt.clone() }),
        _ => None,
    });
    match music {
        Some(m) => out.push(m),
        None => {
            if let Some((i, lines)) = doc.blocks.iter().enumerate().find_map(|(i, b)| match b {
                Block::Paragraph { role, lines } if is_antiphon_role(role.as_ref()) => Some((i, lines)),
                _ => None,
            }) {
                out.push(Piece::Speech {
                    label: Some("People".into()),
                    lines: lines_at(lines, true, i),
                    joins: Vec::new(),
                    indents: Vec::new(),
                });
            }
        }
    }
    // The setting the verses are sung to.
    if let Some(path) = doc.meta.extra("setting_image").and_then(|s| resolve(s, cue, root)) {
        out.push(Piece::Image { path, alt: "Setting".into() });
    }
    let bold_verse = doc.blocks.iter().any(|b| {
        matches!(b, Block::Paragraph { role: Some(Role::Custom(n)), lines }
            if n.starts_with(|c: char| c.is_ascii_digit()) && lines.iter().flatten().any(|s| s.bold))
    });
    if bold_verse {
        out.push(Piece::Rubric { text: PSALM_VERSES_RUBRIC.into(), src: None });
    }
    for (i, b) in doc.blocks.iter().enumerate() {
        match b {
            Block::Paragraph { role: Some(Role::Custom(n)), lines } if n.starts_with(|c: char| c.is_ascii_digit()) => {
                out.push(Piece::Verse {
                    number: n.clone(),
                    lines: keep_asterisks(lines_at(lines, false, i)),
                    indents: verse_indents(n, lines),
                    tail: None,
                });
            }
            b if is_antiphon_head(b) => {
                // The verse before an antiphon says so.
                if let Some(Piece::Verse { tail, .. }) = out.last_mut() {
                    *tail = Some(ANTIPHON.into());
                }
            }
            _ => {}
        }
    }
    Some(out)
}

/// A hymn's picture (`hymn_image`: its music and words), printed in the
/// bulletin and script in place of its words. The slides show the words.
pub const HYMN_IMAGE: &str = "hymn_image";

fn hymn_picture(cue: &Cue, doc: &LiturgyDocument, root: Option<&Path>) -> Option<PathBuf> {
    (cue.category == ElementCategory::Hymn).then_some(())?;
    resolve(doc.meta.extra(HYMN_IMAGE)?, cue, root)
}

/// The full text of an item as pieces. `script`: rubrics are kept and
/// slide headings become section lines.
fn body(cue: &Cue, script: bool, root: Option<&Path>) -> Vec<Piece> {
    let Some(doc) = cue.document() else { return Vec::new() };
    if let Some(path) = hymn_picture(cue, doc, root) {
        return vec![Piece::Image { path, alt: doc.meta.title.clone() }];
    }
    if cue.category == ElementCategory::Psalm {
        if let Some(p) = psalm_body(cue, doc, root) {
            let _ = script; // the bulletin and the script print a psalm alike
            return p;
        }
    }
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
    // Only a slide break since the last piece (not a heading, rubric …).
    let mut after_break = false;
    // Music carrying its own words: the music or the words, by the item's rule.
    let shown = crate::shows::shown(doc, if script { Place::Script } else { Place::Bulletin });
    for (i, b) in doc.blocks.iter().enumerate() {
        if Some(i) == last_break {
            text_at = out.len();
        }
        if !shown[i] {
            continue;
        }
        let was_break = after_break;
        after_break = false;
        match b {
            Block::SlideBreak | Block::OutputBreak { .. } => after_break = true,
            Block::Heading { .. } => {}
            Block::Image { src, alt, .. } => {
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
                    out.push(Piece::Verse {
                        number: n.clone(),
                        lines: keep_asterisks(lines_at(lines, false, i)),
                        indents: verse_indents(n, lines),
                        tail: None,
                    });
                }
                Some(Role::Custom(a)) if a.eq_ignore_ascii_case("antiphon") || a.eq_ignore_ascii_case("refrain") => {
                    out.push(Piece::Speech {
                        label: None,
                        lines: lines_at(lines, true, i),
                        joins: Vec::new(),
                        indents: indents_of(lines),
                    });
                }
                Some(r) => {
                    let new = lines_at(lines, r.is_congregational(), i);
                    // A paragraph the slides split: the same speaker straight
                    // after a slide break runs on in the bulletin and script.
                    match out.last_mut() {
                        Some(Piece::Speech { label: Some(l), lines: prev, joins, indents })
                            if was_break && l == r.label() =>
                        {
                            let more = indents_of(lines);
                            if !more.is_empty() || !indents.is_empty() {
                                indents.resize(prev.len(), 0);
                                indents.extend(if more.is_empty() { vec![0; new.len()] } else { more });
                            }
                            // Prose runs on; text set out in lines (the
                            // Creed) carries on on a new line.
                            let sense = lines.len() > 1 || lines.first().is_some_and(|l| split_indent(l).0 > 0);
                            if !sense {
                                joins.push(prev.len());
                            }
                            prev.extend(new);
                        }
                        _ => out.push(Piece::Speech {
                            label: Some(r.label().to_string()),
                            lines: new,
                            joins: Vec::new(),
                            indents: indents_of(lines),
                        }),
                    }
                }
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
                        out.push(Piece::Speech {
                            label: None,
                            lines: lines_at(lines, false, i),
                            joins: Vec::new(),
                            indents: indents_of(lines),
                        });
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
        Piece::Speech { label: Some(l), .. } => Role::parse(l).is_congregational(),
        Piece::Speech { label: None, lines, .. } => lines.iter().flatten().all(|r| r.bold || r.text.trim().is_empty()),
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
    let entries = prelude_entry(service, false)
        .into_iter()
        .chain(
            service
                .cues
                .iter()
                .map(|cue| {
                    let cue = &with_preface(cue, service);
                    let mut e = entry(cue, style_of(cue), false, service.library_root.as_deref());
                    presider_labels(&mut e.pieces, service);
                    speaker_labels(&mut e.pieces, parish.speaker_labels);
                    e
                })
                .filter(|e| e.style != BulletinStyle::Hidden),
        )
        .chain(postlude_entry(service, false))
        .collect();
    Doc { kind: DocKind::Bulletin, cover: cover(service, parish), entries, back: Some(parish.clone()) }
}

/// The style an item has in the script unless the plan says otherwise:
/// every word, except what has no words to print (announcements, a video,
/// a hymn not chosen yet or named only by its number).
pub fn script_default(cue: &Cue) -> BulletinStyle {
    let doc = cue.document();
    match &cue.content {
        CueContent::Announcements { .. } => BulletinStyle::Title,
        _ if doc.is_some_and(|d| d.meta.screen_only()) => BulletinStyle::Title,
        _ if cue.category == ElementCategory::Hymn
            && !doc.is_some_and(|d| {
                d.meta.extra(HYMN_IMAGE).is_some_and(|v| !v.trim().is_empty())
                    || d.blocks.iter().any(|b| {
                        matches!(b, Block::Image { .. })
                            || matches!(b, Block::Paragraph { role, .. } if !role.as_ref().is_some_and(Role::is_rubric))
                    })
            }) =>
        {
            BulletinStyle::Title
        }
        _ => BulletinStyle::Full,
    }
}

/// The style an item has in the script: a hymn's can be chosen, as in the
/// bulletin (its words, a picture of it, its line only, or left out).
pub fn script_style_of(cue: &Cue) -> BulletinStyle {
    match cue.overrides.script {
        Some(s) if cue.category == ElementCategory::Hymn => s,
        _ => script_default(cue),
    }
}

/// The officiant's script: every word of every item, rubrics included, the
/// Great Thanksgiving starting a new page.
pub fn script(service: &Service, parish: &ParishInfo) -> Doc {
    let mut entries: Vec<Entry> = prelude_entry(service, true).into_iter().collect();
    for cue in &service.cues {
        let style = script_style_of(cue);
        if style == BulletinStyle::Hidden {
            continue;
        }
        let mut e = entry(&with_preface(cue, service), style, true, service.library_root.as_deref());
        if cue.category == ElementCategory::EucharisticPrayer
            && !entries.iter().any(|x: &Entry| x.pieces.contains(&Piece::PageBreak))
        {
            e.pieces.insert(0, Piece::PageBreak);
        }
        presider_labels(&mut e.pieces, service);
        speaker_labels(&mut e.pieces, parish.speaker_labels);
        entries.push(e);
    }
    entries.extend(postlude_entry(service, true));
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
/// A psalm's half-verse mark stays with the word before it (a no-break
/// space), so a long line never wraps to leave the "*" alone on the next
/// line. (Edits in the preview turn it back into a space: see [`tidy`].)
fn keep_asterisks(mut lines: Vec<Vec<Run>>) -> Vec<Vec<Run>> {
    for line in &mut lines {
        let n = line.len();
        for k in 0..n {
            let text = &mut line[k].text;
            if text.ends_with(" *") {
                let at = text.len() - 2;
                text.replace_range(at..at + 1, "\u{a0}");
            } else if text.trim() == "*" && k > 0 && line[k - 1].text.ends_with(' ') {
                let prev = &mut line[k - 1].text;
                prev.pop();
                prev.push('\u{a0}');
                // (The "*" run itself starts with no space now.)
                let t = line[k].text.trim_start().to_string();
                line[k].text = t;
            }
        }
    }
    lines
}

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
                // The preview shows indents, not tabs: keep the line's.
                LineEdit::Set { spans } => {
                    let indent = split_indent(&lines[j]).0;
                    lines[j] = with_indent(&tidy(spans), indent);
                }
                LineEdit::Split { spans, rest } => {
                    let indent = split_indent(&lines[j]).0;
                    lines[j] = with_indent(&tidy(spans), indent);
                    lines.insert(j + 1, with_indent(&tidy(rest), indent));
                }
                LineEdit::JoinPrevious if j > 0 => {
                    let cur = split_indent(&lines.remove(j)).1;
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

/// Lines each on its own (indented, hanging) line; `joins` run on.
fn html_lines(lines: &[Vec<Run>], joins: &[usize], indents: &[usize]) -> String {
    let mut body = String::new();
    for (k, l) in lines.iter().enumerate() {
        if k > 0 && joins.contains(&k) {
            body.push(' ');
            body.push_str(&html_runs(l));
            continue;
        }
        if k > 0 {
            body.push_str("</span>");
        }
        let n = indents.get(k).copied().unwrap_or(0);
        body.push_str(&format!("<span class=\"ln\" style=\"--in:{n}\">"));
        body.push_str(&html_runs(l));
    }
    if !lines.is_empty() {
        body.push_str("</span>");
    }
    body
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
    let paper = match (&doc.kind, &doc.back) {
        (DocKind::Bulletin, Some(p)) => format!(" paper-{}", p.paper.key()),
        (DocKind::Bulletin, None) => format!(" paper-{}", BulletinPaper::default().key()),
        _ => String::new(),
    };
    h.push_str(&format!("<article class=\"doc {kind}{paper}\">"));
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
                Piece::Speech { label, lines, joins, indents } => {
                    h.push_str("<div class=\"speech\">");
                    h.push_str(&format!("<span class=\"who\">{}</span>", escape(label.as_deref().unwrap_or(""))));
                    let body = if indents.is_empty() {
                        let mut body = String::new();
                        for (k, l) in lines.iter().enumerate() {
                            if k > 0 {
                                body.push_str(if joins.contains(&k) { " " } else { "<br>" });
                            }
                            body.push_str(&html_runs(l));
                        }
                        body
                    } else {
                        html_lines(lines, joins, indents)
                    };
                    h.push_str(&format!("<span class=\"said\">{body}</span></div>"));
                }
                Piece::Verse { number, lines, indents, tail } => {
                    let mut body = html_lines(lines, &[], indents);
                    // At the end of the last line, at the right margin.
                    if let Some(t) = tail {
                        if let Some(at) = body.rfind("</span>") {
                            body.insert_str(at, &format!("<b class=\"tail\">{}</b>", escape(t)));
                        }
                    }
                    h.push_str(&format!(
                        "<div class=\"verse\"><span class=\"num\">{}</span><span class=\"said\">{body}</span></div>",
                        escape(number)
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
        assert!(
            matches!(&e.pieces[3], Piece::Speech { label: Some(l), lines, .. } if l == "People" && lines[0][0].bold)
        );

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
    fn a_prayer_split_across_slides_reads_as_one() {
        let confession = cue("---\ntitle: Confession of Sin\ncitation: BCP 360\nbulletin: full\n---\n[Deacon] Let us confess our sins.\n[All] **Most merciful God, we confess that we have sinned against you.**\n\n---\n\n[All] **We are truly sorry and we humbly repent.**\n\n---\n\n[Priest] Almighty God have mercy on you.\n[People] **Amen.**\n");
        let mut s = Service::default();
        s.insert_cue(0, confession);
        for doc in [bulletin(&s, &ParishInfo::default()), script(&s, &ParishInfo::default())] {
            let speeches: Vec<(Option<String>, usize, Vec<usize>)> = doc.entries[0]
                .pieces
                .iter()
                .filter_map(|p| match p {
                    Piece::Speech { label, lines, joins, .. } => Some((label.clone(), lines.len(), joins.clone())),
                    _ => None,
                })
                .collect();
            // Deacon, All (both slides, run on), Priest, People.
            assert_eq!(speeches.len(), 4, "{speeches:?}");
            assert_eq!(speeches[1], (Some("All".into()), 2, vec![1]));
            // Each line still knows where it came from, for editing in place.
            let Piece::Speech { lines, .. } =
                &doc.entries[0].pieces.iter().filter(|p| matches!(p, Piece::Speech { .. })).nth(1).unwrap()
            else {
                unreachable!()
            };
            assert_ne!(lines[0][0].src.unwrap().block, lines[1][0].src.unwrap().block);
            let html = to_html(&doc);
            assert!(html.contains("against you.</b></span> <span"), "run on with a space: {html}");
            let docx = crate::docx::to_docx(&doc).unwrap();
            assert!(docx.len() > 1000);
        }
        // On the slides the two parts stay on their own slides.
        assert_eq!(s.cues[0].slide_count(), 3);
    }

    #[test]
    fn the_sermon_line_names_the_preacher() {
        let sermon = cue("---\ntitle: The Sermon\ntype: homily\nsermon_title: The Kingdom Is Near\npreacher_honorific: The Very Rev.\npreacher: Jane Doe\npreacher_title: Rector\n---\n# The Sermon\n");
        assert_eq!(default_style(&sermon), BulletinStyle::Title);
        assert_eq!(
            title_line(&sermon, BulletinStyle::Title),
            Piece::Line {
                title: "The Sermon".into(),
                detail: None,
                incipit: Some("The Kingdom Is Near".into()),
                reference: Some("The Very Rev. Jane Doe, Rector".into()),
            }
        );
        let p: ParishInfo =
            serde_json::from_str(r#"{"name": "St. X", "rector": {"name": "The Rev. Jane Doe", "title": "Rector"}}"#)
                .unwrap();
        assert_eq!(p.rector.title, "Rector");
        assert_eq!(
            preacher_line(Some("The Rt. Rev."), "John Smith", Some("Bishop of the Diocese")),
            "The Rt. Rev. John Smith, Bishop of the Diocese"
        );
        assert_eq!(p.bishop, Clergy::default());
    }

    #[test]
    fn a_bishop_presiding_relabels_the_celebrant() {
        let mut s = Service::default();
        s.insert_cue(0, cue("---\ntitle: Salutation\n---\n[Celebrant] The Lord be with you.\n[People] **And also with you.**\n[Priest] Let us pray.\n[Deacon] Let us confess our sins.\n"));
        s.bishop = true;
        s.rebuild_all();
        let labels = |d: &Doc| {
            d.entries[0]
                .pieces
                .iter()
                .filter_map(|p| match p {
                    Piece::Speech { label, .. } => Some(label.clone().unwrap_or_default()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(labels(&bulletin(&s, &ParishInfo::default())), ["Bishop", "People", "Bishop", "Deacon"]);
        assert_eq!(labels(&script(&s, &ParishInfo::default())), ["Bishop", "People", "Bishop", "Deacon"]);
        // The slides too.
        let roles: Vec<String> =
            s.cues[0].slide(0).unwrap().blocks.iter().filter_map(|b| b.role().map(|r| r.label().to_string())).collect();
        assert_eq!(roles, ["Bishop", "People", "Bishop", "Deacon"]);
        // No deacon: a priest says the Deacon's parts, even with a bishop.
        s.no_deacon = true;
        s.rebuild_all();
        assert_eq!(labels(&bulletin(&s, &ParishInfo::default())), ["Bishop", "People", "Bishop", "Priest"]);
        assert_eq!(labels(&script(&s, &ParishInfo::default())), ["Bishop", "People", "Bishop", "Priest"]);
        let roles: Vec<String> =
            s.cues[0].slide(0).unwrap().blocks.iter().filter_map(|b| b.role().map(|r| r.label().to_string())).collect();
        assert_eq!(roles, ["Bishop", "People", "Bishop", "Priest"]);
        s.bishop = false;
        s.rebuild_all();
        assert_eq!(labels(&script(&s, &ParishInfo::default())), ["Celebrant", "People", "Priest", ""]);
        // the same speaker carries on
    }

    #[test]
    fn psalms_and_sense_lines_are_indented() {
        let mut s = Service::default();
        s.insert_cue(0, cue("---\ntitle: Psalm 123\ntype: psalm\nbulletin: full\n---\n[1] To you I lift up my eyes, *\nto you enthroned in the heavens.\n\n[2] As the eyes of servants *\nlook to the hand of their masters,\n"));
        s.insert_cue(1, cue("---\ntitle: Creed\nbulletin: full\n---\n[All] **We believe in one God,**\n\t**the Father, the Almighty,**\n\n---\n\n[All]\n\t**Through him all things were made.**\n\t\t**he came down from heaven:**\n"));
        s.rebuild_all();
        for doc in [bulletin(&s, &ParishInfo::default()), script(&s, &ParishInfo::default())] {
            let verses: Vec<Vec<usize>> = doc.entries[0]
                .pieces
                .iter()
                .filter_map(|p| match p {
                    Piece::Verse { indents, .. } => Some(indents.clone()),
                    _ => None,
                })
                .collect();
            // Both halves level; every other verse a step in.
            assert_eq!(verses, [vec![0, 0], vec![1, 1]]);
            let Some(Piece::Speech { lines, joins, indents, .. }) =
                doc.entries[1].pieces.iter().find(|p| matches!(p, Piece::Speech { .. }))
            else {
                panic!()
            };
            // One speech (one label), the lines as written, the tabs gone.
            assert_eq!(lines.len(), 4);
            assert_eq!(indents, &[0, 1, 1, 2]);
            assert!(joins.is_empty(), "lines carry on on new lines");
            assert_eq!(lines[1][0].text, "the Father, the Almighty,");
            let html = to_html(&doc);
            assert!(html.contains("<span class=\"ln\" style=\"--in:2\">"), "{html}");
            let docx = crate::docx::to_docx(&doc).unwrap();
            assert!(!docx.is_empty());
            if let Ok(p) = std::env::var("DOCX_OUT") {
                std::fs::write(p, &docx).unwrap();
            }
        }
        // An edit in the preview keeps the line's indent.
        let d = s.cues[1].document().unwrap();
        let edited =
            edit_line(d, 0, Some(1), LineEdit::Set { spans: vec![Span::bold("the Father Almighty,")] }).unwrap();
        let Block::Paragraph { lines, .. } = &edited.blocks[0] else { panic!() };
        assert_eq!(split_indent(&lines[1]).0, 1);
    }

    #[test]
    fn a_speakers_label_prints_once_per_block() {
        let mut s = Service::default();
        s.insert_cue(0, cue("---\ntitle: Salutation\n---\n[Celebrant] The Lord be with you.\n[People] **And also with you.**\n[Celebrant] Let us pray.\n[People] **Amen.**\n"));
        s.insert_cue(1, cue("---\ntitle: The Nicene Creed\n---\n[Rubric] *All say together*\n[All] **We believe in one God,**\n[All] **We believe in one Lord, Jesus Christ,**\n[Rubric] *All may bow.*\n[All] **We believe in the Holy Spirit,**\n"));
        s.insert_cue(2, cue("---\ntitle: Confession\n---\n[Deacon] Let us confess our sins.\n[People] **Most merciful God,**\n[People] **we confess that we have sinned.**\n[Priest] Almighty God have mercy on you.\n[People] **Amen.**\n"));
        let labels = |doc: &Doc, i: usize| {
            doc.entries[i]
                .pieces
                .iter()
                .filter_map(|p| match p {
                    Piece::Speech { label, .. } => Some(label.clone().unwrap_or_default()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        for doc in [bulletin(&s, &ParishInfo::default()), script(&s, &ParishInfo::default())] {
            // Alternating speakers keep every label.
            assert_eq!(labels(&doc, 0), ["Celebrant", "People", "Celebrant", "People"]);
            // The Creed: one "All" for the whole block, rubrics between and all.
            assert_eq!(labels(&doc, 1), ["All", "", ""]);
            assert_eq!(labels(&doc, 2), ["Deacon", "People", "", "Priest", "People"]);
        }
        // "On every paragraph" keeps them all.
        let every = bulletin(&s, &ParishInfo { speaker_labels: SpeakerLabels::Every, ..Default::default() });
        assert_eq!(labels(&every, 1), ["All", "All", "All"]);
        // All is bold like the people's parts, and counts as a response.
        assert!(matches!(&every.entries[1].pieces[2], Piece::Speech { lines, .. } if lines[0][0].bold));
        let r = entry(&s.cues[1], BulletinStyle::Responses, false, None);
        assert_eq!(r.pieces.len(), 4, "{:?}", r.pieces);
    }

    #[test]
    fn bulletin_and_script_from_a_service() {
        let mut s = Service { date: Some("2025-11-09".into()), time: Some("08:00".into()), ..Default::default() };
        s.insert_cue(0, cue("---\ntitle: Collect for Purity\ncitation: BCP 323\n---\n[Celebrant] Almighty God, unto whom all hearts are open.\n[Rubric] *Then the Gloria.*\n"));
        s.insert_cue(1, cue("---\ntitle: Collect of the Day\ntype: collect\n---\n[Priest] O God, whose blessed Son came into the world.\n[People] **Amen.**\n"));
        s.insert_cue(2, cue("---\ntitle: The Great Thanksgiving\ntype: eucharistic_prayer\ncitation: BCP 333\n---\n[Celebrant] The Lord be with you.\n"));
        s.insert_cue(3, cue("---\ntitle: Clip\nmedia: /x.png\n---\n"));
        let parish = ParishInfo { name: "St. Example".into(), ..Default::default() };
        // A special service names itself on the cover.
        let mut friday =
            Service { date: Some("2027-03-26".into()), liturgy: Some("Good Friday".into()), ..Default::default() };
        assert_eq!(cover(&friday, &parish).title, "Good Friday");
        assert_eq!(cover(&friday, &parish).day, None, "not \"on Good Friday\" again");
        friday.liturgy = Some("The Ordination of a Priest".into());
        friday.date = Some("2027-06-12".into());
        assert!(cover(&friday, &parish).day.is_some_and(|d| d.starts_with("on ") && !d.starts_with("on the the")));

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

    #[test]
    fn music_with_its_words_is_printed_without_the_words_again() {
        let dir = std::env::temp_dir().join(format!("episco-sung-print-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("acclamation.png");
        std::fs::write(&png, b"png").unwrap();
        let text = format!(
            "---\ntitle: Opening Acclamation\ntype: acclamation\n---\n![]({}){{words}}\n\n[Celebrant] Blessed be God.\n[People] **And blessed be his kingdom.**\n\n---\n\n[Rubric] *The Celebrant then says*\n",
            png.display()
        );
        let mut s = Service::default();
        s.insert_cue(0, cue(&text));
        s.cues[0].overrides.bulletin = Some(BulletinStyle::Full);
        let kinds = |d: &Doc| -> Vec<&'static str> {
            d.entries[0]
                .pieces
                .iter()
                .map(|p| match p {
                    Piece::Image { .. } => "music",
                    Piece::Speech { .. } => "words",
                    Piece::Rubric { .. } => "rubric",
                    _ => "line",
                })
                .collect()
        };
        assert_eq!(kinds(&bulletin(&s, &ParishInfo::default())), ["line", "music", "rubric"]);
        assert_eq!(kinds(&script(&s, &ParishInfo::default())), ["line", "music", "rubric"]);
        // The item can ask for the words in the script too.
        if let CueContent::Liturgy { document, .. } = &mut s.cues[0].content {
            document.meta.extra.push(("words_on".into(), "script, sanctuary, stream".into()));
        }
        assert_eq!(kinds(&script(&s, &ParishInfo::default())), ["line", "music", "words", "words", "rubric"]);
    }

    #[test]
    fn a_hymn_in_the_script_is_printed_as_chosen() {
        let dir = std::env::temp_dir().join(format!("episco-hymn-script-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("hymn.png");
        std::fs::write(&png, b"png").unwrap();
        let mut s = Service::default();
        // A hymn that is only its picture: printed in the script.
        s.insert_cue(
            0,
            cue(&format!("---\ntitle: Entrance Hymn\ntype: hymn\nhymn_number: 436\n---\n![]({})\n", png.display())),
        );
        assert_eq!(script_default(&s.cues[0]), BulletinStyle::Full);
        let sc = script(&s, &ParishInfo::default());
        assert!(sc.entries[0].pieces.iter().any(|p| matches!(p, Piece::Image { .. })));
        // A picture of the hymn in place of its words.
        let pic = format!(
            "---\ntitle: Entrance Hymn\ntype: hymn\nhymn_image: {}\n---\n[1] Lift up your heads\n",
            png.display()
        );
        let mut u = Service::default();
        u.insert_cue(0, cue(&pic));
        let kinds: Vec<_> = script(&u, &ParishInfo::default()).entries[0]
            .pieces
            .iter()
            .map(|p| matches!(p, Piece::Image { .. }))
            .collect();
        assert_eq!(kinds, [false, true], "its line, then the picture (not the words)");
        // Chosen: its line only, or left out.
        s.cues[0].overrides.script = Some(BulletinStyle::Reference);
        assert_eq!(script(&s, &ParishInfo::default()).entries[0].pieces.len(), 1);
        s.cues[0].overrides.script = Some(BulletinStyle::Hidden);
        assert!(script(&s, &ParishInfo::default()).entries.is_empty());
        // Not chosen yet: its line.
        let mut t = Service::default();
        t.insert_cue(0, cue("---\ntitle: Entrance Hymn\ntype: hymn\n---\n"));
        assert_eq!(script_default(&t.cues[0]), BulletinStyle::Title);
    }

    #[test]
    fn the_welcome_slide_names_the_service_parish_and_time() {
        let mut s = Service { date: Some("2026-11-08".into()), time: Some("10:30".into()), ..Default::default() };
        s.title = crate::calendar::liturgical_day("2026-11-08".parse().unwrap()).name;
        let parish = ParishInfo { name: "The Cathedral of St. James".into(), ..Default::default() };
        let w = welcome(&s, &parish);
        assert_eq!(
            w.line,
            "The Holy Eucharist on the Twenty-Fourth Sunday after Pentecost, November 8, 2026, 10:30 a.m."
        );
        assert_eq!(w.greeting, "Welcome to the Cathedral of St. James");
        assert_eq!(welcome(&s, &ParishInfo::default()).greeting, "Welcome");
        s.title = "Parish Dinner".into();
        s.time = Some("18:00".into());
        assert_eq!(welcome(&s, &parish).line, "Parish Dinner, November 8, 2026, 6:00 p.m.");
    }

    #[test]
    fn the_prelude_heads_the_bulletin() {
        let s = Service {
            prelude: Some(crate::music::Music::new("Adagio", "Johann Sebastian Bach, arr. by Homer Whitfield")),
            ..Default::default()
        };
        let b = bulletin(&s, &ParishInfo::default());
        assert_eq!(
            b.entries[0].pieces[0],
            Piece::Line {
                title: "Prelude".into(),
                detail: None,
                incipit: Some("Adagio".into()),
                reference: Some("Johann Sebastian Bach, arr. by Homer Whitfield".into()),
            }
        );
        assert!(matches!(script(&s, &ParishInfo::default()).entries[0].pieces[0], Piece::Line { .. }));
    }

    #[test]
    fn the_prelude_and_postlude_can_be_shortened_or_left_out() {
        use crate::music::Music;
        let titles = |s: &Service, script_too: bool| -> Vec<String> {
            let d = if script_too { script(s, &ParishInfo::default()) } else { bulletin(s, &ParishInfo::default()) };
            d.entries
                .iter()
                .filter(|e| e.cue.is_none())
                .map(|e| match &e.pieces[0] {
                    Piece::Line { title, incipit, .. } => {
                        format!("{title}{}", incipit.as_deref().map(|i| format!(": {i}")).unwrap_or_default())
                    }
                    _ => String::new(),
                })
                .collect()
        };
        let mut s = Service {
            prelude: Some(Music::new("Adagio", "J. S. Bach")),
            postlude: Some(Music::new("Toccata", "Charles-Marie Widor")),
            ..Default::default()
        };
        assert_eq!(titles(&s, false), ["Prelude: Adagio", "Postlude: Toccata"]);
        s.prelude = s.prelude.map(|m| m.with_bulletin(BulletinStyle::Title));
        s.postlude = s.postlude.map(|m| m.with_bulletin(BulletinStyle::Hidden));
        assert_eq!(titles(&s, false), ["Prelude"]);
        // The script names both whatever the bulletin shows.
        assert_eq!(titles(&s, true), ["Prelude: Adagio", "Postlude: Toccata"]);
        // "Prelude" alone needs no piece named yet; left out, nothing is kept but that.
        s.prelude = Some(Music::default().with_bulletin(BulletinStyle::Title));
        s.postlude = Some(Music::default().with_bulletin(BulletinStyle::Hidden));
        assert_eq!(titles(&s, false), ["Prelude"]);
        assert!(!s.postlude.as_ref().unwrap().is_unset());
    }

    #[test]
    fn a_half_verse_mark_stays_with_its_word() {
        let run = |t: &str, bold: bool| Run { text: t.into(), bold, italic: false, src: None };
        let lines = keep_asterisks(vec![
            vec![run("Hear my teaching, O my people; *", false)],
            vec![run("I will open my mouth in a parable; ", true), run("*", false)],
            vec![run("incline your ears", false)],
        ]);
        assert_eq!(lines[0][0].text, "Hear my teaching, O my people;\u{a0}*");
        assert_eq!(
            (lines[1][0].text.as_str(), lines[1][1].text.as_str()),
            ("I will open my mouth in a parable;\u{a0}", "*")
        );
        assert_eq!(lines[2][0].text, "incline your ears");
    }

    #[test]
    fn a_psalm_with_an_antiphon_prints_its_music_and_marks_the_antiphons() {
        use crate::propers::{parse_psalm_text, psalm_document, PsalmOptions};
        let dir = std::env::temp_dir().join(format!("episco-psalm-print-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (a, st) = (dir.join("antiphon.png"), dir.join("setting.png"));
        std::fs::write(&a, b"png").unwrap();
        std::fs::write(&st, b"png").unwrap();
        let verses = parse_psalm_text("1 Hear my teaching, O my people; *\n incline your ears.\n2 I will open my mouth in a parable; *\n I will declare the mysteries.\n3 That which we have heard and known, *\n we will not hide.\n4 We will recount to generations to come, *\n and the wonderful works he has done.");
        let opts = PsalmOptions {
            antiphon: Some("We will recount".into()),
            antiphon_every: 2,
            placed: false,
            after_verses: None,
            verses_per_slide: 2,
        };
        let img = vec![Block::Image {
            src: a.to_string_lossy().into_owned(),
            alt: "Antiphon".into(),
            fit: None,
            words: false,
            words_lines: None,
        }];
        let mut doc = psalm_document("Psalm 78:1-7", Some("Psalm 78:1-7"), &verses, &opts, &img);
        doc.meta.extra.push(("setting_image".into(), st.to_string_lossy().into_owned()));
        let mut s = Service::default();
        s.insert_cue(0, Cue::liturgy("p", doc, None, Default::default()));
        s.rebuild_all();
        for d in [bulletin(&s, &ParishInfo::default()), script(&s, &ParishInfo::default())] {
            let p = &d.entries[0].pieces;
            let kinds: Vec<String> = p
                .iter()
                .map(|x| match x {
                    Piece::Line { .. } => "title".into(),
                    Piece::Rubric { text, .. } => format!("rubric:{}", &text[..10]),
                    Piece::Image { path, .. } => format!("image:{}", path.file_name().unwrap().to_string_lossy()),
                    Piece::Verse { number, tail, .. } => {
                        format!("{number}{}", tail.as_deref().map(|t| format!(" {t}")).unwrap_or_default())
                    }
                    other => format!("{other:?}"),
                })
                .collect();
            assert_eq!(
                kinds,
                [
                    "title",
                    "rubric:The Cantor",
                    "image:antiphon.png",
                    "image:setting.png",
                    "rubric:The People",
                    "1",
                    "2 Antiphon",
                    "3",
                    "4 Antiphon"
                ],
                "{:?}",
                d.kind
            );
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
