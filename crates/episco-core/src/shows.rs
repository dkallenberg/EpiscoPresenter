//! Music that carries its own words: where the music is shown and where
//! the words are.
//!
//! A picture of music marked `words` (`![](acclamation.png){words}`) has the
//! words printed in it, and its words also follow it as text on its slide.
//! By the usual rule the bulletin and the script print the music (not the
//! words again), the sanctuary shows the words (the people read them), and
//! the live stream shows both. An item can change that for itself with
//! `music_on` and `words_on` in its front matter, each naming the places
//! (`music_on: bulletin, script, sanctuary, stream`; `none` for nowhere).
//!
//! The words a picture carries are the text after it up to the next
//! heading, slide break or picture. Rubrics are always kept.

use crate::document::{Block, FrontMatter, LiturgyDocument};

/// Where an item appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Bulletin,
    Script,
    Sanctuary,
    Stream,
}

impl Place {
    pub const ALL: [Place; 4] = [Place::Bulletin, Place::Script, Place::Sanctuary, Place::Stream];

    pub fn key(self) -> &'static str {
        match self {
            Place::Bulletin => "bulletin",
            Place::Script => "script",
            Place::Sanctuary => "sanctuary",
            Place::Stream => "stream",
        }
    }

    pub fn parse(s: &str) -> Option<Place> {
        Place::ALL.into_iter().find(|p| p.key().eq_ignore_ascii_case(s.trim()))
    }
}

impl From<crate::look::Output> for Place {
    fn from(o: crate::look::Output) -> Place {
        match o {
            crate::look::Output::Sanctuary => Place::Sanctuary,
            crate::look::Output::Stream => Place::Stream,
        }
    }
}

/// The front matter keys naming where the music and the words are shown.
pub const MUSIC_ON: &str = "music_on";
pub const WORDS_ON: &str = "words_on";

/// Where the music is shown by the usual rule.
pub fn music_default(place: Place) -> bool {
    place != Place::Sanctuary
}

/// Where the words are shown by the usual rule.
pub fn words_default(place: Place) -> bool {
    matches!(place, Place::Sanctuary | Place::Stream)
}

fn listed(meta: &FrontMatter, key: &str, place: Place) -> Option<bool> {
    let v = meta.extra(key)?;
    Some(v.split(',').filter_map(Place::parse).any(|p| p == place))
}

/// Whether the item's music (with words) is shown at `place`.
pub fn music_on(meta: &FrontMatter, place: Place) -> bool {
    listed(meta, MUSIC_ON, place).unwrap_or_else(|| music_default(place))
}

/// Whether the words its music carries are shown at `place`.
pub fn words_on(meta: &FrontMatter, place: Place) -> bool {
    listed(meta, WORDS_ON, place).unwrap_or_else(|| words_default(place))
}

/// Whether the item has music carrying its own words.
pub fn has_music_with_words(doc: &LiturgyDocument) -> bool {
    doc.blocks.iter().any(|b| matches!(b, Block::Image { words: true, .. }))
}

/// For each of the document's blocks, whether it is shown at `place`.
pub fn shown(doc: &LiturgyDocument, place: Place) -> Vec<bool> {
    let music = music_on(&doc.meta, place);
    let words = words_on(&doc.meta, place);
    let mut under = false;
    doc.blocks
        .iter()
        .map(|b| match b {
            Block::Image { words: w, .. } => {
                under = *w;
                !*w || music
            }
            Block::Heading { .. } | Block::SlideBreak | Block::OutputBreak { .. } => {
                under = false;
                true
            }
            Block::Paragraph { role, .. } => !under || words || role.as_ref().is_some_and(|r| r.is_rubric()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCLAMATION: &str = "---\ntitle: Opening Acclamation\ntype: acclamation\n---\n\n![](a.png){words}\n\n[Celebrant] Blessed be God: Father, Son, and Holy Spirit.\n[People] **And blessed be his kingdom, now and for ever. Amen.**\n\n---\n\n[Celebrant] The Lord be with you.\n";

    #[test]
    fn music_with_its_words_by_the_usual_rule() {
        let doc: LiturgyDocument = ACCLAMATION.parse().unwrap();
        assert!(has_music_with_words(&doc));
        assert!(matches!(&doc.blocks[0], Block::Image { words: true, fit: None, .. }));
        // Image, Celebrant, People, break, Celebrant.
        assert_eq!(shown(&doc, Place::Bulletin), [true, false, false, true, true]);
        assert_eq!(shown(&doc, Place::Script), [true, false, false, true, true]);
        assert_eq!(shown(&doc, Place::Sanctuary), [false, true, true, true, true]);
        assert_eq!(shown(&doc, Place::Stream), [true, true, true, true, true]);
        // Written back as it was read.
        assert!(crate::ewm::to_string(&doc).contains("![](a.png){words}\n"));
    }

    #[test]
    fn an_item_can_change_the_rule() {
        let text = ACCLAMATION.replace(
            "type: acclamation\n",
            "type: acclamation\nmusic_on: bulletin, stream\nwords_on: script, sanctuary\n",
        );
        let doc: LiturgyDocument = text.parse().unwrap();
        assert_eq!(shown(&doc, Place::Script), [false, true, true, true, true]);
        assert_eq!(shown(&doc, Place::Stream), [true, false, false, true, true]);
        let none: LiturgyDocument =
            ACCLAMATION.replace("type: acclamation\n", "type: acclamation\nwords_on: none\n").parse().unwrap();
        assert_eq!(shown(&none, Place::Sanctuary), [false, false, false, true, true]);
    }

    #[test]
    fn plain_music_is_shown_everywhere() {
        let doc: LiturgyDocument = ACCLAMATION.replace("{words}", "{width}").parse().unwrap();
        assert!(!has_music_with_words(&doc));
        for p in Place::ALL {
            assert!(shown(&doc, p).iter().all(|x| *x));
        }
    }
}
