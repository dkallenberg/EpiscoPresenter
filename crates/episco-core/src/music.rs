//! Music named in the service: the prelude (on the stream's welcome slide
//! and at the head of the bulletin) and the choir's anthems (an item with a
//! lower third on the stream).

use serde::{Deserialize, Serialize};

use crate::document::{Block, LiturgyDocument, Span};
use crate::role::Role;
use crate::ElementCategory;

/// A piece of music: its title and who wrote it ("Johann Sebastian Bach,
/// arr. by Homer Whitfield").
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Music {
    #[serde(default)]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub composer: String,
}

impl Music {
    pub fn new(title: &str, composer: &str) -> Music {
        Music { title: title.trim().to_string(), composer: composer.trim().to_string() }
    }

    pub fn is_empty(&self) -> bool {
        self.title.trim().is_empty() && self.composer.trim().is_empty()
    }
}

/// Front matter: who wrote an anthem.
pub const COMPOSER: &str = "composer";

/// An anthem's document: its title (italic) and composer on one slide,
/// shown as a lower third on the stream. The title is the piece's own; the
/// item keeps its place in the service ("Offertory Anthem") as its name.
pub fn anthem_document(title: &str, composer: &str) -> LiturgyDocument {
    let mut doc = LiturgyDocument::default();
    doc.meta.category = Some(ElementCategory::Anthem);
    set_anthem(&doc, title, composer)
}

/// Gives an anthem document a new title and composer.
pub fn set_anthem(doc: &LiturgyDocument, title: &str, composer: &str) -> LiturgyDocument {
    let (title, composer) = (title.trim(), composer.trim());
    let mut doc = doc.clone();
    doc.meta.category = Some(ElementCategory::Anthem);
    doc.meta.title = title.to_string();
    doc.meta.extra.retain(|(k, _)| k != COMPOSER);
    if !composer.is_empty() {
        doc.meta.extra.push((COMPOSER.into(), composer.into()));
    }
    let mut line = Vec::new();
    if !title.is_empty() {
        line.push(Span::italic(title));
    }
    if !composer.is_empty() {
        line.push(Span::plain(if title.is_empty() { composer.to_string() } else { format!(", {composer}") }));
    }
    doc.blocks = if line.is_empty() {
        vec![Block::Paragraph {
            role: Some(Role::Rubric),
            lines: vec![vec![Span::italic("Name the anthem: its title and composer.")]],
        }]
    } else {
        vec![Block::Paragraph { role: None, lines: vec![line] }]
    };
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_anthem_shows_its_title_and_composer() {
        let doc = anthem_document("Ave verum corpus", "W. A. Mozart");
        assert_eq!(doc.meta.title, "Ave verum corpus");
        assert_eq!(doc.meta.extra(COMPOSER), Some("W. A. Mozart"));
        assert_eq!(doc.blocks[0].plain_text(), "Ave verum corpus, W. A. Mozart");
        let again = set_anthem(&doc, "If ye love me", "");
        assert_eq!(again.meta.extra(COMPOSER), None);
        assert_eq!(again.blocks[0].plain_text(), "If ye love me");
        assert!(set_anthem(&doc, "", "").blocks[0].role().is_some_and(|r| r.is_rubric()));
    }
}
