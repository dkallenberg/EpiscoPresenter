//! Episcopal Worship Markdown (`.ewm`) reader and writer.
//!
//! See `docs/ewm-format.md` for the full syntax. In short:
//!
//! ```text
//! ---
//! title: Great Thanksgiving - Rite II
//! season: Pentecost
//! color: Green
//! ---
//!
//! [Priest] The Lord be with you.
//! [People] **And also with you.**
//! ```
//!
//! * `[Role] text` starts a speaker block; following non-blank lines continue it.
//! * A blank line ends the current block. Untagged text forms a role-less block.
//! * `# Heading` starts a new section (and slide); a lone `---` is a slide break.
//! * `![alt](path)` on its own line embeds a sheet-music snippet / image.
//! * `<!-- ... -->` comments (single or multi-line) are ignored.
//! * `\` escapes markup characters, e.g. `\[` at the start of a line.

mod frontmatter;
mod inline;

use std::fmt;
use std::path::Path;

use serde::Serialize;

use crate::document::{Block, FrontMatter, Line, LiturgyDocument};
use crate::role::Role;

pub use inline::{parse_inline, write_inline};

/// Conventional file extension.
pub const EXTENSION: &str = "ewm";

/// A non-fatal problem found while parsing. The document is still usable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    /// 1-based source line.
    pub line: usize,
    pub message: String,
}

impl Warning {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Warning { line, message: message.into() }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

/// A problem that prevents the file from being parsed at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("line {line}: front matter opened here is never closed with `---`")]
    UnterminatedFrontMatter { line: usize },
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("could not read {path}: {source}")]
    Io { path: String, source: std::io::Error },
    #[error("{path}: {source}")]
    Parse { path: String, source: ParseError },
}

/// A successfully parsed document plus any warnings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Parsed {
    pub document: LiturgyDocument,
    pub warnings: Vec<Warning>,
}

/// Reads and parses an `.ewm` file from disk.
pub fn load_file(path: impl AsRef<Path>) -> Result<Parsed, LoadError> {
    let path = path.as_ref();
    let display = path.display().to_string();
    let src = std::fs::read_to_string(path).map_err(|source| LoadError::Io { path: display.clone(), source })?;
    parse(&src).map_err(|source| LoadError::Parse { path: display, source })
}

/// Parses `.ewm` source text.
pub fn parse(src: &str) -> Result<Parsed, ParseError> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let lines: Vec<&str> = src.lines().collect();
    let mut warnings = Vec::new();
    let mut meta = FrontMatter::default();

    // ---- front matter ---------------------------------------------------
    let mut body_start = 0;
    if lines.first().map(|l| l.trim_end() == "---").unwrap_or(false) {
        let close = lines
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, l)| l.trim_end() == "---")
            .map(|(i, _)| i)
            .ok_or(ParseError::UnterminatedFrontMatter { line: 1 })?;
        for (i, line) in lines.iter().enumerate().take(close).skip(1) {
            frontmatter::apply_line(&mut meta, line, i + 1, &mut warnings);
        }
        body_start = close + 1;
        if meta.title.is_empty() {
            warnings.push(Warning::new(1, "front matter has no `title`"));
        }
    }

    // ---- body -----------------------------------------------------------
    let mut parser = BodyParser { blocks: Vec::new(), open: None, warnings: &mut warnings };
    let mut comment_start: Option<usize> = None;

    for (idx, raw) in lines.iter().enumerate().skip(body_start) {
        let line_no = idx + 1;
        let line = raw.trim();

        if comment_start.is_some() {
            if line.contains("-->") {
                comment_start = None;
            }
            continue;
        }
        if let Some(comment) = line.strip_prefix("<!--") {
            if !comment.contains("-->") {
                comment_start = Some(line_no);
            }
            continue;
        }
        if line.is_empty() {
            parser.close();
            continue;
        }
        if is_slide_break(line) {
            parser.close();
            parser.blocks.push(Block::SlideBreak);
            continue;
        }
        if let Some((level, text)) = parse_heading(line) {
            parser.close();
            parser.blocks.push(Block::Heading { level, text: text.to_string() });
            continue;
        }
        if let Some((alt, src)) = parse_image(line) {
            parser.close();
            parser.blocks.push(Block::Image { alt: alt.to_string(), src: src.to_string() });
            continue;
        }
        if let Some((role, rest)) = parse_role_tag(line) {
            parser.close();
            let mut lines = Vec::new();
            if !rest.is_empty() {
                lines.push(parse_line(rest, line_no, parser.warnings));
            }
            parser.open = Some(Open { role: Some(role), lines, line_no });
            continue;
        }
        match &mut parser.open {
            Some(open) => open.lines.push(parse_line(line, line_no, parser.warnings)),
            None => {
                let first = parse_line(line, line_no, parser.warnings);
                parser.open = Some(Open { role: None, lines: vec![first], line_no })
            }
        }
    }
    parser.close();
    let blocks = parser.blocks;

    if let Some(line) = comment_start {
        warnings.push(Warning::new(line, "comment `<!--` is never closed with `-->`"));
    }

    Ok(Parsed { document: LiturgyDocument { meta, blocks }, warnings })
}

/// Parses a line's inline markup, warning about a `**` left unmatched (the
/// usual cause is a bold run split across two lines).
fn parse_line(text: &str, line_no: usize, warnings: &mut Vec<Warning>) -> Line {
    let spans = parse_inline(text);
    if spans.iter().any(|s| s.text.contains("**")) {
        warnings.push(Warning::new(
            line_no,
            "unmatched `**`: bold must open and close on the same line (escape as `\\*\\*` for literal asterisks)",
        ));
    }
    spans
}

struct Open {
    role: Option<Role>,
    lines: Vec<Line>,
    line_no: usize,
}

struct BodyParser<'w> {
    blocks: Vec<Block>,
    open: Option<Open>,
    warnings: &'w mut Vec<Warning>,
}

impl BodyParser<'_> {
    fn close(&mut self) {
        if let Some(open) = self.open.take() {
            if open.lines.is_empty() {
                let role = open.role.as_ref().map(Role::label).unwrap_or("text");
                self.warnings.push(Warning::new(open.line_no, format!("`[{role}]` block has no text")));
            }
            self.blocks.push(Block::Paragraph { role: open.role, lines: open.lines });
        }
    }
}

fn is_slide_break(line: &str) -> bool {
    line.len() >= 3 && line.bytes().all(|b| b == b'-')
}

fn parse_heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let text = rest.trim().trim_end_matches('#').trim_end();
    (!text.is_empty()).then_some((hashes as u8, text))
}

fn parse_image(line: &str) -> Option<(&str, &str)> {
    let inner = line.strip_prefix("![")?.strip_suffix(')')?;
    let (alt, src) = inner.split_once("](")?;
    let src = src.trim();
    (!src.is_empty() && !alt.contains(']')).then_some((alt.trim(), src))
}

/// Recognises a leading `[Role]` tag. Returns the role and the remaining text.
fn parse_role_tag(line: &str) -> Option<(Role, &str)> {
    let inner_and_rest = line.strip_prefix('[')?;
    let close = inner_and_rest.find(']')?;
    let name = &inner_and_rest[..close];
    let rest = &inner_and_rest[close + 1..];
    let valid_name = !name.trim().is_empty()
        && name.len() <= 40
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '\'' | '.' | '&'));
    // `[text](url)` is a link, not a role tag.
    if !valid_name || rest.starts_with('(') {
        return None;
    }
    Some((Role::parse(name), rest.trim()))
}

/// Serialises a document to `.ewm` source.
pub fn to_string(doc: &LiturgyDocument) -> String {
    let mut out = String::from("---\n");
    frontmatter::write(&doc.meta, &mut out);
    out.push_str("---\n");

    let mut prev: Option<&Block> = None;
    for block in &doc.blocks {
        // Blank line between blocks; consecutive speaker lines in a dialogue
        // are written without one, matching the conventional style.
        let tight = matches!(
            (prev, block),
            (Some(Block::Paragraph { role: Some(_), lines: a }), Block::Paragraph { role: Some(_), lines: b })
                if a.len() <= 1 && b.len() <= 1 && !a.is_empty() && !b.is_empty()
        );
        if !tight {
            out.push('\n');
        }
        match block {
            Block::Paragraph { role, lines } => {
                for (i, line) in lines.iter().enumerate() {
                    let text = write_inline(line);
                    if i == 0 {
                        if let Some(role) = role {
                            out.push('[');
                            out.push_str(role.label());
                            out.push_str("] ");
                            out.push_str(&text);
                            out.push('\n');
                            continue;
                        }
                    }
                    out.push_str(&escape_line_start(&text));
                    out.push('\n');
                }
                if lines.is_empty() {
                    if let Some(role) = role {
                        out.push_str(&format!("[{}]\n", role.label()));
                    }
                }
            }
            Block::Heading { level, text } => {
                out.push_str(&"#".repeat((*level).clamp(1, 6) as usize));
                out.push(' ');
                out.push_str(text);
                out.push('\n');
            }
            Block::Image { alt, src } => out.push_str(&format!("![{alt}]({src})\n")),
            Block::SlideBreak => out.push_str("---\n"),
        }
        prev = Some(block);
    }
    out
}

/// Prevents an untagged / continuation line from being read as block syntax.
fn escape_line_start(text: &str) -> String {
    let t = text.trim_start();
    let ambiguous =
        t.starts_with('[') || t.starts_with('#') || t.starts_with("![") || t.starts_with("<!--") || is_slide_break(t);
    if ambiguous {
        format!("\\{t}")
    } else {
        t.to_string()
    }
}

impl std::str::FromStr for LiturgyDocument {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s).map(|p| p.document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::ElementCategory;
    use crate::document::Span;
    use crate::season::{LiturgicalColor, Season};

    const SURSUM: &str = r#"---
title: Great Thanksgiving - Rite II
season: Pentecost
color: Green
default_background: "green_stained_glass.png"
---

[Priest] The Lord be with you.
[People] **And also with you.**
[Priest] Lift up your hearts.
[People] **We lift them to the Lord.**
[Priest] Let us give thanks to the Lord our God.
[People] **It is right to give him thanks and praise.**
"#;

    #[test]
    fn parses_spec_example() {
        let parsed = parse(SURSUM).unwrap();
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        let doc = parsed.document;
        assert_eq!(doc.meta.title, "Great Thanksgiving - Rite II");
        assert_eq!(doc.meta.season, Some(Season::Pentecost));
        assert_eq!(doc.meta.color, Some(LiturgicalColor::Green));
        assert_eq!(doc.meta.default_background.as_deref(), Some("green_stained_glass.png"));
        assert_eq!(doc.blocks.len(), 6);
        assert_eq!(doc.blocks[0], Block::speech(Role::Priest, ["The Lord be with you."]));
        assert_eq!(
            doc.blocks[1],
            Block::Paragraph { role: Some(Role::People), lines: vec![vec![Span::bold("And also with you.")]] }
        );
        assert_eq!(doc.slides().len(), 1);
    }

    #[test]
    fn continuation_lines_and_blank_line_separation() {
        let src = "[Reader] A reading from Joshua.\nJoshua gathered all the tribes.\n\nThe Word of the Lord.\n";
        let doc = parse(src).unwrap().document;
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].role(), Some(&Role::Reader));
        assert_eq!(doc.blocks[0].plain_text(), "A reading from Joshua.\nJoshua gathered all the tribes.");
        assert_eq!(doc.blocks[1].role(), None);
    }

    #[test]
    fn headings_images_breaks_and_comments() {
        let src = "\
---
title: Hymn
type: hymn
hymn_number: 436
---
# Verse 1
<!-- operator note -->
Lift up your heads, ye mighty gates;
behold, the King of glory waits.
---
![Hymn 436 refrain](scores/h436-refrain.png)
<!--
multi-line
-->
## Verse 2
Text
";
        let parsed = parse(src).unwrap();
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        let doc = parsed.document;
        assert_eq!(doc.meta.category, Some(ElementCategory::Hymn));
        assert_eq!(doc.meta.hymn_number.as_deref(), Some("436"));
        assert!(matches!(&doc.blocks[0], Block::Heading { level: 1, text } if text == "Verse 1"));
        assert_eq!(doc.blocks[1].plain_text().lines().count(), 2);
        assert_eq!(doc.blocks[2], Block::SlideBreak);
        assert_eq!(
            doc.blocks[3],
            Block::Image { alt: "Hymn 436 refrain".into(), src: "scores/h436-refrain.png".into() }
        );
        assert!(matches!(&doc.blocks[4], Block::Heading { level: 2, .. }));
        assert_eq!(doc.slides().len(), 3);
    }

    #[test]
    fn links_and_escapes_are_not_role_tags() {
        let doc = parse("[see BCP](https://bcponline.org) p. 355\n\n\\[Priest] literal\n").unwrap().document;
        assert_eq!(doc.blocks.len(), 2);
        assert_eq!(doc.blocks[0].role(), None);
        assert_eq!(doc.blocks[1].role(), None);
        assert_eq!(doc.blocks[1].plain_text(), "[Priest] literal");
    }

    #[test]
    fn role_tag_on_its_own_line() {
        let doc = parse("[People]\n**Holy, holy, holy Lord,**\n**God of power and might,**\n").unwrap().document;
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].role(), Some(&Role::People));
        assert_eq!(doc.blocks[0].plain_text().lines().count(), 2);
    }

    #[test]
    fn warnings_do_not_abort() {
        let src = "---\ntitle: X\nseason: Summer\nnot a pair\n---\n[Deacon]\n\n<!-- open\n";
        let parsed = parse(src).unwrap();
        let lines: Vec<usize> = parsed.warnings.iter().map(|w| w.line).collect();
        assert_eq!(lines, vec![3, 4, 6, 8]);
        assert_eq!(parsed.document.meta.extra("season"), Some("Summer"));
    }

    #[test]
    fn bold_split_across_lines_warns() {
        let parsed = parse("[People] **I will open my mouth; *\nI will declare.**\n").unwrap();
        let lines: Vec<usize> = parsed.warnings.iter().map(|w| w.line).collect();
        assert_eq!(lines, vec![1, 2]);
        assert!(parse("[People] **ok** and 5 * 3\n").unwrap().warnings.is_empty());
    }

    #[test]
    fn unterminated_front_matter_is_an_error() {
        assert_eq!(parse("---\ntitle: X\n"), Err(ParseError::UnterminatedFrontMatter { line: 1 }));
    }

    #[test]
    fn no_front_matter_is_allowed() {
        let parsed = parse("[Priest] Let us pray.\n").unwrap();
        assert_eq!(parsed.document.meta, FrontMatter::default());
        assert_eq!(parsed.document.blocks.len(), 1);
    }

    #[test]
    fn crlf_and_bom() {
        let doc = parse("\u{feff}---\r\ntitle: X\r\n---\r\n[Priest] A\r\n[People] **B**\r\n").unwrap().document;
        assert_eq!(doc.meta.title, "X");
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn round_trip() {
        let original = parse(SURSUM).unwrap().document;
        let written = to_string(&original);
        assert_eq!(parse(&written).unwrap().document, original);
        // The spec example is already in canonical form (dialogue lines tight).
        assert_eq!(written, SURSUM.replace("\"green_stained_glass.png\"", "green_stained_glass.png"));
    }

    #[test]
    fn round_trip_tricky_content() {
        let doc = LiturgyDocument {
            meta: FrontMatter {
                title: "Note: \"quotes\"".into(),
                category: Some(ElementCategory::Reading),
                citation: Some("Joshua 24:1-3a".into()),
                extra: vec![("reader_name".into(), "Jane Doe".into())],
                ..Default::default()
            },
            blocks: vec![
                Block::Paragraph {
                    role: None,
                    lines: vec![vec![Span::plain("[Not] a role")], vec![Span::plain("# not a heading")]],
                },
                Block::Paragraph { role: Some(Role::Custom("Cantor".into())), lines: vec![] },
                Block::speech(Role::Reader, ["line one", "---", "*star*"]),
                Block::Heading { level: 2, text: "Section".into() },
                Block::SlideBreak,
                Block::Image { alt: "score".into(), src: "a b.png".into() },
            ],
        };
        let reparsed = parse(&to_string(&doc)).unwrap().document;
        assert_eq!(reparsed, doc);
    }
}
