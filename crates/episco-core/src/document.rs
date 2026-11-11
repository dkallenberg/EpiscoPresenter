//! The in-memory representation of a parsed `.ewm` liturgy file.

use serde::{Deserialize, Serialize};

use crate::category::ElementCategory;
use crate::role::Role;
use crate::season::{LiturgicalColor, Season};

/// A complete liturgy item: front matter plus an ordered list of blocks.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LiturgyDocument {
    pub meta: FrontMatter,
    pub blocks: Vec<Block>,
}

/// The YAML-style header between `---` fences.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FrontMatter {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<Season>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<LiturgicalColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_background: Option<String>,
    /// Element category, written as `type:` in the file.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub category: Option<ElementCategory>,
    /// Scripture citation, e.g. `Joshua 24:1-3a`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
    /// Hymnal number, e.g. `436` or `S 125`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hymn_number: Option<String>,
    /// Hymn tune name or secondary caption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tune: Option<String>,
    /// Unrecognised keys, preserved in file order so files round-trip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<(String, String)>,
}

impl FrontMatter {
    /// Explicit colour, else the season's customary colour.
    pub fn effective_color(&self) -> Option<LiturgicalColor> {
        self.color.or_else(|| self.season.map(Season::default_color))
    }

    /// Explicit background, else the stained-glass image for the effective colour.
    pub fn effective_background(&self) -> Option<String> {
        self.default_background.clone().or_else(|| self.effective_color().map(LiturgicalColor::default_background))
    }

    pub fn effective_category(&self) -> ElementCategory {
        self.category.unwrap_or_default()
    }

    /// Text for a lower-third info badge, e.g. `Hymn 436: Lift Up Your Heads`
    /// or `First Reading: Joshua 24:1-3a`.
    pub fn lower_third_caption(&self) -> String {
        match (self.effective_category(), &self.hymn_number, &self.citation) {
            (ElementCategory::Hymn, Some(n), _) if self.title.is_empty() => format!("Hymn {n}"),
            (ElementCategory::Hymn, Some(n), _) => format!("Hymn {n}: {}", self.title),
            (_, _, Some(c)) if self.title.is_empty() => c.clone(),
            (_, _, Some(c)) if self.title.contains(c.as_str()) => self.title.clone(),
            (_, _, Some(c)) => format!("{}: {c}", self.title),
            _ => self.title.clone(),
        }
    }

    /// Looks up an unrecognised key.
    pub fn extra(&self, key: &str) -> Option<&str> {
        self.extra.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

/// One structural element of a liturgy body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    /// A speaker block (`[People] ...`) or, with `role: None`, untagged text.
    /// Each entry of `lines` is one source line; line breaks are significant.
    Paragraph {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<Role>,
        lines: Vec<Line>,
    },
    /// `## Heading` — a section title; also begins a new slide.
    Heading { level: u8, text: String },
    /// `![alt](src)` — a sheet-music snippet or other image.
    Image { alt: String, src: String },
    /// A lone `---` in the body — an explicit slide break.
    SlideBreak,
}

impl Block {
    /// Convenience constructor for a speaker block from plain-text lines.
    pub fn speech<I, S>(role: Role, lines: I) -> Block
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Block::Paragraph { role: Some(role), lines: lines.into_iter().map(|l| vec![Span::plain(l)]).collect() }
    }

    pub fn role(&self) -> Option<&Role> {
        match self {
            Block::Paragraph { role, .. } => role.as_ref(),
            _ => None,
        }
    }

    /// The block's text with formatting removed and lines joined by `\n`.
    pub fn plain_text(&self) -> String {
        match self {
            Block::Paragraph { lines, .. } => lines.iter().map(|l| line_plain_text(l)).collect::<Vec<_>>().join("\n"),
            Block::Heading { text, .. } => text.clone(),
            Block::Image { alt, .. } => alt.clone(),
            Block::SlideBreak => String::new(),
        }
    }
}

/// A single line of formatted text.
pub type Line = Vec<Span>;

/// A run of text with uniform formatting.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Span {
    pub fn plain(text: impl Into<String>) -> Span {
        Span { text: text.into(), bold: false, italic: false }
    }

    pub fn bold(text: impl Into<String>) -> Span {
        Span { text: text.into(), bold: true, italic: false }
    }

    pub fn italic(text: impl Into<String>) -> Span {
        Span { text: text.into(), bold: false, italic: true }
    }
}

pub fn line_plain_text(line: &[Span]) -> String {
    line.iter().map(|s| s.text.as_str()).collect()
}

/// One projected slide: a contiguous run of blocks between slide breaks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slide {
    /// Nearest preceding heading, carried onto each slide of a section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,
    pub blocks: Vec<Block>,
}

impl LiturgyDocument {
    /// Splits the body into slides.
    ///
    /// A new slide begins at every explicit `---` break and at every heading.
    /// Headings are not repeated as blocks; they become the slide's `heading`
    /// (and carry over to following slides until the next heading).
    /// Slides with no content are dropped.
    pub fn slides(&self) -> Vec<Slide> {
        let mut slides = Vec::new();
        let mut heading: Option<String> = None;
        let mut current: Vec<Block> = Vec::new();

        let flush = |slides: &mut Vec<Slide>, current: &mut Vec<Block>, heading: &Option<String>| {
            if !current.is_empty() {
                slides.push(Slide { heading: heading.clone(), blocks: std::mem::take(current) });
            }
        };

        for block in &self.blocks {
            match block {
                Block::SlideBreak => flush(&mut slides, &mut current, &heading),
                Block::Heading { text, .. } => {
                    flush(&mut slides, &mut current, &heading);
                    heading = Some(text.clone());
                }
                other => current.push(other.clone()),
            }
        }
        flush(&mut slides, &mut current, &heading);
        slides
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_color_and_background_fall_back_to_season() {
        let meta = FrontMatter { season: Some(Season::Lent), ..Default::default() };
        assert_eq!(meta.effective_color(), Some(LiturgicalColor::Violet));
        assert_eq!(meta.effective_background().as_deref(), Some("violet_stained_glass.png"));
    }

    #[test]
    fn lower_third_captions() {
        let hymn = FrontMatter {
            title: "Lift Up Your Heads".into(),
            category: Some(ElementCategory::Hymn),
            hymn_number: Some("436".into()),
            ..Default::default()
        };
        assert_eq!(hymn.lower_third_caption(), "Hymn 436: Lift Up Your Heads");

        let reading = FrontMatter {
            title: "First Reading".into(),
            category: Some(ElementCategory::Reading),
            citation: Some("Joshua 24:1-3a".into()),
            ..Default::default()
        };
        assert_eq!(reading.lower_third_caption(), "First Reading: Joshua 24:1-3a");

        let psalm =
            FrontMatter { title: "Psalm 78:1-7".into(), citation: Some("Psalm 78:1-7".into()), ..Default::default() };
        assert_eq!(psalm.lower_third_caption(), "Psalm 78:1-7");
    }

    #[test]
    fn slides_split_on_breaks_and_headings() {
        let doc = LiturgyDocument {
            meta: FrontMatter::default(),
            blocks: vec![
                Block::speech(Role::Priest, ["A"]),
                Block::SlideBreak,
                Block::SlideBreak,
                Block::Heading { level: 2, text: "Sanctus".into() },
                Block::speech(Role::People, ["B"]),
                Block::SlideBreak,
                Block::speech(Role::People, ["C"]),
            ],
        };
        let slides = doc.slides();
        assert_eq!(slides.len(), 3);
        assert_eq!(slides[0].heading, None);
        assert_eq!(slides[1].heading.as_deref(), Some("Sanctus"));
        assert_eq!(slides[2].heading.as_deref(), Some("Sanctus"));
        assert_eq!(slides[2].blocks[0].plain_text(), "C");
    }
}
