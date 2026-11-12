//! A liturgical text pasted from a book or PDF (e.g. a parish's own copy of
//! a supplemental liturgy) turned into a library item.
//!
//! A line that starts with a speaker's name ("Celebrant", "Presider",
//! "People", "All", "Deacon" …, followed by a tab, a colon or spaces) starts
//! that speaker's part; the lines after it continue it. A blank line ends a
//! paragraph; the next unlabelled paragraph goes on with the same speaker.
//! Lines in square brackets or starting with "The people …", "Here …" and
//! the like that end without a full stop are taken as instructions. The
//! people's and everyone's parts are bold. Slides break between paragraphs
//! at about `words` words, keeping a response with the line before it.

use crate::category::ElementCategory;
use crate::document::{Block, FrontMatter, LiturgyDocument, Span};
use crate::role::Role;

/// Speaker names recognised at the start of a line.
const SPEAKERS: [&str; 14] = [
    "Celebrant and People",
    "Presider and People",
    "Celebrant",
    "Presider",
    "Officiant",
    "Priest",
    "Bishop",
    "Deacon",
    "Reader",
    "Lector",
    "Leader",
    "People",
    "Congregation",
    "All",
];

fn speaker(line: &str) -> Option<(Role, &str)> {
    let t = line.trim_start();
    for name in SPEAKERS {
        let Some(rest) = t.get(..name.len()).filter(|h| h.eq_ignore_ascii_case(name)).map(|_| &t[name.len()..]) else {
            continue;
        };
        // "People\t…", "People: …", "People   …", or the name alone on a line.
        let rest2 = rest.trim_start_matches([':', '.']);
        let sep = rest.is_empty()
            || rest.starts_with(['\t', ':'])
            || rest.starts_with("  ")
            || (rest.starts_with(' ') && rest2.trim_start().starts_with(|c: char| c.is_uppercase()));
        if !sep {
            continue;
        }
        let role = match name {
            "Celebrant and People" | "Presider and People" | "All" => Role::All,
            "Congregation" => Role::People,
            // Supplemental rites say "Presider"; the parish's items say Celebrant.
            "Presider" => Role::Celebrant,
            other => Role::parse(other),
        };
        return Some((role, rest2.trim()));
    }
    None
}

fn is_instruction(line: &str) -> bool {
    let t = line.trim();
    if t.starts_with('[') && t.ends_with(']') || t.starts_with('(') && t.ends_with(')') {
        return true;
    }
    let lower = t.to_lowercase();
    let starts = [
        "the people",
        "the celebrant",
        "the presider",
        "here ",
        "then ",
        "all stand",
        "all kneel",
        "silence",
        "or this",
        "the deacon",
        "facing the",
    ];
    starts.iter().any(|s| lower.starts_with(s)) && !t.ends_with(['.', '?', '!', ';', ','])
}

/// A pasted text as a library item.
pub fn document_from_text(
    title: &str,
    category: ElementCategory,
    citation: Option<&str>,
    text: &str,
    words: usize,
) -> LiturgyDocument {
    // Paragraphs: (role, lines).
    let mut paras: Vec<(Option<Role>, Vec<String>)> = Vec::new();
    let mut current: Option<Role> = None;
    let mut open = false;
    for raw in text.replace("\r\n", "\n").replace('\u{a0}', " ").lines() {
        let line = raw.trim_end();
        if line.trim().is_empty() {
            open = false;
            continue;
        }
        if is_instruction(line) {
            paras.push((Some(Role::Rubric), vec![line.trim().trim_matches(['[', ']', '(', ')']).to_string()]));
            open = false;
            continue;
        }
        if let Some((role, rest)) = speaker(line) {
            current = Some(role.clone());
            paras.push((Some(role), if rest.is_empty() { Vec::new() } else { vec![rest.to_string()] }));
            open = true;
            continue;
        }
        let l = line.trim().to_string();
        match paras.last_mut() {
            Some((r, lines)) if open && r.as_ref() != Some(&Role::Rubric) => lines.push(l),
            _ => {
                paras.push((current.clone(), vec![l]));
                open = true;
            }
        }
    }
    paras.retain(|(_, l)| !l.is_empty());

    let count = |l: &[String]| l.iter().map(|x| x.split_whitespace().count()).sum::<usize>();
    let mut blocks = Vec::new();
    let mut slide_words = 0;
    for (i, (role, lines)) in paras.iter().enumerate() {
        let n = count(lines);
        let response = role.as_ref().is_some_and(Role::is_congregational) && n <= 14;
        let after_rubric = i > 0 && paras[i - 1].0 == Some(Role::Rubric);
        if slide_words > 0 && slide_words + n > words && !response && !after_rubric {
            blocks.push(Block::SlideBreak);
            slide_words = 0;
        }
        slide_words += n;
        let bold = role.as_ref().is_some_and(Role::is_congregational);
        let block = match role {
            Some(Role::Rubric) => {
                Block::Paragraph { role: Some(Role::Rubric), lines: vec![vec![Span::italic(lines.join(" "))]] }
            }
            _ => Block::Paragraph {
                role: role.clone(),
                lines: lines
                    .iter()
                    .map(|l| vec![if bold { Span::bold(l.clone()) } else { Span::plain(l.clone()) }])
                    .collect(),
            },
        };
        blocks.push(block);
    }
    LiturgyDocument {
        meta: FrontMatter {
            title: title.trim().to_string(),
            category: Some(category),
            citation: citation.map(str::trim).filter(|c| !c.is_empty()).map(String::from),
            ..Default::default()
        },
        blocks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speakers_responses_and_instructions() {
        let text = "The people remain standing. The Presider faces them and sings or says\n\
Presider\tThe Lord be with you.\n\
People\tAnd also with you.\n\
Presider: Lift up your hearts.\n\
People: We lift them to the Lord.\n\
\n\
Presider  It is right, and a good and joyful thing,\n\
always and everywhere to give thanks to you.\n\
\n\
Glory and honor and praise to you, holy and living God.\n\
\n\
Presider and People\n\
Holy, holy, holy Lord, God of power and might,\n\
heaven and earth are full of your glory.\n";
        let doc = document_from_text(
            "Eucharistic Prayer 1",
            ElementCategory::EucharisticPrayer,
            Some("EOW 1, p. 57"),
            text,
            80,
        );
        let paras: Vec<(Option<String>, String, bool)> = doc
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph { role, lines } => {
                    Some((role.as_ref().map(|r| r.label().to_string()), b.plain_text(), lines[0][0].bold))
                }
                _ => None,
            })
            .collect();
        assert_eq!(paras[0].0.as_deref(), Some("Rubric"));
        assert_eq!(paras[1], (Some("Celebrant".into()), "The Lord be with you.".into(), false));
        assert_eq!(paras[2], (Some("People".into()), "And also with you.".into(), true));
        assert_eq!(paras[3].1, "Lift up your hearts.");
        assert_eq!(
            paras[5].1,
            "It is right, and a good and joyful thing,\nalways and everywhere to give thanks to you."
        );
        // An unlabelled paragraph goes on with the same speaker.
        assert_eq!(paras[6].0.as_deref(), Some("Celebrant"));
        // Said together: All, bold, on its own lines.
        assert_eq!(paras[7].0.as_deref(), Some("All"));
        assert!(paras[7].2 && paras[7].1.starts_with("Holy, holy, holy"));
        assert_eq!(doc.meta.citation.as_deref(), Some("EOW 1, p. 57"));
        // It saves and reloads as .ewm without warnings.
        let again = crate::ewm::parse(&crate::ewm::to_string(&doc)).unwrap();
        assert!(again.warnings.is_empty(), "{:?}", again.warnings);
        assert_eq!(again.document.blocks.len(), doc.blocks.len());
        // Slides break at about 80 words, never right before a short response.
        let long = (0..40)
            .map(|i| format!("Presider\tLine {i} has a few more words in it.\nPeople\tAmen.\n"))
            .collect::<String>();
        let doc = document_from_text("x", ElementCategory::Prayer, None, &long, 40);
        let mut prev: Option<&Block> = None;
        for b in &doc.blocks {
            if let (Some(Block::SlideBreak), Block::Paragraph { role: Some(Role::People), .. }) = (prev, b) {
                panic!("a response starts a slide");
            }
            prev = Some(b);
        }
        assert!(doc.blocks.iter().filter(|b| matches!(b, Block::SlideBreak)).count() >= 5);
    }

    #[test]
    fn plain_words_are_not_speakers() {
        assert!(speaker("All glory be to thee, Almighty God").is_none());
        assert!(speaker("People of God, let us pray.").is_none());
        assert_eq!(
            speaker("Celebrant The Lord be with you.").map(|(r, t)| (r, t.to_string())),
            Some((Role::Celebrant, "The Lord be with you.".into()))
        );
        assert!(speaker("Peoples of the earth").is_none());
        assert!(speaker("Deacon\tGo in peace.").is_some());
    }
}
