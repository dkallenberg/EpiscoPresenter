//! The in-memory representation of a parsed `.ewm` liturgy file.

use serde::{Deserialize, Serialize};

use crate::category::ElementCategory;
use crate::look::Output;
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

    /// The Book of Common Prayer's own text (a built-in item, not a parish
    /// version of it): it cannot be changed; a change is made in a dated
    /// parish version (`based_on` names the item; `created` marks a version).
    pub fn is_prayer_book_text(&self) -> bool {
        self.extra("based_on").is_some() && self.extra("created").is_none()
    }

    /// Slides made in the app (the slide maker, imported pages): shown as
    /// written, each in its own layout.
    pub fn is_made_slides(&self) -> bool {
        self.extra(SLIDE_MAKER) == Some("true")
    }

    /// Something only for the screens (a picture or video, a planned blank,
    /// made slides, a web video), not printed in the bulletin.
    pub fn screen_only(&self) -> bool {
        ["media", "blank", WEB_VIDEO].iter().any(|k| self.extra(k).is_some()) || self.is_made_slides()
    }
}

/// Front matter: slides made in the app (`slide_maker: true`).
pub const SLIDE_MAKER: &str = "slide_maker";
/// Front matter: a YouTube, Facebook or Instagram video shown full screen.
pub const WEB_VIDEO: &str = "web_video";

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
    /// `![alt](src)` — a sheet-music snippet or other image. `fit` (written
    /// `![alt](src){width}`) sizes a picture on a made slide: `center`
    /// (as it is, at most the box), `width` / `height` (the box's width or
    /// height) or `full` (the whole screen).
    Image {
        alt: String,
        src: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fit: Option<String>,
    },
    /// A lone `---` in the body — an explicit slide break.
    SlideBreak,
    /// `--- stream` or `--- sanctuary`: a slide break on one screen only.
    OutputBreak { output: Output },
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
            Block::SlideBreak | Block::OutputBreak { .. } => String::new(),
        }
    }
}

/// Indent marker: a line's leading tabs are its indent level (`.ewm` files
/// write them as tabs too). Renderers strip them and indent the line.
pub const INDENT: char = '\t';

/// A line's indent level and the line without its leading tabs.
pub fn split_indent(line: &[Span]) -> (usize, Line) {
    let mut level = 0;
    let mut out = Vec::with_capacity(line.len());
    let mut leading = true;
    for span in line {
        if leading {
            let rest = span.text.trim_start_matches(INDENT);
            level += span.text.len() - rest.len();
            if rest.is_empty() {
                continue;
            }
            leading = false;
            out.push(Span { text: rest.to_string(), ..span.clone() });
        } else {
            out.push(span.clone());
        }
    }
    (level, out)
}

/// A line indented `level` steps (replacing any indent it had).
pub fn with_indent(line: &[Span], level: usize) -> Line {
    let (_, mut bare) = split_indent(line);
    if level > 0 {
        bare.insert(0, Span::plain(INDENT.to_string().repeat(level)));
    }
    bare
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
    /// True for the first slide after a heading (where a header is shown).
    #[serde(default, skip_serializing_if = "is_false")]
    pub section_start: bool,
    /// The level of the heading that starts this slide (`#` 1, `##` 2…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// Empty for a *title slide*: a heading followed directly by a slide
    /// break, shown as a title card (e.g. "First Reading" + citation).
    pub blocks: Vec<Block>,
}

impl Slide {
    pub fn is_title(&self) -> bool {
        self.blocks.is_empty() && self.heading.is_some()
    }
}

impl LiturgyDocument {
    /// Splits the body into slides.
    ///
    /// * A new slide begins at every `---` break and at every heading.
    /// * Headings are not blocks on the slide; they become the slide's
    ///   `heading`, carried onto following slides until the next heading.
    /// * A heading with no content before the next break (or the end) makes
    ///   a title slide.
    /// * Other empty slides are dropped.
    pub fn slides(&self) -> Vec<Slide> {
        let mut slides = Vec::new();
        let mut heading: Option<String> = None;
        let mut level: Option<u8> = None;
        let mut current: Vec<Block> = Vec::new();
        // A heading was seen and nothing has been emitted for it yet.
        let mut fresh = false;

        let flush = |slides: &mut Vec<Slide>,
                     current: &mut Vec<Block>,
                     heading: &Option<String>,
                     level: Option<u8>,
                     fresh: &mut bool| {
            if !current.is_empty() || (*fresh && heading.is_some()) {
                slides.push(Slide {
                    heading: heading.clone(),
                    section_start: *fresh,
                    level: level.filter(|_| *fresh),
                    blocks: std::mem::take(current),
                });
                *fresh = false;
            }
        };

        for block in &self.blocks {
            match block {
                Block::SlideBreak => flush(&mut slides, &mut current, &heading, level, &mut fresh),
                // One screen's breaks: see `crate::fit`.
                Block::OutputBreak { .. } => {}
                Block::Heading { text, level: l } => {
                    if !current.is_empty() {
                        flush(&mut slides, &mut current, &heading, level, &mut fresh);
                    } else if fresh {
                        // Two headings in a row: the first becomes a title slide.
                        flush(&mut slides, &mut current, &heading, level, &mut fresh);
                    }
                    heading = Some(text.clone());
                    level = Some(*l);
                    fresh = true;
                }
                other => current.push(other.clone()),
            }
        }
        flush(&mut slides, &mut current, &heading, level, &mut fresh);
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
        assert!(slides[1].section_start);
        assert_eq!(slides[2].heading.as_deref(), Some("Sanctus"));
        assert!(!slides[2].section_start);
        assert_eq!(slides[2].blocks[0].plain_text(), "C");
    }

    #[test]
    fn heading_followed_by_break_is_a_title_slide() {
        let doc = LiturgyDocument {
            meta: FrontMatter::default(),
            blocks: vec![
                Block::Heading { level: 1, text: "First Reading".into() },
                Block::SlideBreak,
                Block::speech(Role::Reader, ["The Word of the Lord."]),
                Block::Heading { level: 1, text: "Antiphon".into() },
                Block::Heading { level: 1, text: "Verse".into() },
                Block::speech(Role::People, ["x"]),
                Block::Heading { level: 1, text: "End".into() },
            ],
        };
        let slides = doc.slides();
        let shape: Vec<(Option<&str>, bool, usize)> =
            slides.iter().map(|s| (s.heading.as_deref(), s.section_start, s.blocks.len())).collect();
        assert_eq!(
            shape,
            vec![
                (Some("First Reading"), true, 0),
                (Some("First Reading"), false, 1),
                (Some("Antiphon"), true, 0),
                (Some("Verse"), true, 1),
                (Some("End"), true, 0),
            ]
        );
        assert!(slides[0].is_title());
        assert!(!slides[1].is_title());
    }
}
