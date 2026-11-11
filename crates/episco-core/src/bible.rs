//! Scripture texts for the readings: the NRSV or NRSVue, pasted by the
//! operator from a licensed source, cleaned up and laid out as slides.
//!
//! The New Revised Standard Version (1989) and its Updated Edition (2021)
//! are copyright the National Council of the Churches of Christ in the USA,
//! so the app does not ship or download their text. The NCC allows their use
//! in church services and orders of service that are not sold, provided the
//! version's initials follow each quotation; [`reading_blocks`] adds them.

use serde::{Deserialize, Serialize};

use crate::document::{Block, LiturgyDocument, Span};

/// A translation of the Bible for the readings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BibleVersion {
    /// New Revised Standard Version (1989).
    #[serde(rename = "NRSV")]
    Nrsv,
    /// New Revised Standard Version Updated Edition (2021).
    #[default]
    #[serde(rename = "NRSVue", alias = "NRSVUE")]
    Nrsvue,
}

impl BibleVersion {
    pub const ALL: [BibleVersion; 2] = [BibleVersion::Nrsvue, BibleVersion::Nrsv];

    /// The initials printed after each quotation.
    pub fn initials(self) -> &'static str {
        match self {
            BibleVersion::Nrsv => "NRSV",
            BibleVersion::Nrsvue => "NRSVue",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BibleVersion::Nrsv => "New Revised Standard Version",
            BibleVersion::Nrsvue => "New Revised Standard Version Updated Edition",
        }
    }

    pub fn parse(s: &str) -> Option<BibleVersion> {
        match s.trim().to_ascii_uppercase().as_str() {
            "NRSV" => Some(BibleVersion::Nrsv),
            "NRSVUE" => Some(BibleVersion::Nrsvue),
            _ => None,
        }
    }

    /// A web page with the passage in this version, for copying the text:
    /// Bible Gateway for the NRSVue, the Oremus Bible Browser for the NRSV.
    pub fn passage_url(self, citation: &str) -> String {
        let q = encode(&citation.replace(['[', ']'], ""));
        match self {
            BibleVersion::Nrsvue => format!("https://www.biblegateway.com/passage/?search={q}&version=NRSVUE"),
            BibleVersion::Nrsv => {
                format!("https://bible.oremus.org/?version=nrsv&vnum=no&fnote=no&heading=no&passage={q}")
            }
        }
    }
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Removes what comes along when a passage is copied from a Bible website:
/// the reference and version lines, section headings, verse and chapter
/// numbers, footnote and cross-reference markers, and the footnotes and
/// copyright notice at the end. Paragraphs are kept (blank-line separated).
pub fn clean_passage(text: &str) -> String {
    let mut paras: Vec<String> = Vec::new();
    let mut current = String::new();
    for raw in text.lines() {
        let line = raw.replace('\u{a0}', " ");
        let t = line.trim();
        let lower = t.to_lowercase();
        if lower == "footnotes"
            || lower == "cross references"
            || lower.starts_with("read full chapter")
            || lower.contains("copyright ©")
            || lower.starts_with("copyright")
            || lower.starts_with("scripture quotations")
        {
            break;
        }
        if t.is_empty() {
            if !current.trim().is_empty() {
                paras.push(std::mem::take(&mut current));
            }
            continue;
        }
        if lower.starts_with("new revised standard version") || is_reference(t) || is_heading(t) {
            continue;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(t);
    }
    if !current.trim().is_empty() {
        paras.push(current);
    }
    paras.iter().map(|p| strip_markers(p)).filter(|p| !p.is_empty()).collect::<Vec<_>>().join("\n\n")
}

/// "Judges 4:1-7", "1 Thessalonians 5:1-11" or "Isaiah 61:1-4, 8-11" on a
/// line of its own: a book name, then chapter:verse, and nothing like prose.
fn is_reference(t: &str) -> bool {
    let words: Vec<&str> = t.split_whitespace().collect();
    if words.is_empty() || words.len() > 8 || t.ends_with(['.', '?', '!', ';']) {
        return false;
    }
    let Some(at) = words.iter().position(|w| {
        let digits: String = w.chars().take_while(char::is_ascii_digit).collect();
        !digits.is_empty() && w[digits.len()..].starts_with(':')
    }) else {
        return false;
    };
    let book = &words[..at];
    !book.is_empty()
        && book.iter().enumerate().all(|(i, w)| {
            (i == 0 && w.chars().all(|c| c.is_ascii_digit()))
                || w.chars().next().is_some_and(char::is_uppercase)
                || *w == "of"
        })
        && words[at..].iter().all(|w| w.chars().all(|c| c.is_ascii_digit() || ":,-–—abc[]()".contains(c)))
}

/// A section heading such as "Deborah and Barak": short, no closing
/// punctuation, mostly capitalised words.
fn is_heading(t: &str) -> bool {
    let words: Vec<&str> = t.split_whitespace().collect();
    if words.is_empty() || words.len() > 8 || t.ends_with(['.', ',', ';', ':', '?', '!', '”', '"', '’']) {
        return false;
    }
    let caps = words.iter().filter(|w| w.chars().next().is_some_and(|c| c.is_uppercase())).count();
    caps * 2 >= words.len() && !t.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// Verse numbers ("4 The Israelites", "2So the Lord"), footnote markers
/// ("[a]") and cross-reference markers ("(A)", "(BC)").
fn strip_markers(p: &str) -> String {
    let chars: Vec<char> = p.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // [a] / [ab]
        if c == '[' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_lowercase() && j - i <= 3 {
                j += 1;
            }
            if j < chars.len() && chars[j] == ']' && j > i + 1 {
                i = j + 1;
                continue;
            }
        }
        // (A) / (AB)
        if c == '(' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_uppercase() && j - i <= 3 {
                j += 1;
            }
            if j < chars.len() && chars[j] == ')' && j > i + 1 {
                i = j + 1;
                continue;
            }
        }
        // A verse number at the start of a word, followed by a capital,
        // a quotation mark or a space and a letter.
        if c.is_ascii_digit() && (i == 0 || chars[i - 1].is_whitespace() || "“\"‘(".contains(chars[i - 1])) {
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_digit() && j - i < 3 {
                j += 1;
            }
            let next = chars.get(j).copied();
            let after = chars.get(j + 1).copied();
            let glued = next.is_some_and(|n| n.is_uppercase() || "“\"‘".contains(n));
            let spaced = next == Some(' ') && after.is_some_and(|a| a.is_alphabetic() || "“\"‘".contains(a));
            let numeral_word = after.is_some_and(|a| a.is_lowercase())
                && matches!(chars.get(j + 1..j + 6).map(|s| s.iter().collect::<String>()).as_deref(), Some(s) if s.starts_with("years") || s.starts_with("days") || s.starts_with("men"));
            if (glued || spaced) && !numeral_word && !(j < chars.len() && chars[j] == ':') {
                i = j;
                while i < chars.len() && chars[i] == ' ' {
                    i += 1;
                }
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Splits a cleaned passage into slide texts of about `words` words, at
/// sentence ends where possible.
pub fn slide_texts(text: &str, words: usize) -> Vec<String> {
    let words = words.max(10);
    let mut slides: Vec<String> = Vec::new();
    for para in text.split("\n\n") {
        let mut sentences: Vec<String> = Vec::new();
        let mut cur = String::new();
        let chars: Vec<char> = para.chars().collect();
        for (i, &c) in chars.iter().enumerate() {
            cur.push(c);
            let end = matches!(c, '.' | '?' | '!' | ';')
                || (matches!(c, '”' | '"' | '’') && i > 0 && matches!(chars[i - 1], '.' | '?' | '!'));
            if end && chars.get(i + 1).is_none_or(|n| *n == ' ') {
                sentences.push(cur.trim().to_string());
                cur.clear();
            }
        }
        if !cur.trim().is_empty() {
            sentences.push(cur.trim().to_string());
        }
        let count = |s: &str| s.split_whitespace().count();
        let mut slide = String::new();
        for s in sentences {
            // Break before a sentence that would overfill the slide, but not
            // while the slide is still less than half full.
            if !slide.is_empty() && count(&slide) + count(&s) > words && count(&slide) * 2 >= words {
                slides.push(std::mem::take(&mut slide));
            }
            if !slide.is_empty() {
                slide.push(' ');
            }
            slide.push_str(&s);
        }
        if !slide.is_empty() {
            // A short tail joins the slide before it when they fit together.
            match slides.last_mut() {
                Some(prev) if count(&slide) * 3 < words && count(prev) + count(&slide) <= words * 3 / 2 => {
                    prev.push(' ');
                    prev.push_str(&slide);
                }
                _ => slides.push(slide),
            }
        }
    }
    slides
}

/// The text of a reading as slide blocks (each followed by a slide break),
/// the last ending with the version's initials.
pub fn reading_blocks(text: &str, version: BibleVersion, words: usize) -> Vec<Block> {
    let slides = slide_texts(&clean_passage(text), words);
    let n = slides.len();
    let mut out = Vec::new();
    for (i, s) in slides.into_iter().enumerate() {
        let mut line = vec![Span::plain(s)];
        if i + 1 == n {
            line.push(Span::plain(format!(" ({})", version.initials())));
        }
        out.push(Block::Paragraph { role: None, lines: vec![line] });
        out.push(Block::SlideBreak);
    }
    out
}

/// Replaces a reading's text slides with `text` (or removes them when the
/// text is empty). The text goes after the title slide and the Gospel's
/// announcement, before the closing response ("The word of the Lord").
pub fn set_reading_text(doc: &LiturgyDocument, text: &str, version: BibleVersion, words: usize) -> LiturgyDocument {
    let mut blocks: Vec<Block> =
        doc.blocks.iter().filter(|b| !matches!(b, Block::Paragraph { role: None, .. })).cloned().collect();
    // Collapse the slide breaks the old text leaves behind.
    blocks.dedup_by(|a, b| matches!(a, Block::SlideBreak) && matches!(b, Block::SlideBreak));
    while matches!(blocks.last(), Some(Block::SlideBreak)) {
        blocks.pop();
    }
    let at = blocks.iter().rposition(|b| matches!(b, Block::SlideBreak)).map(|i| i + 1).unwrap_or(0);
    let new = if text.trim().is_empty() { Vec::new() } else { reading_blocks(text, version, words) };
    blocks.splice(at..at, new);
    let mut meta = doc.meta.clone();
    meta.extra.retain(|(k, _)| k != "bible");
    if !text.trim().is_empty() {
        meta.extra.push(("bible".into(), version.initials().into()));
    }
    LiturgyDocument { meta, blocks }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASTE: &str = "Judges 4:1-7, 12-13
New Revised Standard Version Updated Edition

Deborah and Barak

4 The Israelites again did what was evil in the sight of the Lord after Ehud died.[a] 2 So the Lord sold them into the hand of King Jabin of Canaan, who reigned in Hazor; the commander of his army was Sisera, who lived in Harosheth-ha-goiim.(A) 3Then the Israelites cried out to the Lord for help, for he had nine hundred chariots of iron and had oppressed the Israelites cruelly twenty years.

4 At that time Deborah, a prophet, wife of Lappidoth, was judging Israel.

Footnotes
Judges 4:1 Heb ...
New Revised Standard Version Updated Edition. Copyright © 2021 National Council of Churches of Christ in the United States of America.";

    #[test]
    fn cleans_a_copied_passage() {
        let c = clean_passage(PASTE);
        assert!(c.starts_with("The Israelites again did what was evil"), "{c}");
        assert!(c.contains("after Ehud died. So the Lord sold"), "{c}");
        assert!(c.contains("Harosheth-ha-goiim. Then the Israelites"), "{c}");
        assert!(c.contains("cruelly twenty years.\n\nAt that time Deborah"), "{c}");
        assert!(!c.contains("Deborah and Barak") && !c.contains("Judges 4"));
        assert!(is_reference("Isaiah 61:1-4, 8-11") && is_reference("1 Thessalonians 5:1-11"));
        assert!(is_reference("Song of Solomon 2:8-13"));
        assert!(!is_reference("He said to them 3:16 times."));
        assert!(!c.contains("Copyright") && !c.contains("Footnotes") && !c.contains("[a]") && !c.contains("(A)"));
    }

    #[test]
    fn lays_out_slides_and_credits_the_version() {
        let slides = slide_texts(&clean_passage(PASTE), 30);
        assert!(slides.len() >= 3, "{slides:?}");
        assert!(slides.iter().all(|s| s.split_whitespace().count() <= 45), "{slides:?}");
        // No slide is left nearly empty.
        let short = slide_texts("One two three four five six; seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twenty-one twenty-two twenty-three twenty-four twenty-five twenty-six twenty-seven twenty-eight; end here.", 30);
        assert_eq!(short.len(), 1, "{short:?}");
        let reading: LiturgyDocument = "---\ntitle: First Reading\ntype: reading\ncitation: Judges 4:1-7\n---\n# The First Reading\n\n---\n\n[Reader] The word of the Lord.\n[People] **Thanks be to God.**\n".parse().unwrap();
        let with = set_reading_text(&reading, PASTE, BibleVersion::Nrsvue, 30);
        let slides = with.slides();
        assert!(slides[0].is_title());
        assert!(slides[1].blocks[0].plain_text().starts_with("The Israelites"));
        let last_text = &slides[slides.len() - 2].blocks[0].plain_text();
        assert!(last_text.ends_with("(NRSVue)"), "{last_text}");
        assert_eq!(slides.last().unwrap().blocks[0].plain_text(), "The word of the Lord.");
        assert_eq!(with.meta.extra("bible"), Some("NRSVue"));
        // Replacing and removing the text leaves the responses intact.
        let again = set_reading_text(&with, "At that time Deborah judged Israel.", BibleVersion::Nrsv, 30);
        assert_eq!(again.slides().len(), 3);
        assert!(again.slides()[1].blocks[0].plain_text().ends_with("(NRSV)"));
        let none = set_reading_text(&again, "", BibleVersion::Nrsv, 30);
        assert_eq!(none.blocks, reading.blocks);
        assert_eq!(none.meta.extra("bible"), None);
    }

    #[test]
    fn passage_links() {
        assert_eq!(
            BibleVersion::Nrsvue.passage_url("Isaiah 58:1-9a [9b-12]"),
            "https://www.biblegateway.com/passage/?search=Isaiah+58%3A1-9a+9b-12&version=NRSVUE"
        );
        assert!(BibleVersion::Nrsv.passage_url("John 3:16").contains("oremus"));
    }
}
