//! Readings and the collect for a day, from text the operator pastes in.
//!
//! The Episcopal lectionary site refuses automated requests, so the operator
//! copies the day's page in a browser and pastes it into the app. This module
//! finds what matters for the screens without depending on the page's exact
//! layout:
//!
//! * the day's name ("The Twenty-Fourth Sunday after Pentecost"),
//! * the collect: the paragraph after a "Collect" heading, up to "Amen.",
//! * every scripture citation, labelled by the nearest heading above it
//!   ("First Reading", "Psalm", "Epistle", "Gospel", "Track 1", …).
//!
//! The operator ticks which items to add; [`build_document`] turns each into
//! an `.ewm` document in the house style (title slide + responses).
//!
//! Reading *texts* are deliberately not imported: the screens show only
//! citations and responses, and Bible translations are copyrighted.

use serde::{Deserialize, Serialize};

use crate::category::ElementCategory;
use crate::document::{Block, FrontMatter, LiturgyDocument, Span};
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

fn heading_kind(line: &str) -> Option<Option<ProperKind>> {
    let l = line.to_ascii_lowercase();
    let l = l.trim_end_matches(':').trim();
    if l.len() > 60 {
        return None;
    }
    let k = if l.contains("collect") {
        Some(ProperKind::Collect)
    } else if l.contains("gospel") {
        Some(ProperKind::Gospel)
    } else if l.starts_with("psalm") && citation_at(&normalize(line)).is_none() || l == "psalm" || l == "the psalm" {
        Some(ProperKind::Psalm)
    } else if l.contains("first reading")
        || l.contains("old testament")
        || l.contains("hebrew bible")
        || l.contains("first lesson")
        || l == "reading 1"
    {
        Some(ProperKind::FirstReading)
    } else if l.contains("second reading")
        || l.contains("epistle")
        || l.contains("new testament")
        || l.contains("second lesson")
        || l == "reading 2"
    {
        Some(ProperKind::SecondReading)
    } else if l.starts_with("track") || l.starts_with("year ") || l == "or" {
        // A variant heading; handled by the caller.
        return Some(None);
    } else {
        return None;
    };
    Some(k)
}

fn looks_like_day(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    line.len() < 90
        && (l.contains("sunday") || l.contains(" day") || l.starts_with("proper ") || l.contains("eve of"))
        && !l.contains("collect")
}

/// Extracts the day, the collect and the scripture citations.
pub fn parse(text: &str) -> Propers {
    let lines: Vec<String> = text.lines().map(normalize).filter(|l| !l.is_empty()).collect();
    let mut out = Propers::default();
    let mut current: Option<ProperKind> = None;
    let mut variant: Option<String> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if out.day.is_none() && looks_like_day(line) && citation_at(line).is_none() {
            out.day = Some(line.trim_end_matches(':').to_string());
            i += 1;
            continue;
        }
        if let Some(k) = heading_kind(line) {
            match k {
                Some(ProperKind::Collect) => {
                    // Collect text: following lines up to and including "Amen."
                    let mut body = Vec::new();
                    let mut j = i + 1;
                    while j < lines.len() {
                        let l = &lines[j];
                        if heading_kind(l).is_some() && body.is_empty() {
                            break;
                        }
                        body.push(l.clone());
                        j += 1;
                        if l.trim_end().ends_with("Amen.") || l.trim_end().ends_with("Amen") {
                            break;
                        }
                    }
                    let joined = body.join(" ");
                    let text = joined.trim().trim_end_matches("Amen.").trim_end_matches("Amen").trim().to_string();
                    if !text.is_empty() && out.items.iter().all(|it| it.kind != ProperKind::Collect || it.text != text)
                    {
                        out.items.push(ProperItem {
                            kind: ProperKind::Collect,
                            citation: String::new(),
                            text,
                            variant: variant.clone(),
                            alternative: false,
                        });
                    }
                    i = j;
                    continue;
                }
                Some(kind) => {
                    current = Some(kind);
                    // A heading may carry its citation on the same line:
                    // "Gospel: Matthew 25:1-13".
                    if let Some((_, rest)) = line.split_once(':') {
                        scan_citations(&normalize(rest), current, &variant, &mut out);
                    }
                }
                None => {
                    let l = line.to_ascii_lowercase();
                    if l.starts_with("track") || l.starts_with("year ") {
                        variant = Some(line.trim_end_matches(':').to_string());
                        // A new track starts its own sequence of readings.
                        current = None;
                    }
                }
            }
            i += 1;
            continue;
        }
        scan_citations(line, current, &variant, &mut out);
        i += 1;
    }
    out
}

fn scan_citations(line: &str, current: Option<ProperKind>, variant: &Option<String>, out: &mut Propers) {
    let mut pos = 0;
    let mut previous_end = 0;
    while pos < line.len() {
        if !line.is_char_boundary(pos) {
            pos += 1;
            continue;
        }
        let at_word_start = pos == 0 || line.as_bytes()[pos - 1] == b' ' || line.as_bytes()[pos - 1] == b'(';
        if at_word_start {
            if let Some(len) = citation_at(&line[pos..]) {
                let citation = clean_citation(&line[pos..pos + len]);
                let between = line[previous_end..pos].to_ascii_lowercase();
                let alternative = between.contains(" or") || between.trim() == "or";
                let previous =
                    out.items.iter().rev().find(|it| it.kind != ProperKind::Collect && it.variant == *variant);
                let kind = if citation.starts_with("Psalm") {
                    ProperKind::Psalm
                } else if ["Matthew", "Mark", "Luke", "John "].iter().any(|g| citation.starts_with(g)) {
                    ProperKind::Gospel
                } else if let Some(prev) = previous.filter(|_| alternative) {
                    // "A or B": same slot as the reading before it.
                    prev.kind
                } else {
                    match current {
                        Some(k @ (ProperKind::FirstReading | ProperKind::SecondReading)) => k,
                        // Lectionary order: a reading before the psalm is the
                        // first reading, after it the second.
                        _ if previous.is_some_and(|p| p.kind == ProperKind::Psalm) => ProperKind::SecondReading,
                        _ if previous.is_none() => ProperKind::FirstReading,
                        _ => ProperKind::Reading,
                    }
                };
                // Tracks only distinguish the first reading and psalm; the
                // epistle and Gospel are shared by both.
                let variant = match kind {
                    ProperKind::SecondReading | ProperKind::Gospel => None,
                    _ => variant.clone(),
                };
                let dup = out.items.iter().any(|it| it.citation == citation && it.variant == variant);
                if !dup {
                    out.items.push(ProperItem { kind, citation, text: String::new(), variant, alternative });
                }
                pos += len;
                previous_end = pos;
                continue;
            }
        }
        pos += 1;
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
    let mut meta = FrontMatter::default();
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
            blocks.push(Block::Heading { level: 1, text: "The Holy Gospel".into() });
            blocks.push(Block::SlideBreak);
            blocks.push(Block::speech(
                Role::Deacon,
                [format!("The Holy Gospel of our Lord Jesus Christ according to {who}.")],
            ));
            blocks.push(Block::Paragraph { role: Some(Role::People), lines: vec![bold("Glory to you, Lord Christ.")] });
            blocks.push(Block::SlideBreak);
            blocks.push(Block::speech(Role::Deacon, ["The Gospel of the Lord."]));
            blocks
                .push(Block::Paragraph { role: Some(Role::People), lines: vec![bold("Praise to you, Lord Christ.")] });
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn citation_on_the_heading_line_and_inline_text() {
        let p = parse("Gospel: John 11:32-44\nFirst Reading: Isaiah 25:6-9\nsee also 1 John 3:1-3.");
        let c: Vec<&str> = p.items.iter().map(|i| i.citation.as_str()).collect();
        assert_eq!(c, vec!["John 11:32-44", "Isaiah 25:6-9", "1 John 3:1-3"]);
        assert_eq!(p.items[0].kind, ProperKind::Gospel);
        assert_eq!(p.items[1].kind, ProperKind::FirstReading);
        assert_ne!(p.items[2].kind, ProperKind::Gospel, "1 John is not a Gospel");
    }

    #[test]
    fn words_after_a_number_are_not_part_of_a_citation() {
        let p = parse("Psalm 23 and the readings for today");
        assert_eq!(p.items[0].citation, "Psalm 23");
    }

    #[test]
    fn builds_house_style_documents() {
        let p = parse(PAGE);
        let gospel = p.items.iter().find(|i| i.kind == ProperKind::Gospel).unwrap();
        let doc = build_document(gospel, p.day.as_deref());
        assert_eq!(doc.meta.category, Some(ElementCategory::Gospel));
        assert_eq!(doc.meta.citation.as_deref(), Some("Matthew 25:1-13"));
        assert!(doc.blocks.iter().any(|b| b.plain_text().contains("according to Matthew")));
        assert!(doc.slides()[0].is_title());

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
