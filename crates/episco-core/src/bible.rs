//! Scripture texts for the readings: the NRSV or NRSVue, pasted by the
//! operator from a licensed source, cleaned up and laid out as slides.
//!
//! The New Revised Standard Version (1989) and its Updated Edition (2021)
//! are copyright the National Council of the Churches of Christ in the USA,
//! so the app does not ship or download their text. The NCC allows their use
//! in church services and orders of service that are not sold, provided the
//! version's initials follow each quotation; [`reading_blocks`] adds them.

use std::collections::{BTreeMap, HashMap};

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

/// One slide of a reading's text: the pieces of paragraphs on it (one
/// line each), and whether it starts a paragraph or carries one on from the
/// slide before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSlide {
    pub parts: Vec<String>,
    pub new_paragraph: bool,
}

/// Splits a cleaned passage into slide texts of about `words` words, at
/// sentence ends where possible.
pub fn slide_texts(text: &str, words: usize) -> Vec<String> {
    text_slides(text, words).into_iter().map(|s| s.parts.join(" ")).collect()
}

/// Like [`slide_texts`], keeping track of the paragraphs.
pub fn text_slides(text: &str, words: usize) -> Vec<TextSlide> {
    let words = words.max(10);
    let mut slides: Vec<TextSlide> = Vec::new();
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
        let mut first = true;
        for s in sentences {
            // Break before a sentence that would overfill the slide, but not
            // while the slide is still less than half full.
            if !slide.is_empty() && count(&slide) + count(&s) > words && count(&slide) * 2 >= words {
                slides.push(TextSlide { parts: vec![std::mem::take(&mut slide)], new_paragraph: first });
                first = false;
            }
            if !slide.is_empty() {
                slide.push(' ');
            }
            slide.push_str(&s);
        }
        if !slide.is_empty() {
            // A short tail joins the slide before it when they fit together.
            match slides.last_mut() {
                Some(prev)
                    if count(&slide) * 3 < words && count(&prev.parts.join(" ")) + count(&slide) <= words * 3 / 2 =>
                {
                    if first {
                        prev.parts.push(slide);
                    } else {
                        let last = prev.parts.last_mut().expect("a slide has text");
                        last.push(' ');
                        last.push_str(&slide);
                    }
                }
                _ => slides.push(TextSlide { parts: vec![slide], new_paragraph: first }),
            }
        }
    }
    slides
}

/// The text of a reading as slide blocks (each followed by a slide break),
/// the last ending with the version's initials.
pub fn reading_blocks(text: &str, version: BibleVersion, words: usize) -> Vec<Block> {
    passage_blocks(&clean_passage(text), version, words)
}

/// Like [`reading_blocks`] for text that is already clean (from a [`BibleText`]).
pub fn passage_blocks(clean: &str, version: BibleVersion, words: usize) -> Vec<Block> {
    text_blocks(clean, version, words).0
}

/// The blocks, and the text slides (numbered from 1) that carry a paragraph
/// on from the slide before ("2,5"), for printing the reading in paragraphs.
fn text_blocks(clean: &str, version: BibleVersion, words: usize) -> (Vec<Block>, String) {
    let slides = text_slides(clean, words);
    let _ = version;
    let mut out = Vec::new();
    let mut continues = Vec::new();
    for (i, s) in slides.into_iter().enumerate() {
        if !s.new_paragraph {
            continues.push((i + 1).to_string());
        }
        let lines = s.parts.into_iter().map(|p| vec![Span::plain(p)]).collect();
        out.push(Block::Paragraph { role: None, lines });
        out.push(Block::SlideBreak);
    }
    (out, continues.join(","))
}

/// Front-matter key: the text slides that carry a paragraph on (see [`text_blocks`]).
pub const TEXT_CONTINUES: &str = "text_continues";

/// Takes the translation's initials ("(NRSVue)") off the end of a
/// reading's text, where earlier versions put them. Returns whether it did.
pub fn strip_version_note(doc: &mut LiturgyDocument) -> bool {
    let Some(Block::Paragraph { lines, .. }) =
        doc.blocks.iter_mut().rev().find(|b| matches!(b, Block::Paragraph { role: None, .. }))
    else {
        return false;
    };
    let Some(last) = lines.last_mut().and_then(|l| l.last_mut()) else { return false };
    for v in [" (NRSVue)", " (NRSV)"] {
        if let Some(rest) = last.text.strip_suffix(v) {
            last.text = rest.to_string();
            return true;
        }
    }
    false
}

/// Replaces a reading's text slides with `text` (or removes them when the
/// text is empty). The text goes after the title slide and the Gospel's
/// announcement, before the closing response ("The word of the Lord").
pub fn set_reading_text(doc: &LiturgyDocument, text: &str, version: BibleVersion, words: usize) -> LiturgyDocument {
    let (blocks, continues) = if text.trim().is_empty() {
        (Vec::new(), String::new())
    } else {
        text_blocks(&clean_passage(text), version, words)
    };
    with_reading_blocks(doc, blocks, &continues, version)
}

/// Sets a reading's text from a clean passage (see [`BibleText::passage`]).
pub fn set_reading_passage(doc: &LiturgyDocument, clean: &str, version: BibleVersion, words: usize) -> LiturgyDocument {
    let (blocks, continues) = text_blocks(clean, version, words);
    with_reading_blocks(doc, blocks, &continues, version)
}

fn with_reading_blocks(
    doc: &LiturgyDocument,
    new: Vec<Block>,
    continues: &str,
    version: BibleVersion,
) -> LiturgyDocument {
    let mut blocks: Vec<Block> =
        doc.blocks.iter().filter(|b| !matches!(b, Block::Paragraph { role: None, .. })).cloned().collect();
    // Collapse the slide breaks the old text leaves behind.
    blocks.dedup_by(|a, b| matches!(a, Block::SlideBreak) && matches!(b, Block::SlideBreak));
    while matches!(blocks.last(), Some(Block::SlideBreak)) {
        blocks.pop();
    }
    let at = blocks.iter().rposition(|b| matches!(b, Block::SlideBreak)).map(|i| i + 1).unwrap_or(0);
    let has_text = !new.is_empty();
    blocks.splice(at..at, new);
    let mut meta = doc.meta.clone();
    meta.extra.retain(|(k, _)| k != "bible" && k != TEXT_CONTINUES);
    if has_text {
        meta.extra.push(("bible".into(), version.initials().into()));
        if !continues.is_empty() {
            meta.extra.push((TEXT_CONTINUES.into(), continues.into()));
        }
    }
    LiturgyDocument { meta, blocks }
}

// ---------------------------------------------------------------------------
// A Bible text file
// ---------------------------------------------------------------------------

type Book = BTreeMap<u32, BTreeMap<u32, String>>;

/// The whole text of a Bible translation, from a JSON file the parish
/// provides (the app cannot include the NRSV or NRSVue): books, chapters and
/// verses as nested objects, optionally grouped by testament:
///
/// ```json
/// { "Old Testament": { "Genesis": { "1": { "1": "¹ When God began…", … } } }, … }
/// ```
///
/// Verse numbers at the start of a verse (`¹`, `1`) are removed. A verse
/// ending in a line break ends a paragraph; poetry's line breaks within a
/// verse are run together.
#[derive(Debug, Clone, Default)]
pub struct BibleText {
    books: HashMap<String, Book>,
}

/// "1 Kings" → "1 kings"; the names lectionaries use for books named
/// differently in Bibles.
fn book_key(name: &str) -> String {
    // "Ecclesiasticus (Sirach)" → "Ecclesiasticus".
    let name = match name.find('(') {
        Some(i) if i > 0 => &name[..i],
        _ => name,
    };
    let n = name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase().replace('.', "");
    let n = n.replace("first ", "1 ").replace("second ", "2 ").replace("third ", "3 ");
    match n.as_str() {
        "psalms" | "ps" => "psalm".into(),
        "song of solomon" | "canticle of canticles" | "song" => "song of songs".into(),
        "ecclesiasticus" | "sirach (ecclesiasticus)" => "sirach".into(),
        "wisdom" => "wisdom of solomon".into(),
        "revelation to john" | "the revelation to john" | "apocalypse" => "revelation".into(),
        "esther (greek)" | "additions to esther" => "greek esther".into(),
        _ => n,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Whole,
    /// "27a": the first part of the verse.
    A,
    /// "9b": the rest.
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VerseRef {
    chapter: u32,
    verse: u32,
    part: Part,
}

/// The verse ranges of a citation's spec ("1-3a, 14-25", "1:1—2:4a",
/// "1-4 [5-10] 11-14"). `optional`: include bracketed verses.
fn parse_ranges(spec: &str, optional: bool) -> Result<Vec<(VerseRef, VerseRef)>, String> {
    let mut s = spec.replace(['\u{2014}', '\u{2013}'], "-");
    if optional {
        // "2:[1-7] 8-20" → "2:1-7 8-20".
        s = s.replace(":[", ":").replace(":(", ":").replace(['[', ']', '(', ')'], " ");
    } else {
        let mut out = String::new();
        let mut depth = 0;
        for c in s.chars() {
            match c {
                '[' | '(' => depth += 1,
                ']' | ')' => depth -= 1,
                _ if depth == 0 => out.push(c),
                _ => {}
            }
        }
        s = out;
    }
    let mut chapter = None;
    let mut out = Vec::new();
    let bad = || format!("cannot read the verses “{}”", spec.trim());
    let point = |t: &str, chapter: &mut Option<u32>| -> Result<VerseRef, String> {
        let (ch, v) = match t.split_once(':') {
            Some((c, v)) => (Some(c.trim().parse::<u32>().map_err(|_| bad())?), v.trim()),
            None => (None, t.trim()),
        };
        if let Some(c) = ch {
            *chapter = Some(c);
        }
        let digits: String = v.chars().take_while(char::is_ascii_digit).collect();
        let verse = digits.parse::<u32>().map_err(|_| bad())?;
        let part = match &v[digits.len()..] {
            "" => Part::Whole,
            "a" => Part::A,
            "b" | "c" => Part::B,
            _ => return Err(bad()),
        };
        Ok(VerseRef { chapter: chapter.ok_or_else(bad)?, verse, part })
    };
    for seg in s.split([',', ' ', ';']).map(str::trim).filter(|t| !t.is_empty()) {
        // "2:" left by "2:[1-7] 8-20" without the optional verses.
        if let Some(c) = seg.strip_suffix(':') {
            chapter = Some(c.parse().map_err(|_| bad())?);
            continue;
        }
        let (a, b) = match seg.split_once('-') {
            Some((a, b)) => (a, b),
            None => (seg, seg),
        };
        let start = point(a, &mut chapter)?;
        let end = point(b, &mut chapter)?;
        out.push((start, end));
    }
    if out.is_empty() {
        return Err(bad());
    }
    Ok(out)
}

/// "1 Peter 3:14-18, 2 Corinthians 4:7-15" → the two citations.
fn split_books(citation: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in citation.split(',') {
        let starts_book = part.split_whitespace().take(2).any(|w| w.chars().filter(|c| c.is_alphabetic()).count() >= 2);
        match out.last_mut() {
            Some(last) if !starts_book => {
                last.push(',');
                last.push_str(part);
            }
            _ => out.push(part.trim().to_string()),
        }
    }
    out
}

/// Splits a verse into clauses at sentence and clause ends.
fn clauses(v: &str) -> Vec<String> {
    let chars: Vec<char> = v.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    for (i, &c) in chars.iter().enumerate() {
        cur.push(c);
        let end = matches!(c, '.' | '?' | '!' | ';' | ':')
            || (matches!(c, '”' | '"' | '’') && i > 0 && matches!(chars[i - 1], '.' | '?' | '!' | ','));
        let next_space = chars.get(i + 1).is_none_or(|n| n.is_whitespace());
        let closes_quote = chars.get(i + 1).is_some_and(|n| matches!(n, '”' | '"' | '’'));
        if end && next_space && !closes_quote {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out.into_iter().map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect()
}

/// A verse's words (number removed, lines run together) and whether it ends
/// a paragraph.
fn verse_text(raw: &str) -> (String, bool) {
    let body = raw.trim_start_matches(|c: char| {
        c.is_whitespace()
            || c.is_ascii_digit()
            || "\u{2070}\u{b9}\u{b2}\u{b3}\u{2074}\u{2075}\u{2076}\u{2077}\u{2078}\u{2079}".contains(c)
    });
    let trimmed = body.trim_end_matches([' ', '\t']);
    let poetry =
        trimmed.trim_end().contains('\n') || raw.trim_start_matches(|c: char| !c.is_whitespace()).starts_with("  ");
    let ends_para = trimmed.ends_with('\n') && !poetry;
    (body.split_whitespace().collect::<Vec<_>>().join(" "), ends_para)
}

impl BibleText {
    /// Reads a JSON Bible (see the type's description).
    pub fn from_json(src: &str) -> Result<BibleText, String> {
        let value: serde_json::Value = serde_json::from_str(src).map_err(|e| format!("not a Bible JSON file: {e}"))?;
        let mut bible = BibleText::default();
        fn is_book(v: &serde_json::Value) -> bool {
            v.as_object()
                .is_some_and(|chs| chs.iter().next().is_some_and(|(k, v)| k.parse::<u32>().is_ok() && v.is_object()))
        }
        fn add(bible: &mut BibleText, name: &str, v: &serde_json::Value) {
            let mut book = Book::new();
            for (ch, verses) in v.as_object().into_iter().flatten() {
                let Ok(ch) = ch.parse::<u32>() else { continue };
                let entry = book.entry(ch).or_default();
                for (n, text) in verses.as_object().into_iter().flatten() {
                    if let (Ok(n), Some(t)) = (n.parse::<u32>(), text.as_str()) {
                        entry.insert(n, t.to_string());
                    }
                }
            }
            bible.books.insert(book_key(name), book);
        }
        for (k, v) in value.as_object().ok_or("not a Bible JSON file: expected an object of books")? {
            if is_book(v) {
                add(&mut bible, k, v);
            } else if let Some(group) = v.as_object() {
                for (name, book) in group {
                    if is_book(book) {
                        add(&mut bible, name, book);
                    }
                }
            }
        }
        if bible.books.len() < 20 {
            return Err(format!("only {} books found: is this a whole Bible?", bible.books.len()));
        }
        Ok(bible)
    }

    pub fn book_count(&self) -> usize {
        self.books.len()
    }

    /// The text of a citation such as "Job 19:23-27a", "Isaiah 61:1-4,
    /// 8-11" or "Genesis 1:1—2:4a", clean and ready for
    /// [`set_reading_passage`]: paragraphs separated by blank lines, a new
    /// paragraph where verses are skipped. Bracketed (optional) verses are
    /// included when `optional`. A partial verse ("27a", "9b") is cut at
    /// the clause nearest its middle; check those by eye.
    pub fn passage(&self, citation: &str, optional: bool) -> Result<String, String> {
        // "Acts 2:1-21 (or Ezekiel 37:1-14)": the first choice.
        let citation = citation.split(" or ").next().unwrap_or(citation);
        let citation = citation.split(" (or").next().unwrap_or(citation).trim();
        // "1 Peter 3:14-18, 2 Corinthians 4:7-15": one passage after the other.
        let parts = split_books(citation);
        if parts.len() > 1 {
            let texts = parts.iter().map(|p| self.passage(p, optional)).collect::<Result<Vec<_>, _>>()?;
            return Ok(texts.join("\n\n"));
        }
        // The book: the words before the first one starting with a digit
        // (after a leading "1", "2" or "3").
        let words: Vec<&str> = citation.split_whitespace().collect();
        let at = words
            .iter()
            .enumerate()
            .position(|(i, w)| i > 0 && w.trim_start_matches(['[', '(']).starts_with(|c: char| c.is_ascii_digit()))
            .ok_or_else(|| format!("“{citation}” has no chapter and verse"))?;
        let name = words[..at].join(" ");
        let mut spec = words[at..].join(" ");
        let book =
            self.books.get(&book_key(&name)).ok_or_else(|| format!("no book called “{name}” in the Bible file"))?;
        // One-chapter books: "Philemon 1-21", "Jude 17-25".
        if !spec.contains(':') && book.len() == 1 {
            spec = format!("1:{spec}");
        }
        let mut paras: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut last: Option<(u32, u32)> = None;
        for (start, end) in parse_ranges(&spec, optional)? {
            let mut any = false;
            for (&ch, verses) in book.range(start.chapter..=end.chapter) {
                for (&n, raw) in verses {
                    if (ch, n) < (start.chapter, start.verse) || (ch, n) > (end.chapter, end.verse) {
                        continue;
                    }
                    any = true;
                    // Skipped verses start a new paragraph.
                    if last.is_some_and(|(lc, ln)| (lc, ln + 1) != (ch, n) && !(ch == lc + 1 && n == 1))
                        && !cur.is_empty()
                    {
                        paras.push(std::mem::take(&mut cur));
                    }
                    let (mut text, ends_para) = verse_text(raw);
                    let is_start = (ch, n) == (start.chapter, start.verse);
                    let is_end = (ch, n) == (end.chapter, end.verse);
                    let cs = clauses(&text);
                    let half = cs.len().div_ceil(2);
                    if cs.len() > 1 {
                        if is_end && end.part == Part::A {
                            text = cs[..half].join(" ");
                        } else if is_start && start.part == Part::B {
                            text = cs[half..].join(" ");
                        }
                    }
                    if !cur.is_empty() {
                        cur.push(' ');
                    }
                    cur.push_str(&text);
                    if ends_para && !(is_end && end.part == Part::A) {
                        paras.push(std::mem::take(&mut cur));
                    }
                    last = Some((ch, n));
                }
            }
            if !any {
                return Err(format!("{citation}: those verses are not in the Bible file"));
            }
        }
        if !cur.is_empty() {
            paras.push(cur);
        }
        // Lower-case starts after a cut ("and after my skin…") are kept as they are.
        Ok(paras.join("\n\n"))
    }
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

    const SAMPLE: &str = r#"{
      "Old Testament": {
        "Job": { "19": {
          "23": "²³ “O that my words were written down!\n    O that they were inscribed in a book!\n",
          "24": "²⁴ O that with an iron pen and with lead\n    they were engraved on a rock forever!\n",
          "25": "²⁵ For I know that my vindicator lives\n    and that in the end he will stand upon the earth;\n",
          "26": "²⁶ and after my skin has been destroyed,\n    then in my flesh I shall see God,\n",
          "27": "²⁷ whom I shall see on my side,\n    and my eyes shall behold, and not another.\n    My heart faints within me!\n"
        } },
        "Genesis": { "1": { "1": "¹ When God began to create the heavens and the earth, ", "2": "² the earth was complete chaos. \n", "3": "³ Then God said, “Let there be light,” and there was light. " },
                     "2": { "1": "¹ Thus the heavens and the earth were finished. ", "4": "⁴ These are the generations of the heavens and the earth when they were created. In the day that the Lord God made the earth and the heavens, " } }
      },
      "New Testament": { "Matthew": { "25": { "1": "¹ “Then the kingdom of heaven will be like this. ", "2": "² Five of them were foolish. \n", "3": "³ When the foolish took their lamps, ", "4": "⁴ the wise took flasks of oil. " } } }
    }"#;

    fn sample() -> BibleText {
        assert!(BibleText::from_json(SAMPLE).unwrap_err().contains("only 3 books"));
        // A whole Bible has more books; pad the sample for the test.
        let mut v: serde_json::Value = serde_json::from_str(SAMPLE).unwrap();
        for i in 0..20 {
            v["New Testament"][format!("Book{i}")] = serde_json::json!({ "1": { "1": "x" } });
        }
        BibleText::from_json(&v.to_string()).unwrap()
    }

    #[test]
    fn passages_from_a_bible_file() {
        let bible = sample();
        let job = bible.passage("Job 19:23-27a", true).unwrap();
        assert!(
            job.starts_with("“O that my words were written down! O that they were inscribed in a book! O that with"),
            "{job}"
        );
        assert!(job.ends_with("and my eyes shall behold, and not another."), "27a stops before the last clause: {job}");
        // Prose paragraphs, a skip starts a paragraph, chapters run on.
        assert_eq!(
            bible.passage("Matthew 25:1-2, 4", true).unwrap(),
            "“Then the kingdom of heaven will be like this. Five of them were foolish.\n\nthe wise took flasks of oil."
        );
        let gen = bible.passage("Genesis 1:1\u{2014}2:4a", true).unwrap();
        assert!(gen.contains("chaos.\n\nThen God said") && gen.contains("light. Thus the heavens"), "{gen}");
        assert!(gen.ends_with("when they were created."), "{gen}");
        assert_eq!(
            bible.passage("Genesis 1:1 [2]", false).unwrap(),
            "When God began to create the heavens and the earth,"
        );
        assert!(bible.passage("Acts 2:1-21 (or Job 19:23)", true).unwrap_err().contains("Acts"));
        assert!(bible.passage("Job 40:1", true).is_err());
        // The text becomes the reading's slides, with the version's initials.
        let reading: LiturgyDocument = "---\ntitle: First Reading\ntype: reading\ncitation: Job 19:23-27a\n---\n# The First Reading\n\n---\n\n[Reader] The word of the Lord.\n".parse().unwrap();
        let with = set_reading_passage(&reading, &job, BibleVersion::Nrsvue, 40);
        let text: String = with
            .blocks
            .iter()
            .filter(|b| matches!(b, Block::Paragraph { role: None, .. }))
            .map(Block::plain_text)
            .collect();
        assert!(text.starts_with("“O that my words") && !text.contains("(NRSV"), "{text}");
        assert_eq!(with.meta.extra("bible"), Some("NRSVue"));

        // Paragraphs survive the slides: a long paragraph over two slides is
        // marked as carried on; a short one after it shares a slide.
        let long = (1..=30).map(|i| format!("Sentence number {i} is here.")).collect::<Vec<_>>().join(" ");
        let text = format!("{long}\n\nA short end.");
        let slides = text_slides(&text, 40);
        assert!(slides.len() >= 2 && slides[0].new_paragraph && !slides[1].new_paragraph, "{slides:?}");
        assert_eq!(slides.last().unwrap().parts.last().unwrap(), "A short end.");
        let with = set_reading_passage(&reading, &text, BibleVersion::Nrsvue, 40);
        assert!(with.meta.extra(TEXT_CONTINUES).is_some_and(|v| v.starts_with('2')));
        // …and it all survives saving as .ewm.
        let again = crate::ewm::parse(&crate::ewm::to_string(&with)).unwrap();
        assert!(again.warnings.is_empty(), "{:?}", again.warnings);
        assert_eq!(again.document, with);
        // Replacing the text clears the marks.
        let plain = set_reading_text(&with, "", BibleVersion::Nrsvue, 40);
        assert!(plain.meta.extra(TEXT_CONTINUES).is_none() && plain.meta.extra("bible").is_none());
    }

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
        assert!(!last_text.contains("NRSV"), "no version at the end: {last_text}");
        assert_eq!(slides.last().unwrap().blocks[0].plain_text(), "The word of the Lord.");
        assert_eq!(with.meta.extra("bible"), Some("NRSVue"));
        // Replacing and removing the text leaves the responses intact.
        let again = set_reading_text(&with, "At that time Deborah judged Israel.", BibleVersion::Nrsv, 30);
        assert_eq!(again.slides().len(), 3);
        assert_eq!(again.meta.extra("bible"), Some("NRSV"));
        // Older readings lose the initials they ended with.
        let mut old = again.clone();
        if let Some(Block::Paragraph { lines, .. }) =
            old.blocks.iter_mut().rev().find(|b| matches!(b, Block::Paragraph { role: None, .. }))
        {
            lines.last_mut().unwrap().last_mut().unwrap().text.push_str(" (NRSV)");
        }
        assert!(strip_version_note(&mut old));
        assert_eq!(old, again);
        assert!(!strip_version_note(&mut old));
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
