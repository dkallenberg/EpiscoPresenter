//! Readings and the collect for a day, from text the operator pastes in.
//!
//! The Episcopal lectionary site refuses automated requests, so the operator
//! copies the day's page in a browser and pastes it into the app. This module
//! finds what matters for the screens without depending on the page's exact
//! layout:
//!
//! * the day's name ("Twenty-fifth Sunday after Pentecost"),
//! * the collect, once per form when the page gives both ("Rite I:",
//!   "Rite II:"),
//! * the scripture citations from heading lines such as
//!   "Old Testament: Judges 4:1-7" (citations inside reading texts are
//!   ignored), with the track they belong to ("Track 1", "Track 2"),
//! * the psalm's verses (the psalter is from the 1979 BCP), which become
//!   slides with the verses alternating between choir (plain) and
//!   congregation (bold), and an optional antiphon.
//!
//! The operator ticks which items to add; [`build_document`] turns each into
//! an `.ewm` document in the house style (title slide + responses).
//!
//! Reading *texts* are deliberately not imported: the screens show only
//! citations and responses, and Bible translations are copyrighted.

use serde::{Deserialize, Serialize};

use crate::category::ElementCategory;
use crate::document::{Block, FrontMatter, Line, LiturgyDocument, Span};
use crate::role::Role;

/// What kind of item a found entry becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProperKind {
    Collect,
    FirstReading,
    Psalm,
    SecondReading,
    Gospel,
    Reading,
}

impl ProperKind {
    pub fn label(self) -> &'static str {
        match self {
            ProperKind::Collect => "Collect of the Day",
            ProperKind::FirstReading => "First Reading",
            ProperKind::Psalm => "Psalm",
            ProperKind::SecondReading => "Second Reading",
            ProperKind::Gospel => "The Holy Gospel",
            ProperKind::Reading => "Reading",
        }
    }
}

/// One importable item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProperItem {
    pub kind: ProperKind,
    /// Scripture citation (readings) — empty for the collect.
    #[serde(default)]
    pub citation: String,
    /// Collect text, without the closing "Amen."
    #[serde(default)]
    pub text: String,
    /// Nearest variant heading, e.g. "Track 1" or "Track 2", if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// True if the page offered this as an alternative ("or …").
    #[serde(default)]
    pub alternative: bool,
    /// Psalm verses found on the page.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verses: Vec<PsalmVerse>,
    /// Psalm antiphon to add (chosen in the import dialog).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub antiphon: Option<String>,
    /// Repeat the antiphon after every N verses (0: only at start and end).
    #[serde(default)]
    pub antiphon_every: u32,
}

/// One psalm verse: its number and its lines (half-verses).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PsalmVerse {
    pub number: String,
    pub lines: Vec<String>,
    /// Sung by the people (bold) or the choir (plain). `None`: alternate,
    /// starting with the choir.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub people: Option<bool>,
    /// An antiphon follows this verse (kept by "Where placed").
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub antiphon_after: bool,
}

impl PsalmVerse {
    pub fn new(number: impl Into<String>, lines: Vec<String>) -> PsalmVerse {
        PsalmVerse { number: number.into(), lines, ..Default::default() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Propers {
    /// The day, e.g. "The Twenty-Fourth Sunday after Pentecost".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<String>,
    pub items: Vec<ProperItem>,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Books of the Bible and Apocrypha, longest names first within a prefix so
/// that "Song of Solomon" wins over "Song".
const BOOKS: &[&str] = &[
    "Genesis",
    "Exodus",
    "Leviticus",
    "Numbers",
    "Deuteronomy",
    "Joshua",
    "Judges",
    "Ruth",
    "1 Samuel",
    "2 Samuel",
    "1 Kings",
    "2 Kings",
    "1 Chronicles",
    "2 Chronicles",
    "Ezra",
    "Nehemiah",
    "Esther",
    "Job",
    "Psalms",
    "Psalm",
    "Proverbs",
    "Ecclesiastes",
    "Song of Solomon",
    "Song of Songs",
    "Isaiah",
    "Jeremiah",
    "Lamentations",
    "Ezekiel",
    "Daniel",
    "Hosea",
    "Joel",
    "Amos",
    "Obadiah",
    "Jonah",
    "Micah",
    "Nahum",
    "Habakkuk",
    "Zephaniah",
    "Haggai",
    "Zechariah",
    "Malachi",
    "Tobit",
    "Judith",
    "Wisdom of Solomon",
    "Wisdom",
    "Sirach",
    "Ecclesiasticus",
    "Baruch",
    "1 Maccabees",
    "2 Maccabees",
    "Matthew",
    "Mark",
    "Luke",
    "John",
    "Acts",
    "Romans",
    "1 Corinthians",
    "2 Corinthians",
    "Galatians",
    "Ephesians",
    "Philippians",
    "Colossians",
    "1 Thessalonians",
    "2 Thessalonians",
    "1 Timothy",
    "2 Timothy",
    "Titus",
    "Philemon",
    "Hebrews",
    "James",
    "1 Peter",
    "2 Peter",
    "1 John",
    "2 John",
    "3 John",
    "Jude",
    "Revelation",
    "Canticle",
];

fn normalize(s: &str) -> String {
    s.replace(['\u{2013}', '\u{2014}', '\u{2012}'], "-")
        .replace('\u{a0}', " ")
        .replace(['\u{2018}', '\u{2019}'], "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Finds a citation starting at `s` (already normalised). Returns its length.
fn citation_at(s: &str) -> Option<usize> {
    // Book names may also be written "I Kings", "First Kings" — map those.
    let book = BOOKS.iter().filter(|b| s.starts_with(**b)).max_by_key(|b| b.len())?;
    let rest = &s[book.len()..];
    let rest_trim = rest.strip_prefix(' ')?;
    let offset = book.len() + 1;
    // chapter[:verse[a-z]] then any of  - , ; : digits letters "and" "or"? no "or"
    let bytes = rest_trim.as_bytes();
    if !bytes.first()?.is_ascii_digit() {
        return None;
    }
    let mut end = 0;
    let mut last_good = 0;
    while end < bytes.len() {
        let c = bytes[end] as char;
        if c.is_ascii_digit() {
            end += 1;
            last_good = end;
        } else if matches!(c, ':' | '-' | ',' | ';' | '(' | ')' | '[' | ']') || c == ' ' {
            end += 1;
        } else if c.is_ascii_lowercase() && end > 0 && (bytes[end - 1] as char).is_ascii_digit() {
            // verse part letters like 3a, 25b (but not words)
            let next = bytes.get(end + 1).map(|b| *b as char);
            if next.is_none_or(|n| !n.is_ascii_alphabetic()) {
                end += 1;
                last_good = end;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    (last_good > 0).then_some(offset + last_good)
}

fn clean_citation(c: &str) -> String {
    let mut s = c.trim().trim_end_matches([',', ';', ':', '(', '[']).trim().to_string();
    // Balance a dangling bracket from "(78:1-7)" style annotations.
    for (open, close) in [('(', ')'), ('[', ']')] {
        if s.matches(open).count() < s.matches(close).count() {
            s = s.trim_end_matches(close).to_string();
        }
        if s.matches(open).count() > s.matches(close).count() {
            s.push(close);
        }
    }
    s
}

/// What a heading label names ("Old Testament", "Psalm", "Epistle", …).
fn label_kind(label: &str) -> Option<ProperKind> {
    let l = label.trim().trim_end_matches(':').trim().to_ascii_lowercase();
    if l.is_empty() || l.len() > 40 {
        return None;
    }
    Some(if l.contains("gospel") {
        ProperKind::Gospel
    } else if l == "psalm" || l == "the psalm" || l == "psalms" {
        ProperKind::Psalm
    } else if l.contains("first reading")
        || l.contains("old testament")
        || l.contains("hebrew bible")
        || l.contains("first lesson")
        || l == "reading 1"
    {
        ProperKind::FirstReading
    } else if l.contains("second reading")
        || l.contains("epistle")
        || l.contains("new testament")
        || l.contains("second lesson")
        || l == "reading 2"
    {
        ProperKind::SecondReading
    } else if l == "reading" || l == "lesson" || l == "the reading" {
        ProperKind::Reading
    } else {
        return None;
    })
}

fn is_collect_heading(line: &str) -> bool {
    let l = line.trim().trim_end_matches(':').trim().to_ascii_lowercase();
    l.len() < 40 && (l == "collect" || l.starts_with("the collect") || l.starts_with("collect of the day"))
}

/// "Rite I:" / "Rite II:" (and spelled-out forms) under the collect.
fn rite_heading(line: &str) -> Option<&'static str> {
    match line.trim().trim_end_matches(':').trim().to_ascii_lowercase().as_str() {
        "rite i" | "rite one" | "traditional" => Some("Rite I"),
        "rite ii" | "rite two" | "contemporary" => Some("Rite II"),
        _ => None,
    }
}

/// "Track 1", "Track 2", "Year A" …
fn variant_heading(line: &str) -> Option<String> {
    let t = line.trim().trim_end_matches(':').trim();
    let l = t.to_ascii_lowercase();
    ((l.starts_with("track ") || l.starts_with("year ")) && t.len() <= 12).then(|| t.to_string())
}

/// Removes markdown links (`[text](url)` → `text`) left by copying a page.
fn strip_links(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        match after.find("](").and_then(|m| after[m + 2..].find(')').map(|c| (m, m + 2 + c))) {
            Some((mid, close)) => {
                out.push_str(&rest[..open]);
                out.push_str(&after[..mid]);
                rest = &after[close + 1..];
            }
            None => {
                out.push_str(&rest[..=open]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The day's name from a heading, without breadcrumb numbers or the site's
/// "The Lectionary - " prefix.
fn day_name(line: &str) -> Option<String> {
    let mut t = line.trim();
    if let Some(rest) = t.split_once(". ").filter(|(n, _)| n.chars().all(|c| c.is_ascii_digit())).map(|(_, r)| r) {
        t = rest;
    }
    for prefix in ["The Lectionary - ", "The Lectionary: "] {
        t = t.strip_prefix(prefix).unwrap_or(t);
    }
    let l = t.to_ascii_lowercase();
    let is_day = t.len() < 90
        && (l.contains("sunday") || l.contains(" day") || l.ends_with(" eve") || l.contains("eve of"))
        && !l.contains("collect")
        && !l.contains("lectionary");
    is_day.then(|| t.trim_end_matches(':').to_string())
}

/// If `s` consists only of citations (joined by "or", "and", commas or
/// semicolons), returns them with whether each was offered as an
/// alternative. Prose, which merely mentions a citation, returns `None`.
fn citation_list(s: &str) -> Option<Vec<(String, bool)>> {
    let mut out = Vec::new();
    let mut rest = s.trim();
    let mut alternative = false;
    while !rest.is_empty() {
        let lower = rest.to_ascii_lowercase();
        if let Some(r) = lower.strip_prefix("or ").map(|_| &rest[3..]) {
            alternative = true;
            rest = r.trim_start();
            continue;
        }
        if lower.starts_with("and ") {
            rest = rest[4..].trim_start();
            continue;
        }
        if rest.starts_with([',', ';', '.']) {
            rest = rest[1..].trim_start();
            continue;
        }
        let len = citation_at(rest)?;
        out.push((clean_citation(&rest[..len]), alternative));
        alternative = false;
        rest = rest[len..].trim_start();
    }
    (!out.is_empty()).then_some(out)
}

/// Extracts the day, the collect(s), citations and psalm verses.
pub fn parse(text: &str) -> Propers {
    // (indented?, normalised text) per line, links stripped.
    let lines: Vec<(bool, String)> =
        text.lines().map(|raw| (raw.starts_with([' ', '\t']), normalize(&strip_links(raw)))).collect();
    let mut out = Propers::default();
    let mut current: Option<ProperKind> = None;
    let mut variant: Option<String> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i].1;
        if line.is_empty() {
            i += 1;
            continue;
        }
        if out.day.is_none() {
            if let Some(day) = day_name(line) {
                out.day = Some(day);
                i += 1;
                continue;
            }
        }
        if is_collect_heading(line) {
            i = parse_collects(&lines, i + 1, &mut out);
            continue;
        }
        if let Some(v) = variant_heading(line) {
            variant = Some(v);
            current = None;
            i += 1;
            continue;
        }
        // "Label: citation(s)", a bare "Label", or a line of citations.
        let (kind, citations) = match line.split_once(':') {
            Some((label, rest)) if label_kind(label).is_some() => (label_kind(label), citation_list(rest)),
            _ if label_kind(line).is_some() => {
                current = label_kind(line);
                i += 1;
                continue;
            }
            _ => (None, citation_list(line)),
        };
        let Some(citations) = citations else {
            i += 1; // prose (a reading's text, page furniture …)
            continue;
        };
        if kind.is_some() {
            current = kind;
        }
        let first_new = out.items.len();
        for (citation, alternative) in citations {
            push_citation(&mut out, citation, alternative, current, &variant);
        }
        i += 1;
        // A psalm heading is followed by the psalm's verses.
        if let Some(item) = out.items.get_mut(first_new).filter(|it| it.kind == ProperKind::Psalm) {
            let (verses, next) = parse_verses(&lines, i);
            item.verses = verses;
            i = next;
        }
    }
    out
}

/// Collect paragraphs after a "Collect" heading, one per rite if given.
fn parse_collects(lines: &[(bool, String)], mut j: usize, out: &mut Propers) -> usize {
    let mut rite: Option<&str> = None;
    let mut body: Vec<&str> = Vec::new();
    while j < lines.len() {
        let line = lines[j].1.as_str();
        if line.is_empty() {
            j += 1;
            continue;
        }
        if let Some(r) = rite_heading(line) {
            rite = Some(r);
            body.clear();
            j += 1;
            continue;
        }
        if body.is_empty()
            && (variant_heading(line).is_some() || line.split_once(':').is_some_and(|(l, _)| label_kind(l).is_some()))
        {
            break;
        }
        body.push(line);
        j += 1;
        if line.ends_with("Amen.") || line.ends_with("Amen") {
            let joined = body.join(" ");
            let text = joined.trim_end_matches("Amen.").trim_end_matches("Amen").trim().to_string();
            out.items.push(ProperItem {
                kind: ProperKind::Collect,
                citation: String::new(),
                text,
                variant: rite.map(String::from),
                alternative: false,
                verses: Vec::new(),
                antiphon: None,
                antiphon_every: 0,
            });
            body.clear();
            // Another form may follow ("Rite II:"); otherwise we are done.
            let next = lines[j..].iter().find(|(_, l)| !l.is_empty());
            if !next.is_some_and(|(_, l)| rite_heading(l).is_some()) {
                break;
            }
        }
    }
    j
}

/// True for a line that ends a psalm: the next heading, track, collect or
/// citation line, or the page's "Printable …" / "Download …" furniture.
fn ends_psalm(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    variant_heading(line).is_some()
        || is_collect_heading(line)
        || rite_heading(line).is_some()
        || line.split_once(':').is_some_and(|(label, _)| label_kind(label).is_some())
        || label_kind(line).is_some()
        || citation_list(line).is_some()
        || l.starts_with("printable")
        || l.starts_with("download")
}

/// Splits "12 So teach us…", "12So teach us…" or "12" into number and text.
fn verse_start(line: &str) -> Option<(&str, &str)> {
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || digits > 3 {
        return None;
    }
    let (number, rest) = line.split_at(digits);
    let rest_trim = rest.trim_start();
    // The text must start like psalm text, not like "1:3" or "1,000".
    let ok = rest.is_empty()
        || rest.starts_with(' ')
        || rest_trim
            .starts_with(|c: char| c.is_alphabetic() || matches!(c, '[' | '"' | '\u{201c}' | '\u{2018}' | '\''));
    ok.then_some((number, rest_trim))
}

/// Psalm verses after a psalm heading. A line starting with a number starts
/// a verse ("1 To you…", also "1To you…" or a number on its own line, as
/// browsers copy superscript verse numbers); other lines continue the verse,
/// whether or not they are indented. Blank lines are skipped. Square
/// brackets marking optional verses are removed.
fn parse_verses(lines: &[(bool, String)], mut j: usize) -> (Vec<PsalmVerse>, usize) {
    let clean = |s: &str| s.replace(['[', ']'], "").trim().to_string();
    let mut verses: Vec<PsalmVerse> = Vec::new();
    let mut pending: Option<String> = None; // a number on a line of its own
    while j < lines.len() {
        let line = lines[j].1.as_str();
        if line.is_empty() {
            j += 1;
            continue;
        }
        if ends_psalm(line) {
            break;
        }
        if let Some((number, text)) = verse_start(line) {
            if text.is_empty() {
                pending = Some(number.to_string());
            } else {
                verses.push(PsalmVerse::new(number, vec![clean(text)]));
            }
        } else if let Some(number) = pending.take() {
            verses.push(PsalmVerse::new(number, vec![clean(line)]));
        } else if let Some(last) = verses.last_mut().filter(|_| line.chars().count() <= 160) {
            last.lines.push(clean(line));
        } else {
            break; // prose before any verse, or a long paragraph: not psalm text
        }
        j += 1;
    }
    (verses, j)
}

fn push_citation(
    out: &mut Propers,
    citation: String,
    alternative: bool,
    current: Option<ProperKind>,
    variant: &Option<String>,
) {
    let previous = out.items.iter().rev().find(|it| it.kind != ProperKind::Collect && it.variant == *variant);
    let kind = if citation.starts_with("Psalm") {
        ProperKind::Psalm
    } else if ["Matthew", "Mark", "Luke", "John "].iter().any(|g| citation.starts_with(g)) {
        ProperKind::Gospel
    } else if let Some(prev) = previous.filter(|_| alternative) {
        // "A or B": same slot as the reading before it.
        prev.kind
    } else {
        match current {
            Some(k @ (ProperKind::FirstReading | ProperKind::SecondReading | ProperKind::Reading)) => k,
            // Lectionary order: a reading before the psalm is the first
            // reading, after it the second.
            _ if previous.is_some_and(|p| p.kind == ProperKind::Psalm) => ProperKind::SecondReading,
            _ if previous.is_none() => ProperKind::FirstReading,
            _ => ProperKind::Reading,
        }
    };
    // Tracks only distinguish the first reading and psalm; the epistle and
    // Gospel are shared by both.
    let variant = match kind {
        ProperKind::SecondReading | ProperKind::Gospel => None,
        _ => variant.clone(),
    };
    if !out.items.iter().any(|it| it.citation == citation && it.variant == variant) {
        out.items.push(ProperItem {
            kind,
            citation,
            text: String::new(),
            variant,
            alternative,
            verses: Vec::new(),
            antiphon: None,
            antiphon_every: 0,
        });
    }
}

// ---------------------------------------------------------------------------
// Building documents
// ---------------------------------------------------------------------------

fn bold(text: &str) -> Vec<Span> {
    vec![Span::bold(text)]
}

fn evangelist(citation: &str) -> Option<&'static str> {
    ["Matthew", "Mark", "Luke", "John"].into_iter().find(|g| citation.starts_with(g))
}

/// An `.ewm` document for an imported item, in the parish style:
/// readings get a title slide and the responses; the Gospel gets both
/// acclamations; the collect gets the text and the people's Amen.
pub fn build_document(item: &ProperItem, day: Option<&str>) -> LiturgyDocument {
    build_document_in(item, day, crate::bcp::Rite::Two)
}

/// [`build_document`] for a service in `rite`: Rite I has its own Gospel
/// responses ("Glory be to thee, O Lord") and traditional-language canticles.
pub fn build_document_in(item: &ProperItem, day: Option<&str>, rite: crate::bcp::Rite) -> LiturgyDocument {
    use crate::bcp::{self, Rite};
    if rite == Rite::One && item.kind == ProperKind::Psalm {
        if let Some(t) = bcp::canticle_number(&item.citation).and_then(bcp::traditional_canticle) {
            if let (Some(c), Some(verses)) = (bcp::canticle(t), bcp::canticle_verses(t)) {
                let mut traditional = item.clone();
                traditional.citation = format!("Canticle {t}: {}", c.title);
                traditional.verses = verses;
                return build_document_in(&traditional, day, Rite::Two);
            }
        }
    }
    let (glory, praise) = match rite {
        Rite::One => ("Glory be to thee, O Lord.", "Praise be to thee, O Christ."),
        Rite::Two => ("Glory to you, Lord Christ.", "Praise to you, Lord Christ."),
    };
    let mut meta = FrontMatter::default();
    if rite == Rite::One {
        meta.extra.push(("rite".into(), "I".into()));
    }
    let mut blocks = Vec::new();
    match item.kind {
        ProperKind::Collect => {
            meta.title = "Collect of the Day".into();
            meta.category = Some(ElementCategory::Collect);
            if let Some(day) = day {
                blocks.push(Block::Heading { level: 2, text: day.to_string() });
            }
            blocks.push(Block::speech(Role::Celebrant, [item.text.clone()]));
            blocks.push(Block::Paragraph { role: Some(Role::People), lines: vec![bold("Amen.")] });
        }
        ProperKind::Psalm if !item.verses.is_empty() => {
            let opts = PsalmOptions {
                antiphon: item.antiphon.clone().filter(|a| !a.trim().is_empty()),
                antiphon_every: item.antiphon_every,
                verses_per_slide: 2,
                placed: false,
            };
            return psalm_document(&item.citation, Some(&item.citation), &item.verses, &opts, &[]);
        }
        ProperKind::Psalm => {
            meta.title = item.citation.clone();
            meta.citation = Some(item.citation.clone());
            meta.category = Some(ElementCategory::Psalm);
            blocks.push(Block::Heading { level: 1, text: item.citation.clone() });
            blocks.push(Block::SlideBreak);
            blocks.push(Block::Paragraph {
                role: Some(Role::Rubric),
                lines: vec![vec![Span::italic("Psalm verses: paste or type them here, one slide per pair.")]],
            });
        }
        ProperKind::Gospel => {
            let who = evangelist(&item.citation).unwrap_or("");
            meta.title = "The Holy Gospel".into();
            meta.citation = Some(item.citation.clone());
            meta.category = Some(ElementCategory::Gospel);
            // The announcement, then the title slide, then (when added) the
            // text, then the response: see `arrange_gospel`.
            blocks.push(Block::speech(
                Role::Deacon,
                [format!("The Holy Gospel of our Lord Jesus Christ according to {who}.")],
            ));
            blocks.push(Block::Paragraph { role: Some(Role::People), lines: vec![bold(glory)] });
            blocks.push(Block::SlideBreak);
            blocks.push(Block::Heading { level: 1, text: "The Holy Gospel".into() });
            blocks.push(Block::SlideBreak);
            blocks.push(Block::speech(Role::Deacon, ["The Gospel of the Lord."]));
            blocks.push(Block::Paragraph { role: Some(Role::People), lines: vec![bold(praise)] });
        }
        ProperKind::FirstReading | ProperKind::SecondReading | ProperKind::Reading => {
            let label = match item.kind {
                ProperKind::FirstReading => "The First Reading",
                ProperKind::SecondReading => "The Second Reading",
                _ => "The Reading",
            };
            meta.title = item.kind.label().into();
            meta.citation = Some(item.citation.clone());
            meta.category = Some(ElementCategory::Reading);
            blocks.push(Block::Heading { level: 1, text: label.into() });
            blocks.push(Block::SlideBreak);
            blocks.push(Block::speech(Role::Reader, ["The word of the Lord."]));
            blocks.push(Block::Paragraph { role: Some(Role::People), lines: vec![bold("Thanks be to God.")] });
        }
    }
    LiturgyDocument { meta, blocks }
}

// ---------------------------------------------------------------------------
// The Gospel
// ---------------------------------------------------------------------------

/// Heading of the slide with the music sung before the Gospel.
pub const GOSPEL_MUSIC: &str = "Gospel Acclamation";

/// Puts a Gospel's slides in the order they are used: the music sung before
/// it (when there is a picture of it), the announcement and its response
/// ("The Holy Gospel of our Lord…" / "Glory to you, Lord Christ."), the
/// title slide, then the rest (the text, "The Gospel of the Lord."…).
/// `music`: `Some(Some((src, alt)))` sets the music line, `Some(None)`
/// removes it, `None` keeps what is there.
pub fn arrange_gospel(doc: &LiturgyDocument, music: Option<Option<(String, String)>>) -> LiturgyDocument {
    // Slides: the blocks between slide breaks.
    let mut slides: Vec<Vec<Block>> = vec![Vec::new()];
    for b in &doc.blocks {
        match b {
            Block::SlideBreak => slides.push(Vec::new()),
            other => slides.last_mut().expect("at least one slide").push(other.clone()),
        }
    }
    slides.retain(|s| !s.is_empty());
    let is_music = |s: &Vec<Block>| matches!(s.first(), Some(Block::Heading { text, .. }) if text == GOSPEL_MUSIC);
    let is_title = |s: &Vec<Block>| s.len() == 1 && matches!(s[0], Block::Heading { .. });
    let is_announcement = |s: &Vec<Block>| {
        s.iter().any(|b| {
            matches!(b, Block::Paragraph { role: Some(r), .. } if !r.is_congregational() && !r.is_rubric())
                && b.plain_text().starts_with("The Holy Gospel of our Lord")
        })
    };
    let mut music_slide = slides.iter().find(|s| is_music(s)).cloned();
    if let Some(m) = music {
        music_slide = m.map(|(src, alt)| {
            vec![Block::Heading { level: 1, text: GOSPEL_MUSIC.into() }, Block::Image { src, alt, fit: None }]
        });
    }
    let announcement = slides.iter().position(|s| !is_music(s) && is_announcement(s));
    let title = slides.iter().position(|s| !is_music(s) && is_title(s));
    let mut ordered: Vec<Vec<Block>> = Vec::new();
    ordered.extend(music_slide);
    if let Some(a) = announcement {
        ordered.push(slides[a].clone());
    }
    if let Some(t) = title {
        ordered.push(slides[t].clone());
    }
    for (i, s) in slides.iter().enumerate() {
        if Some(i) != announcement && Some(i) != title && !is_music(s) {
            ordered.push(s.clone());
        }
    }
    let mut blocks = Vec::new();
    for (i, s) in ordered.into_iter().enumerate() {
        if i > 0 {
            blocks.push(Block::SlideBreak);
        }
        blocks.extend(s);
    }
    LiturgyDocument { meta: doc.meta.clone(), blocks }
}

// ---------------------------------------------------------------------------
// Psalms
// ---------------------------------------------------------------------------

/// How a psalm is laid out on slides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsalmOptions {
    /// Antiphon text (sung by all), shown before and after the psalm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub antiphon: Option<String>,
    /// Also repeat the antiphon after every N verses (0: start and end only).
    #[serde(default)]
    pub antiphon_every: u32,
    /// Keep the antiphons where they are now (after the verses marked
    /// `antiphon_after`) instead of every N verses.
    #[serde(default)]
    pub placed: bool,
    /// Verses per slide.
    #[serde(default = "two")]
    pub verses_per_slide: u32,
}

fn two() -> u32 {
    2
}

impl Default for PsalmOptions {
    fn default() -> Self {
        PsalmOptions { antiphon: None, antiphon_every: 0, placed: false, verses_per_slide: 2 }
    }
}

/// Section heading and speaker role of an antiphon.
pub const ANTIPHON: &str = "Antiphon";

/// True for the role that marks an antiphon's words.
pub fn is_antiphon_role(role: Option<&Role>) -> bool {
    matches!(role, Some(Role::Custom(r)) if r.eq_ignore_ascii_case(ANTIPHON))
}

/// The blocks of one antiphon: its heading (a new slide), the music line
/// (`images`) and the words, bold, under it.
pub fn antiphon_blocks(text: &str, images: &[Block]) -> Vec<Block> {
    let mut blocks = vec![Block::Heading { level: 1, text: ANTIPHON.into() }];
    blocks.extend(images.iter().cloned());
    let lines: Vec<Line> = text.split(['\n', '/']).map(str::trim).filter(|l| !l.is_empty()).map(bold).collect();
    blocks.push(Block::Paragraph { role: Some(Role::Custom(ANTIPHON.into())), lines });
    blocks
}

/// A psalm's slides. Each verse shows its number but no role label and is
/// sung by the choir (plain) or the people (bold): as the verse says, else
/// alternating, starting with the choir. The antiphon, if any, is shown at
/// the start, at the end, and after every `antiphon_every` verses (or where
/// the verses say, with `placed`). `images` (the antiphon's music line) go
/// on the antiphon slides, or before the verses when there is no antiphon.
pub fn psalm_document(
    title: &str,
    citation: Option<&str>,
    verses: &[PsalmVerse],
    opts: &PsalmOptions,
    images: &[Block],
) -> LiturgyDocument {
    let mut meta = FrontMatter {
        title: title.to_string(),
        category: Some(ElementCategory::Psalm),
        citation: citation.map(String::from),
        ..Default::default()
    };
    let antiphon = opts.antiphon.as_deref().map(str::trim).filter(|a| !a.is_empty());
    if let Some(a) = antiphon {
        meta.extra.push(("antiphon".into(), a.replace('\n', " / ")));
        if opts.placed {
            meta.extra.push(("antiphon_placed".into(), "true".into()));
        } else if opts.antiphon_every > 0 {
            meta.extra.push(("antiphon_every".into(), opts.antiphon_every.to_string()));
        }
    }
    if opts.verses_per_slide != 2 {
        meta.extra.push(("verses_per_slide".into(), opts.verses_per_slide.to_string()));
    }

    let section = citation.unwrap_or(title).to_string();
    let push_antiphon = |blocks: &mut Vec<Block>| {
        if let Some(a) = antiphon {
            blocks.extend(antiphon_blocks(a, images));
        }
    };

    let mut blocks = Vec::new();
    push_antiphon(&mut blocks);
    blocks.push(Block::Heading { level: 1, text: section.clone() });
    if antiphon.is_none() {
        blocks.extend(images.iter().cloned());
    }
    let per = opts.verses_per_slide.max(1) as usize;
    let every = opts.antiphon_every as usize;
    let mut on_slide = 0;
    for (i, verse) in verses.iter().enumerate() {
        let antiphon_here = i > 0
            && antiphon.is_some()
            && if opts.placed { verses[i - 1].antiphon_after } else { every > 0 && i % every == 0 };
        if antiphon_here {
            push_antiphon(&mut blocks);
            blocks.push(Block::Heading { level: 1, text: section.clone() });
            on_slide = 0;
        } else if on_slide == per {
            blocks.push(Block::SlideBreak);
            on_slide = 0;
        }
        let people = verse.people.unwrap_or(i % 2 == 1);
        let lines = verse.lines.iter().map(|l| if people { bold(l) } else { vec![Span::plain(l.clone())] }).collect();
        blocks.push(Block::Paragraph { role: Some(Role::Custom(verse.number.clone())), lines });
        on_slide += 1;
    }
    if !verses.is_empty() {
        push_antiphon(&mut blocks);
    }
    LiturgyDocument { meta, blocks }
}

fn is_verse_number(role: &Role) -> bool {
    matches!(role, Role::Custom(n) if n.chars().next().is_some_and(|c| c.is_ascii_digit()))
}

/// Psalm verses from pasted text (e.g. just the psalm, copied from the
/// lectionary page). A leading "Psalm 123" line is skipped.
pub fn parse_psalm_text(text: &str) -> Vec<PsalmVerse> {
    let lines: Vec<(bool, String)> =
        text.lines().map(|raw| (raw.starts_with([' ', '\t']), normalize(&strip_links(raw)))).collect();
    let start = lines.iter().position(|(_, l)| !l.is_empty() && verse_start(l).is_some()).unwrap_or(lines.len());
    parse_verses(&lines, start).0
}

/// The verses of a psalm document (numbered paragraphs), in order, with
/// who sings each (bold: the people) and where antiphons are.
pub fn psalm_verses(doc: &LiturgyDocument) -> Vec<PsalmVerse> {
    let mut verses: Vec<PsalmVerse> = Vec::new();
    for b in &doc.blocks {
        match b {
            Block::Paragraph { role: Some(role), lines } if is_verse_number(role) => {
                // The half-verse mark "*" does not count either way.
                let spans: Vec<&Span> = lines.iter().flatten().filter(|s| !matches!(s.text.trim(), "" | "*")).collect();
                let people = !spans.is_empty() && spans.iter().all(|s| s.bold);
                verses.push(PsalmVerse {
                    number: role.label().to_string(),
                    lines: lines.iter().map(|l| crate::document::line_plain_text(l)).collect(),
                    people: Some(people),
                    antiphon_after: false,
                });
            }
            Block::Heading { text, .. } if text.eq_ignore_ascii_case(ANTIPHON) => {
                if let Some(last) = verses.last_mut() {
                    last.antiphon_after = true;
                }
            }
            _ => {}
        }
    }
    verses
}

/// The words of the psalm's first antiphon, lines joined by " / ".
pub fn first_antiphon(doc: &LiturgyDocument) -> Option<String> {
    let mut in_antiphon = false;
    for b in &doc.blocks {
        match b {
            Block::Heading { text, .. } => in_antiphon = text.eq_ignore_ascii_case(ANTIPHON),
            Block::Paragraph { role, lines } if is_antiphon_role(role.as_ref()) || (in_antiphon && role.is_none()) => {
                let text = lines.iter().map(|l| crate::document::line_plain_text(l)).collect::<Vec<_>>().join(" / ");
                if !text.trim().is_empty() {
                    return Some(text);
                }
            }
            _ => {}
        }
    }
    None
}

/// The music images of a psalm (each once, in order).
pub fn psalm_images(doc: &LiturgyDocument) -> Vec<Block> {
    let mut seen = Vec::new();
    let mut out = Vec::new();
    for b in &doc.blocks {
        if let Block::Image { src, .. } = b {
            if !src.is_empty() && !seen.contains(src) {
                seen.push(src.clone());
                out.push(b.clone());
            }
        }
    }
    out
}

/// The layout a psalm document was built with (from its front matter and
/// its antiphons).
pub fn psalm_options(doc: &LiturgyDocument) -> PsalmOptions {
    let num = |k: &str| doc.meta.extra(k).and_then(|v| v.parse().ok());
    PsalmOptions {
        antiphon: first_antiphon(doc).or_else(|| doc.meta.extra("antiphon").map(String::from)),
        antiphon_every: num("antiphon_every").unwrap_or(0),
        placed: doc.meta.extra("antiphon_placed") == Some("true"),
        verses_per_slide: num("verses_per_slide").unwrap_or(2),
    }
}

/// Rebuilds a psalm document's slides from `verses` with new options
/// (antiphon, where it repeats, verses per slide), keeping its title,
/// citation, front matter and music images (which go on the antiphon slides).
pub fn rearrange_psalm_with(doc: &LiturgyDocument, opts: &PsalmOptions, verses: &[PsalmVerse]) -> LiturgyDocument {
    let images = psalm_images(doc);
    let mut out = psalm_document(&doc.meta.title, doc.meta.citation.as_deref(), verses, opts, &images);
    let mut meta = doc.meta.clone();
    meta.extra
        .retain(|(k, _)| !matches!(k.as_str(), "antiphon" | "antiphon_every" | "antiphon_placed" | "verses_per_slide"));
    meta.extra.append(&mut out.meta.extra);
    meta.category = Some(ElementCategory::Psalm);
    out.meta = meta;
    out
}

/// [`rearrange_psalm_with`] the document's own verses.
pub fn rearrange_psalm(doc: &LiturgyDocument, opts: &PsalmOptions) -> LiturgyDocument {
    rearrange_psalm_with(doc, opts, &psalm_verses(doc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gospel_in_the_order_it_is_used() {
        let item = ProperItem {
            kind: ProperKind::Gospel,
            citation: "Matthew 25:1-13".into(),
            text: String::new(),
            variant: None,
            alternative: false,
            verses: Vec::new(),
            antiphon: None,
            antiphon_every: 0,
        };
        let doc = build_document_in(&item, None, crate::bcp::Rite::Two);
        let titles = |d: &LiturgyDocument| -> Vec<String> {
            d.slides()
                .iter()
                .map(|s| match s.blocks.first() {
                    None => format!("[{}]", s.heading.clone().unwrap_or_default()),
                    Some(Block::Image { .. }) => "music".into(),
                    Some(b) => b.plain_text().lines().next().unwrap_or_default().to_string(),
                })
                .collect()
        };
        assert_eq!(
            titles(&doc),
            [
                "The Holy Gospel of our Lord Jesus Christ according to Matthew.",
                "[The Holy Gospel]",
                "The Gospel of the Lord."
            ]
        );
        // The text goes after the title slide, before the closing response.
        let with_text = crate::bible::set_reading_passage(
            &doc,
            "Then the kingdom of heaven will be like this.",
            crate::bible::BibleVersion::Nrsvue,
            40,
        );
        assert_eq!(titles(&with_text)[2], "Then the kingdom of heaven will be like this.");
        // Music first; set again replaces it; None removes it.
        let m = arrange_gospel(&with_text, Some(Some(("media/alleluia.png".into(), "Alleluia".into()))));
        assert_eq!(titles(&m)[0], "music");
        assert_eq!(titles(&m)[1], "The Holy Gospel of our Lord Jesus Christ according to Matthew.");
        assert_eq!(titles(&m).len(), 5);
        let again = arrange_gospel(&m, Some(Some(("media/other.png".into(), "".into()))));
        assert_eq!(titles(&again).len(), 5);
        assert_eq!(titles(&arrange_gospel(&again, Some(None))), titles(&with_text));
        // An older Gospel (title slide first) is put in order.
        let old: LiturgyDocument = "---\ntitle: The Holy Gospel\ntype: gospel\n---\n# The Holy Gospel\n\n---\n\n[Deacon] The Holy Gospel of our Lord Jesus Christ according to Luke.\n[People] **Glory to you, Lord Christ.**\n\n---\n\n[Deacon] The Gospel of the Lord.\n[People] **Praise to you, Lord Christ.**\n".parse().unwrap();
        assert_eq!(titles(&arrange_gospel(&old, None))[1], "[The Holy Gospel]");
        // It still saves and loads cleanly.
        let text = crate::ewm::to_string(&m);
        assert!(crate::ewm::parse(&text).unwrap().warnings.is_empty());
    }

    const PAGE: &str = "
The Lectionary Page
Twenty-Fourth Sunday after Pentecost
Proper 27, Year A

The Collect
O God, whose blessed Son came into the world that he might destroy the works of the devil
and make us children of God and heirs of eternal life: Grant that, having this hope, we may
purify ourselves as he is pure; that, when he comes again with power and great glory, we may
be made like him in his eternal and glorious kingdom; where he lives and reigns with you and
the Holy Spirit, one God, for ever and ever. Amen.

Track 1
Old Testament
Joshua 24:1–3a, 14–25
Psalm 78:1–7
Track 2
Wisdom of Solomon 6:12–16 or Amos 5:18–24
Psalm 70
Epistle
1 Thessalonians 4:13–18
Gospel
Matthew 25:1–13
";

    #[test]
    fn finds_day_collect_and_citations() {
        let p = parse(PAGE);
        assert_eq!(p.day.as_deref(), Some("Twenty-Fourth Sunday after Pentecost"));
        let collect = &p.items[0];
        assert_eq!(collect.kind, ProperKind::Collect);
        assert!(collect.text.starts_with("O God, whose blessed Son"));
        assert!(collect.text.ends_with("for ever and ever."), "{}", collect.text);

        let found: Vec<(ProperKind, &str, Option<&str>, bool)> =
            p.items[1..].iter().map(|i| (i.kind, i.citation.as_str(), i.variant.as_deref(), i.alternative)).collect();
        assert_eq!(
            found,
            vec![
                (ProperKind::FirstReading, "Joshua 24:1-3a, 14-25", Some("Track 1"), false),
                (ProperKind::Psalm, "Psalm 78:1-7", Some("Track 1"), false),
                (ProperKind::FirstReading, "Wisdom of Solomon 6:12-16", Some("Track 2"), false),
                (ProperKind::FirstReading, "Amos 5:18-24", Some("Track 2"), true),
                (ProperKind::Psalm, "Psalm 70", Some("Track 2"), false),
                (ProperKind::SecondReading, "1 Thessalonians 4:13-18", None, false),
                (ProperKind::Gospel, "Matthew 25:1-13", None, false),
            ]
        );
    }

    #[test]
    fn citations_only_from_headings_never_from_prose() {
        let p =
            parse("Gospel: John 11:32-44\nFirst Reading: Isaiah 25:6-9\nAs we read in 1 John 3:1-3, love one another.");
        let c: Vec<&str> = p.items.iter().map(|i| i.citation.as_str()).collect();
        assert_eq!(c, vec!["John 11:32-44", "Isaiah 25:6-9"]);
        assert_eq!(p.items[0].kind, ProperKind::Gospel);
        assert_eq!(p.items[1].kind, ProperKind::FirstReading);
        assert_eq!(citation_list("Psalm 23 and the readings"), None);
        assert_eq!(citation_list("Psalm 90:1-8, (9-11), 12").unwrap()[0].0, "Psalm 90:1-8, (9-11), 12");
        assert_eq!(strip_links("[Track 1](https://x.org/#)"), "Track 1");
    }

    /// The lectionary page as copied from a browser (reading texts replaced
    /// by placeholders; the psalms and collects are the 1979 BCP's).
    const SITE: &str = include_str!("../tests/fixtures/lectionary-proper-28a.txt");

    #[test]
    fn parses_the_lectionary_site() {
        let p = parse(SITE);
        assert_eq!(p.day.as_deref(), Some("Twenty-fifth Sunday after Pentecost"));
        let summary: Vec<(ProperKind, &str, Option<&str>, usize)> = p
            .items
            .iter()
            .map(|i| {
                (
                    i.kind,
                    if i.citation.is_empty() { &i.text[..14] } else { i.citation.as_str() },
                    i.variant.as_deref(),
                    i.verses.len(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                (ProperKind::Collect, "Blessed Lord, ", Some("Rite I"), 0),
                (ProperKind::Collect, "Blessed Lord, ", Some("Rite II"), 0),
                (ProperKind::FirstReading, "Judges 4:1-7", Some("Track 1"), 0),
                (ProperKind::Psalm, "Psalm 123", Some("Track 1"), 5),
                (ProperKind::FirstReading, "Zephaniah 1:7, 12-18", Some("Track 2"), 0),
                (ProperKind::Psalm, "Psalm 90:1-8, (9-11), 12", Some("Track 2"), 12),
                (ProperKind::SecondReading, "1 Thessalonians 5:1-11", None, 0),
                (ProperKind::Gospel, "Matthew 25:14-30", None, 0),
            ]
        );
        assert!(p.items[1].text.starts_with("Blessed Lord, who caused all holy Scriptures"));
        assert!(p.items[1].text.ends_with("for ever and ever."));
        assert!(p.items[0].text.contains("who hast caused"));

        let ps123 = &p.items[3].verses;
        assert_eq!(
            ps123[0],
            PsalmVerse::new("1", vec!["To you I lift up my eyes, *".into(), "to you enthroned in the heavens.".into()])
        );
        let ps90 = &p.items[5].verses;
        assert_eq!(ps90[1].lines.len(), 3, "multi-line verse");
        assert_eq!(ps90[8].lines[0], "When you are angry, all our days are gone; *", "optional-verse brackets removed");
        assert_eq!(ps90[9].lines.len(), 4);
        assert_eq!(ps90.last().unwrap().number, "12");
    }

    #[test]
    fn psalm_verses_survive_how_browsers_copy_them() {
        let expected = vec![
            PsalmVerse::new("1", vec!["To you I lift up my eyes, *".into(), "to you enthroned in the heavens.".into()]),
            PsalmVerse::new(
                "2",
                vec![
                    "As the eyes of servants look to the hand of their masters, *".into(),
                    "and the eyes of a maid to the hand of her mistress,".into(),
                ],
            ),
        ];
        let pages = [
            // Blank lines everywhere (Safari / Chrome copying block elements).
            "Psalm: Psalm 123\n\n1 To you I lift up my eyes, *\n\n       to you enthroned in the heavens.\n\n2 As the eyes of servants look to the hand of their masters, *\n\n       and the eyes of a maid to the hand of her mistress,\n\nEpistle: 1 Thessalonians 5:1-11",
            // Non-breaking-space indentation.
            "Psalm: Psalm 123\n1 To you I lift up my eyes, *\n\u{a0}\u{a0}\u{a0}to you enthroned in the heavens.\n2 As the eyes of servants look to the hand of their masters, *\n\u{a0}\u{a0}\u{a0}and the eyes of a maid to the hand of her mistress,\nGospel: Matthew 25:14-30",
            // No indentation; numbers glued to the text; tabs.
            "Psalm: Psalm 123\n1To you I lift up my eyes, *\nto you enthroned in the heavens.\n2\tAs the eyes of servants look to the hand of their masters, *\n\tand the eyes of a maid to the hand of her mistress,\n[Track 2](https://lectionary.episcopalchurch.org/#)",
            // Verse numbers on their own lines.
            "Psalm: Psalm 123\n1\nTo you I lift up my eyes, *\nto you enthroned in the heavens.\n2\nAs the eyes of servants look to the hand of their masters, *\nand the eyes of a maid to the hand of her mistress,\nPrintable Lectionary Readings",
        ];
        for page in pages {
            let p = parse(page);
            let psalm = p.items.iter().find(|i| i.kind == ProperKind::Psalm).expect("psalm found");
            assert_eq!(psalm.verses, expected, "{page:?}");
        }
    }

    #[test]
    fn pasting_just_the_verses() {
        let v = parse_psalm_text("Psalm 123\n1 To you I lift up my eyes, *\n   to you enthroned in the heavens.\n2 As the eyes of servants *\n   and the eyes of a maid,\n");
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].lines, vec!["As the eyes of servants *", "and the eyes of a maid,"]);
        assert!(parse_psalm_text("nothing here").is_empty());
    }

    #[test]
    fn psalm_slides_alternate_choir_and_congregation() {
        let p = parse(SITE);
        let mut psalm = p.items[3].clone();
        let doc = build_document(&psalm, None);
        let slides = doc.slides();
        assert_eq!(slides.len(), 3, "5 verses, two per slide");
        assert_eq!(slides[0].heading.as_deref(), Some("Psalm 123"));
        let verse = |s: usize, b: usize| match &slides[s].blocks[b] {
            Block::Paragraph { role, lines } => (role.clone(), lines[0][0].bold),
            other => panic!("{other:?}"),
        };
        // Verse numbers as labels, no Choir/Congregation roles; even verses bold.
        assert_eq!(verse(0, 0), (Some(Role::Custom("1".into())), false));
        assert_eq!(verse(0, 1), (Some(Role::Custom("2".into())), true));
        assert_eq!(verse(1, 0), (Some(Role::Custom("3".into())), false));
        assert_eq!(verse(2, 0), (Some(Role::Custom("5".into())), false));

        // With an antiphon at start, end and after every 2 verses.
        psalm.antiphon = Some("Our eyes look to the Lord our God.".into());
        psalm.antiphon_every = 2;
        let doc = build_document(&psalm, None);
        let slides = doc.slides();
        let headings: Vec<&str> = slides.iter().map(|s| s.heading.as_deref().unwrap_or("")).collect();
        assert_eq!(
            headings,
            vec!["Antiphon", "Psalm 123", "Antiphon", "Psalm 123", "Antiphon", "Psalm 123", "Antiphon"]
        );
        let antiphon = &slides[0].blocks[0];
        assert!(is_antiphon_role(antiphon.role()), "the antiphon is marked, not a speaker");
        assert_eq!(antiphon.plain_text(), "Our eyes look to the Lord our God.");

        // Round-trips through .ewm, and can be rearranged later.
        let reparsed = crate::ewm::parse(&crate::ewm::to_string(&doc)).unwrap();
        assert!(reparsed.warnings.is_empty(), "{:?}", reparsed.warnings);
        assert_eq!(reparsed.document, doc);
        assert_eq!(psalm_options(&doc).antiphon.as_deref(), Some("Our eyes look to the Lord our God."));
        assert_eq!(psalm_options(&doc).antiphon_every, 2);
        let plain = rearrange_psalm(&doc, &PsalmOptions::default());
        assert_eq!(plain.slides().len(), 3);
        let lines = |v: &[PsalmVerse]| v.iter().map(|v| (v.number.clone(), v.lines.clone())).collect::<Vec<_>>();
        assert_eq!(lines(&psalm_verses(&plain)), lines(&p.items[3].verses));
        let three = rearrange_psalm(&doc, &PsalmOptions { verses_per_slide: 3, ..PsalmOptions::default() });
        assert_eq!(three.slides().len(), 2);
    }

    #[test]
    fn choir_or_people_and_antiphons_where_placed_survive_rearranging() {
        let verses: Vec<PsalmVerse> = (1..=6)
            .map(|n| PsalmVerse::new(n.to_string(), vec![format!("Verse {n} *"), "second half".into()]))
            .collect();
        let image = Block::Image { alt: "Antiphon".into(), src: "/media/antiphon.png".into(), fit: None };
        let opts = PsalmOptions { antiphon: Some("Sing to the Lord".into()), ..PsalmOptions::default() };
        let mut doc = psalm_document("Psalm 96", Some("Psalm 96"), &verses, &opts, std::slice::from_ref(&image));
        // The operator marks verse 3 as the people's and adds an antiphon
        // after verse 3 by copying the first one.
        let v3 = doc.blocks.iter().position(|b| b.role() == Some(&Role::Custom("3".into()))).unwrap();
        if let Block::Paragraph { lines, .. } = &mut doc.blocks[v3] {
            *lines = lines.iter().map(|l| bold(&crate::document::line_plain_text(l))).collect();
        }
        let first: Vec<Block> = doc.blocks[..3].to_vec();
        assert!(matches!(first[1], Block::Image { .. }) && is_antiphon_role(first[2].role()));
        let mut insert = first.clone();
        insert.push(Block::Heading { level: 1, text: "Psalm 96".into() });
        doc.blocks.splice(v3 + 1..v3 + 1, insert);

        let verses = psalm_verses(&doc);
        let who: Vec<bool> = verses.iter().map(|v| v.people.unwrap()).collect();
        assert_eq!(who, vec![false, true, true, true, false, true]);
        assert_eq!(verses.iter().position(|v| v.antiphon_after), Some(2));
        assert_eq!(first_antiphon(&doc).as_deref(), Some("Sing to the Lord"));

        let placed = PsalmOptions { placed: true, ..psalm_options(&doc) };
        let again = rearrange_psalm(&doc, &placed);
        let headings: Vec<String> = again.slides().iter().map(|s| s.heading.clone().unwrap_or_default()).collect();
        assert_eq!(headings, vec!["Antiphon", "Psalm 96", "Psalm 96", "Antiphon", "Psalm 96", "Psalm 96", "Antiphon"]);
        assert_eq!(psalm_images(&again).len(), 1, "the music line is not duplicated");
        assert_eq!(
            psalm_verses(&again).iter().map(|v| v.people).collect::<Vec<_>>(),
            who.iter().map(|w| Some(*w)).collect::<Vec<_>>()
        );
        assert!(psalm_options(&again).placed);
    }

    #[test]
    fn builds_house_style_documents() {
        let p = parse(PAGE);
        let gospel = p.items.iter().find(|i| i.kind == ProperKind::Gospel).unwrap();
        let doc = build_document(gospel, p.day.as_deref());
        assert_eq!(doc.meta.category, Some(ElementCategory::Gospel));
        assert_eq!(doc.meta.citation.as_deref(), Some("Matthew 25:1-13"));
        assert!(doc.blocks.iter().any(|b| b.plain_text().contains("according to Matthew")));
        // The announcement comes first, then the title slide.
        assert!(doc.slides()[1].is_title());

        let collect = build_document(&p.items[0], p.day.as_deref());
        assert_eq!(collect.meta.title, "Collect of the Day");
        assert_eq!(collect.slides()[0].heading.as_deref(), Some("Twenty-Fourth Sunday after Pentecost"));
        assert_eq!(collect.blocks.last().unwrap().plain_text(), "Amen.");

        // Every imported document round-trips through the .ewm writer.
        for item in &p.items {
            let doc = build_document(item, p.day.as_deref());
            let src = crate::ewm::to_string(&doc);
            assert_eq!(crate::ewm::parse(&src).unwrap().document, doc, "{src}");
        }
    }
}
