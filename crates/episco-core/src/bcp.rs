//! Texts from the 1979 Book of Common Prayer (public domain), compiled in:
//!
//! * the Psalter (`library/bcp/psalter.txt`),
//! * the canticles of the Daily Office (`library/bcp/canticles.txt`),
//! * the Collects of the Church Year, Holy Days and the Common of Saints,
//!   contemporary (Rite II) and traditional (Rite I) (`library/bcp/collects.txt`).
//!
//! The files are generated from public-domain sources; see `library/README.md`.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::propers::PsalmVerse;

const PSALTER: &str = include_str!("../../../library/bcp/psalter.txt");
const CANTICLES: &str = include_str!("../../../library/bcp/canticles.txt");
const COLLECTS: &str = include_str!("../../../library/bcp/collects.txt");
const PREFACES: &str = include_str!("../../../library/bcp/prefaces.txt");
const PREFACE_DAYS: &str = include_str!("../../../library/bcp/preface-days.txt");

/// Rite I (traditional language) or Rite II (contemporary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Rite {
    #[serde(rename = "I", alias = "Rite I", alias = "1")]
    One,
    #[default]
    #[serde(rename = "II", alias = "Rite II", alias = "2")]
    Two,
}

impl Rite {
    pub fn label(self) -> &'static str {
        match self {
            Rite::One => "Rite I",
            Rite::Two => "Rite II",
        }
    }

    pub fn parse(s: &str) -> Option<Rite> {
        match s.trim().trim_start_matches("Rite").trim() {
            "I" | "1" | "i" => Some(Rite::One),
            "II" | "2" | "ii" => Some(Rite::Two),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Psalter
// ---------------------------------------------------------------------------

/// One verse of the Psalter: its halves (a, b and sometimes a third line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsalterVerse {
    pub number: u32,
    pub halves: Vec<String>,
}

fn psalter() -> &'static Vec<Vec<PsalterVerse>> {
    static P: OnceLock<Vec<Vec<PsalterVerse>>> = OnceLock::new();
    P.get_or_init(|| {
        let mut psalms: Vec<Vec<PsalterVerse>> = vec![Vec::new(); 151];
        let mut current = 0usize;
        for line in PSALTER.lines() {
            if let Some(n) = line.strip_prefix("@ ") {
                current = n.trim().parse().unwrap_or(0);
                continue;
            }
            if line.starts_with('#') || line.trim().is_empty() || current == 0 || current > 150 {
                continue;
            }
            let mut parts = line.split('|');
            let Some(number) = parts.next().and_then(|n| n.parse().ok()) else { continue };
            let halves = parts.filter(|h| !h.is_empty()).map(String::from).collect();
            psalms[current].push(PsalterVerse { number, halves });
        }
        psalms
    })
}

/// The verses of a whole psalm (1–150).
pub fn psalm(n: u32) -> Option<&'static [PsalterVerse]> {
    psalter().get(n as usize).map(Vec::as_slice).filter(|v| !v.is_empty())
}

/// A verse or part of one in a psalm citation: `7`, `7a`, `9b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VersePoint {
    verse: u32,
    /// 0: whole verse; 1, 2, 3: half a, b, c.
    part: u8,
}

fn point(s: &str) -> Option<VersePoint> {
    let s = s.trim();
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    let verse = digits.parse().ok()?;
    let part = match s[digits.len()..].trim() {
        "" => 0,
        "a" => 1,
        "b" => 2,
        "c" => 3,
        _ => return None,
    };
    Some(VersePoint { verse, part })
}

/// A psalm reference: the psalm and its verse ranges (each marked optional
/// when it was in brackets or parentheses).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsalmRef {
    pub psalm: u32,
    /// `(from, to, optional)`; empty for the whole psalm.
    ranges: Vec<(VersePoint, VersePoint, bool)>,
}

/// Parses `Psalm 72:1-7, 18-19`, `Ps. 90:1-8,(9-11),12`, `Psalm 69:8-11 [12-17] 18-20`,
/// `Psalm 65:[1-8] 9-14`, `Psalm 105:1-11, 45b`, `Psalm 23`. Text after the
/// reference (e.g. ` (or 103:8-14)`) must be removed first.
pub fn parse_psalm_ref(citation: &str) -> Option<PsalmRef> {
    let c = citation.trim();
    let lower = c.to_ascii_lowercase();
    let rest =
        ["psalms ", "psalm ", "ps. ", "ps "].iter().find_map(|p| lower.strip_prefix(p).map(|_| &c[p.len()..]))?;
    let rest = rest.trim();
    let (num, verses) = match rest.split_once(':') {
        Some((n, v)) => (n.trim(), v),
        None => (rest, ""),
    };
    let psalm: u32 = num.parse().ok()?;
    if !(1..=150).contains(&psalm) {
        return None;
    }
    let mut ranges = Vec::new();
    let mut optional = false;
    let mut token = String::new();
    let flush = |token: &mut String, optional: bool, ranges: &mut Vec<(VersePoint, VersePoint, bool)>| -> Option<()> {
        let t = token.trim().trim_matches(',').trim().to_string();
        token.clear();
        if t.is_empty() {
            return Some(());
        }
        let (a, b) = match t.split_once(['-', '–', '—']) {
            Some((a, b)) => (point(a)?, point(b)?),
            None => {
                let p = point(&t)?;
                (p, p)
            }
        };
        ranges.push((a, b, optional));
        Some(())
    };
    for ch in verses.chars() {
        match ch {
            '[' | '(' => {
                flush(&mut token, optional, &mut ranges)?;
                optional = true;
            }
            ']' | ')' => {
                flush(&mut token, optional, &mut ranges)?;
                optional = false;
            }
            ',' | ' ' | ';' => flush(&mut token, optional, &mut ranges)?,
            _ => token.push(ch),
        }
    }
    flush(&mut token, optional, &mut ranges)?;
    Some(PsalmRef { psalm, ranges })
}

/// Formats a verse's halves as slide lines: the first half ends with the
/// chant asterisk, as printed in the Prayer Book.
fn verse_lines(halves: &[String]) -> Vec<String> {
    let mut lines: Vec<String> = halves.to_vec();
    if lines.len() > 1 {
        // A no-break space keeps the asterisk with the last word when the line wraps.
        lines[0] = format!("{}\u{a0}*", lines[0].trim_end());
    }
    lines
}

/// The verses of a psalm citation, ready for slides. Optional verses (in
/// brackets) are included when `include_optional` is set. A half verse
/// (`45b`) brings only that half.
pub fn psalm_selection(citation: &str, include_optional: bool) -> Option<Vec<PsalmVerse>> {
    let r = parse_psalm_ref(citation)?;
    let all = psalm(r.psalm)?;
    if r.ranges.is_empty() {
        return Some(all.iter().map(|v| PsalmVerse::new(v.number.to_string(), verse_lines(&v.halves))).collect());
    }
    let mut out = Vec::new();
    for (from, to, optional) in &r.ranges {
        if *optional && !include_optional {
            continue;
        }
        for v in all.iter().filter(|v| v.number >= from.verse && v.number <= to.verse) {
            let len = v.halves.len();
            let start = if v.number == from.verse && from.part > 0 { (from.part as usize - 1).min(len) } else { 0 };
            let end = if v.number == to.verse && to.part > 0 { (to.part as usize).min(len) } else { len };
            if start >= end {
                continue;
            }
            if start == 0 && end == len {
                out.push(PsalmVerse::new(v.number.to_string(), verse_lines(&v.halves)));
                continue;
            }
            let mut lines: Vec<String> = v.halves[start..end].to_vec();
            if start > 0 {
                // A verse that starts mid-way begins with a capital, as printed.
                let mut c = lines[0].chars();
                if let Some(f) = c.next() {
                    lines[0] = f.to_uppercase().collect::<String>() + c.as_str();
                }
            }
            let suffix = if start > 0 { ["a", "b", "c"][start.min(2)] } else { "a" };
            out.push(PsalmVerse::new(format!("{}{suffix}", v.number), lines));
        }
    }
    (!out.is_empty()).then_some(out)
}

// ---------------------------------------------------------------------------
// Canticles
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canticle {
    pub number: u32,
    pub title: String,
    pub latin: String,
    pub scripture: String,
    pub verses: Vec<Vec<String>>,
}

fn canticles() -> &'static Vec<Canticle> {
    static C: OnceLock<Vec<Canticle>> = OnceLock::new();
    C.get_or_init(|| {
        let mut out: Vec<Canticle> = Vec::new();
        for line in CANTICLES.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if let Some(head) = line.strip_prefix("@ ") {
                let mut f = head.split('|');
                let number = f.next().and_then(|n| n.trim().parse().ok()).unwrap_or(0);
                out.push(Canticle {
                    number,
                    title: f.next().unwrap_or("").to_string(),
                    latin: f.next().unwrap_or("").to_string(),
                    scripture: f.next().unwrap_or("").to_string(),
                    verses: Vec::new(),
                });
            } else if let Some(c) = out.last_mut() {
                c.verses.push(line.split('|').map(String::from).collect());
            }
        }
        out
    })
}

pub fn canticle(number: u32) -> Option<&'static Canticle> {
    canticles().iter().find(|c| c.number == number)
}

/// The traditional-language (Rite I) form of a contemporary canticle, where
/// the Prayer Book has one (e.g. Canticle 15, the Magnificat → Canticle 3).
pub fn traditional_canticle(number: u32) -> Option<u32> {
    match number {
        12 => Some(1),
        13 => Some(2),
        15 => Some(3),
        16 => Some(4),
        17 => Some(5),
        20 => Some(6),
        21 => Some(7),
        _ => None,
    }
}

/// `Canticle 15: Magnificat` → 15.
pub fn canticle_number(citation: &str) -> Option<u32> {
    let rest = citation.trim().strip_prefix("Canticle")?.trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Canticle verses as psalm verses (numbered in order) for slides.
pub fn canticle_verses(number: u32) -> Option<Vec<PsalmVerse>> {
    let c = canticle(number)?;
    Some(c.verses.iter().enumerate().map(|(i, lines)| PsalmVerse::new((i + 1).to_string(), lines.clone())).collect())
}

// ---------------------------------------------------------------------------
// Collects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CollectEntry {
    pub title: String,
    /// Contemporary (Rite II); more than one where the book offers "or this".
    pub contemporary: Vec<String>,
    /// Traditional (Rite I).
    pub traditional: Vec<String>,
}

impl CollectEntry {
    pub fn texts(&self, rite: Rite) -> &[String] {
        match rite {
            Rite::One => &self.traditional,
            Rite::Two => &self.contemporary,
        }
    }
}

pub fn collects() -> &'static Vec<CollectEntry> {
    static C: OnceLock<Vec<CollectEntry>> = OnceLock::new();
    C.get_or_init(|| {
        let mut out: Vec<CollectEntry> = Vec::new();
        for line in COLLECTS.lines() {
            if let Some(t) = line.strip_prefix("== ") {
                out.push(CollectEntry {
                    title: t.trim().to_string(),
                    contemporary: Vec::new(),
                    traditional: Vec::new(),
                });
            } else if let (Some(text), Some(c)) = (line.strip_prefix("II: "), out.last_mut()) {
                c.contemporary.push(text.to_string());
            } else if let (Some(text), Some(c)) = (line.strip_prefix("I: "), out.last_mut()) {
                c.traditional.push(text.to_string());
            }
        }
        out
    })
}

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .replace(['\u{2019}', '\''], "")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !matches!(*w, "the" | "of" | "our" | "a"))
        .map(String::from)
        .collect()
}

/// Finds a collect by name as the lectionary gives it ("Proper 28",
/// "Baptism of Our Lord", "Sunday after Ascension Day", "Saint James"): the
/// shortest title containing every word of the name.
pub fn find_collect(name: &str) -> Option<&'static CollectEntry> {
    let name = name.split(" (").next().unwrap_or(name);
    let name = name.split(" / ").next().unwrap_or(name);
    let want = words(name);
    if want.is_empty() {
        return None;
    }
    collects()
        .iter()
        .filter_map(|c| {
            let have = words(&c.title);
            want.iter().all(|w| have.contains(w)).then_some((have.len() - want.len().min(have.len()), c))
        })
        .min_by_key(|(extra, c)| (*extra, c.title.len()))
        .map(|(_, c)| c)
}

// ---------------------------------------------------------------------------
// Proper Prefaces
// ---------------------------------------------------------------------------

/// A Proper Preface (BCP pp. 344-349, 377-382), e.g. "Easter" or "The Lord's
/// Day: Of God the Son".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrefaceEntry {
    pub name: String,
    /// Contemporary (Rite II); more than one where the book offers "or this".
    pub contemporary: Vec<String>,
    /// Traditional (Rite I).
    pub traditional: Vec<String>,
}

impl PrefaceEntry {
    /// The preface in a rite (the first, where the book gives a choice).
    pub fn text(&self, rite: Rite) -> Option<&str> {
        match rite {
            Rite::One => self.traditional.first(),
            Rite::Two => self.contemporary.first(),
        }
        .map(String::as_str)
    }
}

/// Every Proper Preface, in the book's order.
pub fn prefaces() -> &'static Vec<PrefaceEntry> {
    static P: OnceLock<Vec<PrefaceEntry>> = OnceLock::new();
    P.get_or_init(|| {
        let mut out: Vec<PrefaceEntry> = Vec::new();
        for line in PREFACES.lines() {
            if let Some(t) = line.strip_prefix("== ") {
                out.push(PrefaceEntry { name: t.trim().into(), contemporary: Vec::new(), traditional: Vec::new() });
            } else if let (Some(text), Some(p)) = (line.strip_prefix("II: "), out.last_mut()) {
                p.contemporary.push(text.into());
            } else if let (Some(text), Some(p)) = (line.strip_prefix("I: "), out.last_mut()) {
                p.traditional.push(text.into());
            }
        }
        out
    })
}

/// A preface by name. "The Lord's Day" alone is the first of its three.
pub fn preface(name: &str) -> Option<&'static PrefaceEntry> {
    let all = prefaces();
    all.iter().find(|p| p.name == name).or_else(|| all.iter().find(|p| p.name.starts_with(&format!("{name}: "))))
}

/// The prefaces a day's collect is appointed with (the note under it:
/// "Preface of the Epiphany, or of the Lord's Day"), by the collect's
/// title. `["None"]` where the book says no Proper Preface is used;
/// "Season" means the season's own.
pub fn preface_names_for(collect_title: &str) -> Option<Vec<String>> {
    static D: OnceLock<Vec<(String, Vec<String>)>> = OnceLock::new();
    let days = D.get_or_init(|| {
        PREFACE_DAYS
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once(" => "))
            .map(|(day, names)| (day.trim().to_string(), names.split(" | ").map(|n| n.trim().to_string()).collect()))
            .collect()
    });
    if let Some((_, names)) = days.iter().find(|(d, _)| d == collect_title) {
        return Some(names.clone());
    }
    // The two sources spell a few titles differently: the closest by words.
    let want = words(collect_title);
    days.iter()
        .filter(|(d, _)| {
            let have = words(d);
            !have.is_empty() && have.iter().all(|w| want.contains(w)) && want.iter().all(|w| have.contains(w))
        })
        .map(|(_, n)| n.clone())
        .next()
}

/// Puts a preface in a Eucharistic Prayer after its rubric "Here a Proper
/// Preface is sung or said …", for the Celebrant. `None` for a prayer
/// without that place (Prayers C and D).
pub fn with_preface(doc: &crate::LiturgyDocument, text: &str) -> Option<crate::LiturgyDocument> {
    use crate::document::{Block, Span};
    use crate::role::Role;
    let at = doc.blocks.iter().position(|b| {
        matches!(b, Block::Paragraph { role: Some(Role::Rubric), .. }) && b.plain_text().contains("Proper Preface")
    })?;
    let mut out = doc.clone();
    out.blocks.insert(at + 1, Block::Paragraph { role: Some(Role::Celebrant), lines: vec![vec![Span::plain(text)]] });
    Some(out)
}

/// The rubric that marks where the Proper Preface goes.
pub const PREFACE_RUBRIC: &str =
    "Here a Proper Preface is sung or said on all Sundays, and on other occasions as appointed.";

/// A parish's own Eucharistic Prayer (pasted, e.g. from Enriching Our
/// Worship) with the place for the Proper Preface marked, if it has none
/// yet: after the presider's first words following the people's "It is
/// right to give … thanks and praise", as in Prayers A and B. Unchanged if
/// the place is marked already or cannot be found.
pub fn mark_preface_place(doc: &crate::LiturgyDocument) -> crate::LiturgyDocument {
    use crate::document::{Block, Span};
    use crate::role::Role;
    let has = doc.blocks.iter().any(|b| {
        matches!(b, Block::Paragraph { role: Some(Role::Rubric), .. }) && b.plain_text().contains("Proper Preface")
    });
    if has {
        return doc.clone();
    }
    let is_people = |b: &Block| matches!(b, Block::Paragraph { role: Some(r), .. } if r.is_congregational());
    let Some(right) = doc.blocks.iter().position(|b| {
        let t = b.plain_text().to_lowercase();
        is_people(b) && t.contains("right to give")
    }) else {
        return doc.clone();
    };
    let Some(at) = doc.blocks.iter().enumerate().skip(right + 1).find_map(|(i, b)| {
        matches!(b, Block::Paragraph { role: Some(r), .. } if !r.is_rubric() && !r.is_congregational()).then_some(i)
    }) else {
        return doc.clone();
    };
    let mut out = doc.clone();
    out.blocks
        .insert(at + 1, Block::Paragraph { role: Some(Role::Rubric), lines: vec![vec![Span::italic(PREFACE_RUBRIC)]] });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_parish_prayer_gets_the_preface_after_its_opening_words() {
        let doc: crate::LiturgyDocument = "---\ntitle: Eucharistic Prayer 1 (EOW)\ntype: eucharistic_prayer\n---\n[Celebrant] The Lord be with you.\n[People] **And also with you.**\n[Celebrant] Lift up your hearts.\n[People] **We lift them to the Lord.**\n[Celebrant] Let us give thanks to the Lord our God.\n[People] **It is right to give our thanks and praise.**\n\n[Celebrant] It is truly right to glorify you.\n\n[Celebrant] Therefore we praise you.\n".parse().unwrap();
        let marked = mark_preface_place(&doc);
        let texts: Vec<String> = marked.blocks.iter().map(|b| b.plain_text()).collect();
        let at = texts.iter().position(|t| t.contains("Proper Preface")).expect("marked");
        assert_eq!(texts[at - 1], "It is truly right to glorify you.");
        assert_eq!(texts[at + 1], "Therefore we praise you.");
        // The preface then goes in.
        assert!(with_preface(&marked, "For you are the source of light.").is_some());
        // Marked once only.
        assert_eq!(mark_preface_place(&marked), marked);
    }

    #[test]
    fn psalter_is_complete() {
        assert_eq!((1..=150).filter(|n| psalm(*n).is_some()).count(), 150);
        assert_eq!(psalm(117).unwrap().len(), 2);
        assert_eq!(psalm(119).unwrap().len(), 176);
        let p23 = psalm(23).unwrap();
        assert_eq!(p23[0].halves, vec!["The Lord is my shepherd;", "I shall not be in want."]);
    }

    #[test]
    fn psalm_citations() {
        let v = psalm_selection("Psalm 123", true).unwrap();
        assert_eq!(v.len(), 5);
        assert_eq!(v[0].number, "1");
        assert_eq!(v[0].lines, vec!["To you I lift up my eyes,\u{a0}*", "to you enthroned in the heavens."]);

        let v = psalm_selection("Psalm 72:1-7, 18-19", true).unwrap();
        let n: Vec<&str> = v.iter().map(|v| v.number.as_str()).collect();
        assert_eq!(n, vec!["1", "2", "3", "4", "5", "6", "7", "18", "19"]);

        let with = psalm_selection("Psalm 69:8-11 [12-17] 18-20", true).unwrap();
        let without = psalm_selection("Psalm 69:8-11 [12-17] 18-20", false).unwrap();
        assert_eq!(with.len(), 13);
        assert_eq!(without.len(), 7);
        assert_eq!(psalm_selection("Ps. 90:1-8,(9-11),12", true).unwrap().len(), 12);
        assert_eq!(psalm_selection("Psalm 65:[1-8] 9-14", false).unwrap()[0].number, "9");

        // Half verses.
        let v = psalm_selection("Psalm 105:1-11, 45b", true).unwrap();
        assert_eq!(v.len(), 12);
        assert_eq!(v[11].number, "45b");
        assert_eq!(v[11].lines.len(), 1);
        let v = psalm_selection("Psalm 95:1-7a", true).unwrap();
        assert_eq!(v.last().unwrap().number, "7a");
        assert!(!v.last().unwrap().lines[0].ends_with('*'));

        assert!(psalm_selection("Psalm 151", true).is_none());
        assert!(psalm_selection("1 Samuel 2:1-10", true).is_none());
    }

    #[test]
    fn canticles_and_rite_one_forms() {
        assert_eq!(canticle_number("Canticle 15: Magnificat"), Some(15));
        let m = canticle(15).unwrap();
        assert_eq!(m.latin, "Magnificat");
        assert!(m.verses[0][0].starts_with("My soul proclaims the greatness of the Lord"));
        let t = canticle(traditional_canticle(15).unwrap()).unwrap();
        assert!(t.verses[0][0].starts_with("My soul doth magnify the Lord"), "{:?}", t.verses[0]);
        assert_eq!(canticle_verses(9).unwrap().len(), 7);
    }

    #[test]
    fn proper_prefaces() {
        assert_eq!(prefaces().len(), 19);
        let easter = preface("Easter").unwrap();
        assert!(easter.text(Rite::Two).unwrap().starts_with("But chiefly are we bound to praise you"));
        assert!(easter.text(Rite::One).unwrap().starts_with("But chiefly are we bound to praise thee"));
        assert_eq!(preface("The Lord's Day").unwrap().name, "The Lord's Day: Of God the Father");
        assert_eq!(preface("Lent").unwrap().contemporary.len(), 2);
        assert!(preface("A Saint").unwrap().traditional[2].starts_with("Because thou art greatly"));
        // Every name the day table uses is a preface.
        for line in PREFACE_DAYS.lines().filter(|l| !l.starts_with('#')) {
            for name in line.split(" => ").nth(1).unwrap().split(" | ") {
                assert!(matches!(name, "None" | "Season") || preface(name).is_some(), "{line}");
            }
        }
        // Every collect of the seasons and holy days has its preface.
        let missing: Vec<&str> = collects()
            .iter()
            .take_while(|c| c.title != "Of a Martyr")
            .filter(|c| preface_names_for(&c.title).is_none())
            .map(|c| c.title.as_str())
            .collect();
        // No Eucharistic Prayer on these days.
        assert_eq!(missing, ["Good Friday", "Holy Saturday"]);
        assert_eq!(preface_names_for("Second Sunday after the Epiphany").unwrap(), ["Epiphany", "The Lord's Day"]);
        assert_eq!(preface_names_for("Proper 1").unwrap(), ["None"]);
    }

    #[test]
    fn a_preface_goes_after_its_rubric() {
        let doc: crate::LiturgyDocument = "---\ntitle: EP\n---\n[Celebrant] It is right.\n\n---\n\n[Rubric] *Here a Proper Preface is sung or said.*\n[Celebrant] Therefore we praise you.\n".parse().unwrap();
        let with = with_preface(&doc, "For you are the source of light and life.").unwrap();
        let texts: Vec<String> = with.blocks.iter().map(|b| b.plain_text()).collect();
        let at = texts.iter().position(|t| t.starts_with("For you are")).unwrap();
        assert!(texts[at - 1].contains("Proper Preface") && texts[at + 1].starts_with("Therefore"));
        let c: crate::LiturgyDocument = "---\ntitle: C\n---\n[Celebrant] The Lord be with you.\n".parse().unwrap();
        assert!(with_preface(&c, "x").is_none());
    }

    #[test]
    fn collects_in_both_rites() {
        assert!(collects().len() > 100);
        let p28 = find_collect("Proper 28 (BCP p. 236)").unwrap();
        assert_eq!(p28.title, "Proper 28");
        assert!(p28.texts(Rite::Two)[0].starts_with("Blessed Lord, who caused all holy Scriptures"));
        assert!(p28.texts(Rite::One)[0].starts_with("Blessed Lord, who hast caused all holy Scriptures"));
        assert_eq!(find_collect("Proper 2").unwrap().title, "Proper 2");
        assert_eq!(find_collect("Proper 29 / Christ the King").unwrap().title, "Proper 29");
        for (name, title) in [
            ("Baptism of Our Lord", "First Sunday after the Epiphany: The Baptism of our Lord"),
            ("Sunday after Ascension Day", "Seventh Sunday of Easter: The Sunday after Ascension Day"),
            ("Palm Sunday", "Sunday of the Passion: Palm Sunday"),
            ("Christmas Day", "The Nativity of Our Lord: Christmas Day"),
            ("Saint James", "Saint James"),
            ("All Saints' Day", "All Saints' Day"),
            ("First Sunday after Christmas", "First Sunday after Christmas Day"),
            ("Of a Martyr", "Of a Martyr"),
        ] {
            assert_eq!(find_collect(name).map(|c| c.title.as_str()), Some(title), "{name}");
        }
        assert_eq!(find_collect("The Nativity of Our Lord: Christmas Day").unwrap().contemporary.len(), 3);
    }
}
