//! Hymns from the parish's own licensed sources.
//!
//! Hymn texts (the Hymnal 1982 and most hymnals) are largely under copyright,
//! so the app ships none. The operator pastes a hymn's words, or imports a
//! folder of text files, from a source the parish is licensed to project
//! (e.g. a OneLicense or CCLI reprint licence), and [`hymn_blocks`] lays them
//! out: one stanza per slide, verse numbers as labels, a refrain where marked.

use crate::document::{Block, LiturgyDocument, Span};
use crate::role::Role;

/// How a pasted hymn is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HymnOptions {
    /// Repeat the refrain after every stanza (else only where it was pasted).
    #[serde(default)]
    pub refrain_after_each: bool,
    /// Split stanzas longer than this many lines over two slides (0: never).
    #[serde(default = "eight")]
    pub max_lines: usize,
}

fn eight() -> usize {
    8
}

impl Default for HymnOptions {
    fn default() -> Self {
        HymnOptions { refrain_after_each: false, max_lines: 8 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Stanza {
    /// Verse number, or `None` for a refrain / unnumbered stanza.
    number: Option<String>,
    refrain: bool,
    lines: Vec<String>,
}

fn is_refrain_label(line: &str) -> Option<&str> {
    let l = line.trim();
    for label in ["Refrain:", "Refrain", "Chorus:", "Chorus", "Ref.", "R."] {
        if let Some(rest) = l.strip_prefix(label) {
            if rest.is_empty() || rest.starts_with([' ', '\t']) || label.ends_with([':', '.']) {
                return Some(rest.trim());
            }
        }
    }
    None
}

/// A leading verse number: "1 ", "1. ", "1) ", "2\t".
fn verse_number(line: &str) -> Option<(String, &str)> {
    let t = line.trim_start();
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > 2 {
        return None;
    }
    let rest = &t[digits.len()..];
    let rest = rest.strip_prefix(['.', ')']).unwrap_or(rest);
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some((digits, rest.trim_start()))
}

/// Lines that come along when words are copied from a hymnal site or PDF.
fn is_noise(line: &str) -> bool {
    let l = line.trim().to_lowercase();
    l.starts_with("words:")
        || l.starts_with("music:")
        || l.starts_with("text:")
        || l.starts_with("tune:")
        || l.contains("copyright")
        || l.contains("©")
        || l.starts_with("reprinted")
        || l.starts_with("onelicense")
        || l.starts_with("ccli")
}

fn stanzas(text: &str) -> (Vec<Stanza>, Vec<(String, String)>) {
    let mut out: Vec<Stanza> = Vec::new();
    let mut credits = Vec::new();
    let mut cur: Option<Stanza> = None;
    let push = |cur: &mut Option<Stanza>, out: &mut Vec<Stanza>| {
        if let Some(s) = cur.take() {
            if !s.lines.is_empty() {
                out.push(s);
            }
        }
    };
    for raw in text.lines() {
        let line = raw.replace('\u{a0}', " ");
        let t = line.trim();
        if t.is_empty() {
            push(&mut cur, &mut out);
            continue;
        }
        if is_noise(t) {
            let lower = t.to_lowercase();
            for key in ["words", "music", "text", "tune"] {
                if let Some(v) = lower.strip_prefix(&format!("{key}:")).map(|_| t[key.len() + 1..].trim()) {
                    let key = if key == "text" { "words" } else { key };
                    credits.push((key.to_string(), v.to_string()));
                }
            }
            if lower.contains("copyright") || lower.contains('©') || lower.starts_with("reprinted") {
                credits.push(("license".into(), t.to_string()));
            }
            continue;
        }
        if let Some(rest) = is_refrain_label(t) {
            push(&mut cur, &mut out);
            cur = Some(Stanza { number: None, refrain: true, lines: Vec::new() });
            if !rest.is_empty() {
                cur.as_mut().unwrap().lines.push(rest.to_string());
            }
            continue;
        }
        if let Some((n, rest)) = verse_number(t) {
            push(&mut cur, &mut out);
            cur = Some(Stanza { number: Some(n), refrain: false, lines: vec![rest.to_string()] });
            continue;
        }
        cur.get_or_insert(Stanza { number: None, refrain: false, lines: Vec::new() }).lines.push(t.to_string());
    }
    push(&mut cur, &mut out);
    // Unnumbered stanzas of a numbered hymn are refrains when they repeat.
    (out, credits)
}

/// The hymn's body: one stanza per slide (long ones split), verse numbers as
/// labels, refrains untagged and repeated if asked. Also returns credits
/// found in the text (words, music, licence notice).
pub fn hymn_blocks(text: &str, opts: &HymnOptions) -> (Vec<Block>, Vec<(String, String)>) {
    let (stanzas, credits) = stanzas(text);
    let refrain = stanzas.iter().find(|s| s.refrain).cloned();
    let mut seq: Vec<Stanza> = Vec::new();
    for s in &stanzas {
        if opts.refrain_after_each && s.refrain && seq.iter().any(|x| x.refrain) {
            continue; // repeated below instead
        }
        seq.push(s.clone());
        if opts.refrain_after_each && !s.refrain {
            if let Some(r) = &refrain {
                seq.push(r.clone());
            }
        }
    }
    let mut blocks = Vec::new();
    for s in seq {
        let chunks: Vec<Vec<String>> = if opts.max_lines > 0 && s.lines.len() > opts.max_lines {
            let half = s.lines.len().div_ceil(2);
            vec![s.lines[..half].to_vec(), s.lines[half..].to_vec()]
        } else {
            vec![s.lines.clone()]
        };
        for (k, lines) in chunks.into_iter().enumerate() {
            if !blocks.is_empty() {
                blocks.push(Block::SlideBreak);
            }
            let role = match (&s.number, s.refrain) {
                (Some(n), _) if k == 0 => Some(Role::Custom(n.clone())),
                (_, true) if k == 0 => Some(Role::Custom("Refrain".into())),
                _ => None,
            };
            let lines =
                lines.into_iter().map(|l| vec![if s.refrain { Span::italic(l) } else { Span::plain(l) }]).collect();
            blocks.push(Block::Paragraph { role, lines });
        }
    }
    (blocks, credits)
}

/// A hymn document from pasted words.
pub fn hymn_document(title: &str, number: Option<&str>, text: &str, opts: &HymnOptions) -> LiturgyDocument {
    let mut doc = LiturgyDocument::default();
    doc.meta.title = title.to_string();
    doc.meta.category = Some(crate::ElementCategory::Hymn);
    doc.meta.hymn_number = number.map(String::from).filter(|n| !n.is_empty());
    let (blocks, credits) = hymn_blocks(text, opts);
    doc.blocks = blocks;
    for (k, v) in credits {
        if !doc.meta.extra.iter().any(|(x, _)| *x == k) {
            doc.meta.extra.push((k, v));
        }
    }
    doc
}

/// Replaces a hymn document's words, keeping its title, number, looks and
/// credits (credits found in the pasted text fill only those not set).
pub fn set_hymn_text(doc: &LiturgyDocument, text: &str, opts: &HymnOptions) -> LiturgyDocument {
    let (blocks, credits) = hymn_blocks(text, opts);
    let mut meta = doc.meta.clone();
    meta.category = Some(crate::ElementCategory::Hymn);
    for (k, v) in credits {
        if meta.extra(&k).is_none() {
            meta.extra.push((k, v));
        }
    }
    LiturgyDocument { meta, blocks }
}

/// "436 Lift Up Your Heads.txt" → (Some("436"), "Lift Up Your Heads");
/// "S 125 Gloria.txt" → (Some("S 125"), "Gloria"); "Amazing grace.txt" → (None, …).
pub fn split_file_name(stem: &str) -> (Option<String>, String) {
    let s = stem.trim();
    let (prefix, rest) = match s.strip_prefix("S ").or_else(|| s.strip_prefix("S-")).or_else(|| s.strip_prefix("S")) {
        Some(r) if r.starts_with(|c: char| c.is_ascii_digit()) => ("S ", r),
        _ => ("", s),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return (None, s.to_string());
    }
    let title = rest[digits.len()..].trim_start_matches([' ', '-', '_', '.', '–']).trim().to_string();
    (Some(format!("{prefix}{digits}")), if title.is_empty() { format!("Hymn {prefix}{digits}") } else { title })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Placeholder words (not a real hymn) in the shapes hymnal copies take.
    const PASTED: &str = "1 First line of stanza one,
second line of stanza one;
third line of stanza one,
fourth line of stanza one.

Refrain:
A refrain line,
another refrain line.

2. First line of stanza two,
second line of stanza two.

3) Third stanza here,
and its last line.

Words: A. Writer (1800-1870)
Music: Tune Name, A. Composer
Reprinted under OneLicense #A-000000. All rights reserved.";

    #[test]
    fn lays_out_stanzas_and_refrain() {
        let (blocks, credits) = hymn_blocks(PASTED, &HymnOptions::default());
        let doc = LiturgyDocument { meta: Default::default(), blocks };
        let slides = doc.slides();
        assert_eq!(slides.len(), 4);
        assert_eq!(slides[0].blocks[0].role(), Some(&Role::Custom("1".into())));
        assert_eq!(slides[1].blocks[0].role(), Some(&Role::Custom("Refrain".into())));
        assert_eq!(slides[2].blocks[0].plain_text(), "First line of stanza two,\nsecond line of stanza two.");
        assert_eq!(slides[3].blocks[0].role(), Some(&Role::Custom("3".into())));
        assert_eq!(credits[0], ("words".into(), "A. Writer (1800-1870)".into()));
        assert_eq!(credits[1].0, "music");
        assert_eq!(credits[2].0, "license");

        // The refrain after every stanza.
        let (blocks, _) = hymn_blocks(PASTED, &HymnOptions { refrain_after_each: true, max_lines: 8 });
        let doc = LiturgyDocument { meta: Default::default(), blocks };
        let roles: Vec<String> = doc
            .slides()
            .iter()
            .map(|s| s.blocks[0].role().map(|r| r.label().to_string()).unwrap_or_default())
            .collect();
        assert_eq!(roles, vec!["1", "Refrain", "2", "Refrain", "3", "Refrain"]);
    }

    #[test]
    fn long_stanzas_split_and_documents_round_trip() {
        let long = (1..=10).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let (blocks, _) = hymn_blocks(&format!("1 {long}"), &HymnOptions::default());
        let doc = LiturgyDocument { meta: Default::default(), blocks };
        assert_eq!(doc.slides().len(), 2);

        let doc = hymn_document("Placeholder Hymn", Some("999"), PASTED, &HymnOptions::default());
        assert_eq!(doc.meta.hymn_number.as_deref(), Some("999"));
        assert_eq!(doc.meta.extra("music"), Some("Tune Name, A. Composer"));
        let back = crate::ewm::parse(&crate::ewm::to_string(&doc)).unwrap();
        assert!(back.warnings.is_empty(), "{:?}", back.warnings);
        assert_eq!(back.document, doc);
    }

    #[test]
    fn file_names() {
        assert_eq!(split_file_name("436 Lift Up Your Heads"), (Some("436".into()), "Lift Up Your Heads".into()));
        assert_eq!(split_file_name("S 125 - Gloria"), (Some("S 125".into()), "Gloria".into()));
        assert_eq!(split_file_name("S125"), (Some("S 125".into()), "Hymn S 125".into()));
        assert_eq!(split_file_name("Saint Patrick"), (None, "Saint Patrick".into()));
        assert_eq!(split_file_name("Evening Hymn"), (None, "Evening Hymn".into()));
    }
}
