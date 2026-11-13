//! Fitting an item's text to each screen's slides.
//!
//! Every slide type holds so many lines of text: the number set in the
//! Layout tool, or what its text box holds at its type size. The sanctuary
//! keeps the slides as written (the `---` breaks) and only splits one that
//! is too long. The live stream's boxes usually hold more, so it runs the
//! written slides together where they fit (two verses of a hymn, more of
//! the Creed), and splits where they do not.
//!
//! A split falls at the end of a line where it can, else at the punctuation
//! nearest the end of the room (a full stop, semicolon or colon before a
//! comma, a comma before a space), and a response stays on the slide of the
//! line it answers. `--- stream` and `--- sanctuary` in a file break one
//! screen only.
//!
//! The operator steps through both screens together: a step begins wherever
//! either screen begins a slide, and the other screen holds its slide.

use crate::category::ElementCategory;
use crate::document::{split_indent, with_indent, Block, Line, LiturgyDocument, Slide, Span};
use crate::look::{Layout, Output};
use crate::template::Templates;

/// Room between paragraphs, in lines (none in a box of a line or two).
const GAP: f32 = 0.25;

fn gap(cap: &Capacity) -> f32 {
    if cap.lines < 4.0 {
        0.0
    } else {
        GAP
    }
}

/// How much text one slide type holds, and how it lays text out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Capacity {
    /// Lines of text a slide holds.
    pub lines: f32,
    /// Characters a line holds (less a speaker label's column).
    pub chars: usize,
    /// A paragraph's lines run together (the stream's hymn verses).
    pub join: bool,
    /// Rubrics are shown, so they take room.
    pub rubrics: bool,
    /// Written slides run together where they fit.
    pub pack: bool,
    /// How far past `lines` a written slide may run before it is split
    /// (the text shrinks a little to fit): 1.0 when the operator set the
    /// number of lines.
    pub slack: f32,
}

impl Capacity {
    /// No fitting: the slides as written.
    pub const AS_WRITTEN: Capacity =
        Capacity { lines: f32::INFINITY, chars: 1000, join: false, rubrics: false, pack: false, slack: 1.0 };
}

/// The built-in text box of each slide type: width, height and type size on
/// the 1920×1080 stage, and whether lines run together. These match the
/// defaults in `ui/js/templates.js`.
const TEXT_BOXES: &[(&str, f32, f32, f32, bool)] = &[
    ("stream.left_panel", 800.0, 908.0, 52.0, false),
    ("stream.bottom_bar", 1808.0, 344.0, 50.0, false),
    ("stream.lower_third", 1228.0, 116.0, 50.0, false),
    ("stream.hymn", 960.0, 830.0, 52.0, true),
    ("sanctuary.prayer", 1780.0, 718.0, 73.0, false),
    ("sanctuary.psalm", 1780.0, 718.0, 73.0, false),
];

/// The slide type an item's slides use on a screen (as `slideTypeFor` in
/// `ui/js/templates.js`), or `None` for the fixed layouts.
pub fn slide_type(output: Output, layout: Layout, category: ElementCategory) -> Option<&'static str> {
    match output {
        Output::Sanctuary => match layout {
            Layout::Lyrics | Layout::InfoBar | Layout::LowerThird => None,
            _ if category == ElementCategory::Homily => Some("sanctuary.sermon"),
            _ if category == ElementCategory::Psalm => Some("sanctuary.psalm"),
            _ => Some("sanctuary.prayer"),
        },
        Output::Stream => match layout {
            Layout::LeftPanel => Some("stream.left_panel"),
            Layout::LeftPanelWide => Some("stream.hymn"),
            Layout::BottomBar => Some("stream.bottom_bar"),
            Layout::LowerThird => Some("stream.lower_third"),
            _ => None,
        },
    }
}

/// The slide types whose text can be fitted.
pub fn fits(key: &str) -> bool {
    TEXT_BOXES.iter().any(|t| t.0 == key)
}

/// What a slide type holds, with the operator's changes from the Layout tool
/// (a moved or resized text box, a new type size, "Lines per slide").
pub fn capacity(key: Option<&str>, templates: &Templates) -> Capacity {
    let Some((key, w, h, size, join)) = key.and_then(|k| TEXT_BOXES.iter().find(|t| t.0 == k)).copied() else {
        return Capacity::AS_WRITTEN;
    };
    let custom = templates.get(key);
    let text = custom.and_then(|t| t.elements.get("text"));
    let w = text.and_then(|e| e.w).unwrap_or(w);
    let h = text.and_then(|e| e.h).unwrap_or(h);
    let size = text.and_then(|e| e.size).unwrap_or(size).max(8.0);
    let stream = key.starts_with("stream.");
    let set = custom.and_then(|t| t.max_lines);
    // The sanctuary keeps its slides as written unless told how many lines
    // they hold; the stream fills its boxes (line height 1.25).
    let lines = match set {
        Some(n) => n as f32,
        None if stream => (h / (size * 1.25)).round(),
        None => f32::INFINITY,
    }
    .max(1.0);
    let join = custom.and_then(|t| t.line_breaks.as_deref()).map(|b| b == "join").unwrap_or(join);
    Capacity {
        lines,
        chars: ((w / (size * 0.46)) as usize).max(10),
        join,
        rubrics: stream,
        pack: stream,
        slack: if set.is_some() { 1.0 } else { 1.2 },
    }
}

/// The slides of both screens and the steps through them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fitted {
    /// The sanctuary's slide at each step (a slide repeats while the stream
    /// moves on through a longer passage).
    pub steps: Vec<Slide>,
    /// The stream's slides, when they differ from the steps.
    pub stream: Vec<Slide>,
    /// The stream's slide at each step (empty with `stream`).
    pub stream_at: Vec<usize>,
}

/// Fits `doc` to both screens. `keep` says which blocks are shown at all
/// (a reading's slides leave out its text).
pub fn fit(doc: &LiturgyDocument, sanctuary: Capacity, stream: Capacity, keep: &dyn Fn(&Block) -> bool) -> Fitted {
    let a = layout(doc, Output::Sanctuary, sanctuary, keep);
    let b = layout(doc, Output::Stream, stream, keep);
    if a.is_empty() || b.is_empty() {
        return Fitted { steps: a.into_iter().map(|s| s.0).collect(), ..Default::default() };
    }
    let mut starts: Vec<Pos> = a.iter().chain(b.iter()).map(|s| s.1).collect();
    starts.sort_unstable();
    starts.dedup();
    let at = |list: &[(Slide, Pos)], p: Pos| list.iter().rposition(|s| s.1 <= p).unwrap_or(0);
    let same = a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| x == y);
    if same {
        return Fitted { steps: a.into_iter().map(|s| s.0).collect(), ..Default::default() };
    }
    let steps = starts.iter().map(|&p| a[at(&a, p)].0.clone()).collect();
    let stream_at = starts.iter().map(|&p| at(&b, p)).collect();
    Fitted { steps, stream: b.into_iter().map(|s| s.0).collect(), stream_at }
}

/// Where a slide begins in the document: block, line, character.
type Pos = (usize, usize, usize);

#[derive(Debug, Clone)]
struct Part {
    block: Block,
    at: Pos,
}

/// How a written slide began, weakest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Begins {
    Start,
    /// A `---` break: the stream may run text on across it.
    Break,
    /// A break for this screen only: always kept.
    Own,
    Heading,
}

struct Draft {
    heading: Option<String>,
    section_start: bool,
    parts: Vec<Part>,
    begins: Begins,
    start: Pos,
}

/// One screen's slides, each with where it begins.
fn layout(doc: &LiturgyDocument, output: Output, cap: Capacity, keep: &dyn Fn(&Block) -> bool) -> Vec<(Slide, Pos)> {
    let drafts = gather(doc, output, keep);
    let mut packed: Vec<Draft> = Vec::new();
    for d in drafts {
        if let Some(prev) = packed.last_mut() {
            if cap.pack && d.begins == Begins::Break && mergeable(prev) && mergeable(&d) {
                let both = total(prev.parts.iter().chain(&d.parts), &cap);
                if both <= cap.lines + 0.01 {
                    prev.parts.extend(d.parts);
                    continue;
                }
            }
        }
        packed.push(d);
    }
    let mut out = Vec::new();
    for d in packed {
        let too_long = total(d.parts.iter(), &cap) > cap.lines * cap.slack + 0.01;
        let pieces = if too_long && mergeable(&d) { split(d.parts, &cap) } else { vec![d.parts] };
        for (i, parts) in pieces.into_iter().enumerate() {
            let start = if i == 0 { d.start } else { parts.first().map_or(d.start, |p| p.at) };
            let slide = Slide {
                heading: d.heading.clone(),
                section_start: d.section_start && i == 0,
                blocks: parts.into_iter().map(|p| p.block).collect(),
            };
            out.push((slide, start));
        }
    }
    out
}

/// A slide that may be run together with another, or split: text only.
fn mergeable(d: &Draft) -> bool {
    !d.parts.is_empty() && d.parts.iter().all(|p| matches!(p.block, Block::Paragraph { .. }))
}

struct Gather<'a> {
    out: Vec<Draft>,
    heading: Option<String>,
    parts: Vec<Part>,
    /// Text was seen since the last break (perhaps all left out).
    saw: bool,
    fresh: bool,
    fresh_at: Pos,
    begins: Begins,
    keep: &'a dyn Fn(&Block) -> bool,
}

impl Gather<'_> {
    fn flush(&mut self) -> bool {
        let pushed = if !self.parts.is_empty() {
            let start = if self.fresh { self.fresh_at } else { self.parts[0].at };
            self.out.push(Draft {
                heading: self.heading.clone(),
                section_start: self.fresh,
                parts: std::mem::take(&mut self.parts),
                begins: if self.fresh { Begins::Heading } else { self.begins },
                start,
            });
            true
        } else if self.fresh && self.heading.is_some() && !self.saw {
            // A heading alone: a title slide.
            self.out.push(Draft {
                heading: self.heading.clone(),
                section_start: true,
                parts: Vec::new(),
                begins: Begins::Heading,
                start: self.fresh_at,
            });
            true
        } else {
            false
        };
        if pushed || self.saw {
            self.fresh = false;
        }
        self.saw = false;
        pushed
    }
}

/// The written slides for one screen (its own breaks and the `---` ones).
fn gather(doc: &LiturgyDocument, output: Output, keep: &dyn Fn(&Block) -> bool) -> Vec<Draft> {
    let mut g = Gather {
        out: Vec::new(),
        heading: None,
        parts: Vec::new(),
        saw: false,
        fresh: false,
        fresh_at: (0, 0, 0),
        begins: Begins::Start,
        keep,
    };
    for (i, block) in doc.blocks.iter().enumerate() {
        match block {
            Block::SlideBreak => {
                let pushed = g.flush();
                g.begins = if pushed { Begins::Break } else { g.begins.max(Begins::Break) };
            }
            Block::OutputBreak { output: o } if *o == output => {
                g.flush();
                g.begins = Begins::Own;
            }
            Block::OutputBreak { .. } => {}
            Block::Heading { text, .. } => {
                if !g.parts.is_empty() || g.fresh || g.saw {
                    g.flush();
                }
                g.heading = Some(text.clone());
                g.fresh = true;
                g.fresh_at = (i, 0, 0);
            }
            other => {
                g.saw = true;
                if (g.keep)(other) {
                    g.parts.push(Part { block: other.clone(), at: (i, 0, 0) });
                }
            }
        }
    }
    g.flush();
    let mut out = g.out;
    if let Some(first) = out.first_mut() {
        if first.begins != Begins::Heading {
            first.begins = Begins::Start;
        }
    }
    out
}

// ---- measuring --------------------------------------------------------------

fn is_rubric(b: &Block) -> bool {
    matches!(b, Block::Paragraph { role: Some(r), .. } if r.is_rubric())
}

fn is_response(b: &Block) -> bool {
    matches!(b, Block::Paragraph { role: Some(r), .. } if r.is_congregational())
}

/// Characters a line of this block holds (a label takes a column).
fn line_chars(b: &Block, cap: &Capacity) -> usize {
    match b {
        // The label is set smaller than the text (about 0.6 of its width a
        // letter), with a gap after it.
        Block::Paragraph { role: Some(r), .. } if !r.label().eq_ignore_ascii_case("antiphon") => {
            let label = (r.label().chars().count() as f32 + 1.5) * 0.6;
            ((cap.chars as f32) - label) as usize
        }
        _ => cap.chars,
    }
    .max(8)
}

fn text_len(line: &[Span]) -> (usize, usize) {
    let (indent, bare) = split_indent(line);
    (indent, bare.iter().map(|s| s.text.chars().count()).sum())
}

fn line_rows(line: &[Span], chars: usize) -> f32 {
    let (indent, n) = text_len(line);
    ((n + indent * 3).max(1) as f32 / chars as f32).ceil()
}

/// Lines a block takes on a slide.
fn rows(b: &Block, cap: &Capacity) -> f32 {
    let Block::Paragraph { lines, .. } = b else { return 0.0 };
    if is_rubric(b) && !cap.rubrics {
        return 0.0;
    }
    let chars = line_chars(b, cap);
    if cap.join || is_rubric(b) {
        let n: usize = lines.iter().map(|l| text_len(l).1 + 1).sum();
        return (n.max(1) as f32 / chars as f32).ceil();
    }
    lines.iter().map(|l| line_rows(l, chars)).sum()
}

fn total<'a>(parts: impl Iterator<Item = &'a Part>, cap: &Capacity) -> f32 {
    let mut sum = 0.0;
    let mut shown: usize = 0;
    for p in parts {
        let r = rows(&p.block, cap);
        if r > 0.0 {
            sum += r;
            shown += 1;
        }
    }
    sum + gap(cap) * shown.saturating_sub(1) as f32
}

// ---- splitting ---------------------------------------------------------------

/// Fills slides with the parts in order.
struct Filler<'a> {
    cap: &'a Capacity,
    slides: Vec<Vec<Part>>,
    cur: Vec<Part>,
    used: f32,
}

impl Filler<'_> {
    fn gap(&self) -> f32 {
        if self.cur.iter().any(|p| rows(&p.block, self.cap) > 0.0) {
            gap(self.cap)
        } else {
            0.0
        }
    }

    fn room(&self) -> f32 {
        self.cap.lines - self.used - self.gap()
    }

    fn add(&mut self, part: Part) {
        let r = rows(&part.block, self.cap);
        if r > 0.0 {
            self.used += self.gap() + r;
        }
        self.cur.push(part);
    }

    fn flush(&mut self) {
        if !self.cur.is_empty() {
            self.slides.push(std::mem::take(&mut self.cur));
        }
        self.used = 0.0;
    }

    /// A part kept whole, on a new slide if it does not fit here.
    fn whole(&mut self, part: Part) {
        let r = rows(&part.block, self.cap);
        if r > self.room() + 0.01 && !self.cur.is_empty() {
            self.flush();
        }
        self.add(part);
    }

    /// A paragraph split line by line (and within a line if it must be),
    /// leaving `reserve` lines after its last line for a response.
    fn paragraph(&mut self, part: Part, reserve: f32) {
        let Block::Paragraph { role, lines } = part.block.clone() else {
            return self.whole(part);
        };
        let chars = line_chars(&part.block, self.cap);
        let b = part.at.0;
        let n = lines.len();
        let mut piece: Vec<Line> = Vec::new();
        let mut piece_at = part.at;
        let mut piece_rows = 0.0;
        // Room for the piece being built.
        let base = |f: &Filler| f.room();
        let mut room = base(self);
        let commit = |f: &mut Filler, piece: &mut Vec<Line>, at: Pos| {
            if !piece.is_empty() {
                f.add(Part { block: Block::Paragraph { role: role.clone(), lines: std::mem::take(piece) }, at });
            }
        };
        for (j, line) in lines.into_iter().enumerate() {
            let last = j + 1 == n;
            let (indent, mut rest) = split_indent(&line);
            let mut off = 0;
            loop {
                let r = line_rows(&with_indent(&rest, indent), chars);
                let avail = room - piece_rows - if last { reserve } else { 0.0 };
                if r <= avail + 0.01 {
                    piece.push(with_indent(&rest, indent));
                    piece_rows += r;
                    break;
                }
                if !piece.is_empty() || !self.cur.is_empty() {
                    commit(self, &mut piece, piece_at);
                    self.flush();
                    piece_rows = 0.0;
                    room = base(self);
                    piece_at = (b, j, off);
                    continue;
                }
                // Alone on a slide and still too long: split the line.
                let fit_rows = avail.max(1.0).floor();
                let limit = ((fit_rows * chars as f32) as usize).saturating_sub(indent * 3).max(chars / 2);
                match split_line(&rest, limit) {
                    Some((head, tail, used)) => {
                        piece.push(with_indent(&head, indent));
                        commit(self, &mut piece, piece_at);
                        self.flush();
                        piece_rows = 0.0;
                        room = base(self);
                        off += used;
                        piece_at = (b, j, off);
                        rest = tail;
                    }
                    None => {
                        piece.push(with_indent(&rest, indent));
                        piece_rows += r;
                        break;
                    }
                }
            }
        }
        commit(self, &mut piece, piece_at);
    }
}

/// Splits a slide's parts into slides that fit.
fn split(parts: Vec<Part>, cap: &Capacity) -> Vec<Vec<Part>> {
    // Groups: rubrics go with what follows; a short response goes with the
    // words it answers.
    let mut groups: Vec<Vec<Part>> = Vec::new();
    let mut pending: Vec<Part> = Vec::new();
    for p in parts {
        if is_rubric(&p.block) {
            pending.push(p);
            continue;
        }
        let answers = is_response(&p.block) && rows(&p.block, cap) <= 2.0;
        match groups.last_mut() {
            Some(g) if pending.is_empty() && answers && g.last().is_some_and(|l| !is_response(&l.block)) => g.push(p),
            _ => {
                let mut g = std::mem::take(&mut pending);
                g.push(p);
                groups.push(g);
            }
        }
    }
    if !pending.is_empty() {
        groups.push(pending);
    }

    let mut f = Filler { cap, slides: Vec::new(), cur: Vec::new(), used: 0.0 };
    for g in groups {
        let need = total(g.iter(), cap);
        if need <= f.room() + 0.01 {
            g.into_iter().for_each(|p| f.add(p));
            continue;
        }
        if !f.cur.is_empty() && need <= cap.lines + 0.01 {
            f.flush();
            g.into_iter().for_each(|p| f.add(p));
            continue;
        }
        // Too long for one slide: split the words, keeping the response(s)
        // at the end with the last of them.
        let responses = g.iter().rev().take_while(|p| is_response(&p.block)).count().min(g.len() - 1);
        let lead = g.len() - responses;
        let reserve = if responses > 0 { total(g[lead..].iter(), cap) + gap(cap) } else { 0.0 };
        for (k, p) in g.into_iter().enumerate() {
            if k < lead && rows(&p.block, cap) > 0.0 {
                f.paragraph(p, if k + 1 == lead { reserve } else { 0.0 });
            } else {
                f.whole(p);
            }
        }
    }
    f.flush();
    f.slides
}

/// Splits a line at the best place at or before `limit` characters: after a
/// full stop, semicolon, colon, question or exclamation mark; else after a
/// comma; else at a space. Returns the two halves and how many characters
/// the first took (with the space after it).
fn split_line(line: &[Span], limit: usize) -> Option<(Line, Line, usize)> {
    let text: String = line.iter().map(|s| s.text.as_str()).collect();
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= limit {
        return None;
    }
    let limit = limit.min(chars.len() - 1);
    let floor = limit * 2 / 5;
    let after = |set: &[char]| {
        (floor.max(1)..=limit).rev().find(|&i| chars.get(i) == Some(&' ') && i > 0 && set.contains(&chars[i - 1]))
    };
    let at = after(&['.', ';', ':', '?', '!'])
        .or_else(|| after(&[',']))
        .or_else(|| (1..=limit).rev().find(|&i| chars[i] == ' '))?;
    // `at` is a space: the head ends before it, the tail starts after it.
    let mut skip = at;
    while chars.get(skip) == Some(&' ') {
        skip += 1;
    }
    if skip >= chars.len() {
        return None;
    }
    Some((take_chars(line, 0, at), take_chars(line, skip, chars.len()), skip))
}

/// The spans covering characters `from..to` of a line.
fn take_chars(line: &[Span], from: usize, to: usize) -> Line {
    let mut out = Vec::new();
    let mut pos = 0;
    for s in line {
        let n = s.text.chars().count();
        let (a, b) = (from.max(pos), to.min(pos + n));
        if a < b {
            let text: String = s.text.chars().skip(a - pos).take(b - a).collect();
            out.push(Span { text, ..s.clone() });
        }
        pos += n;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role::Role;

    fn doc(text: &str) -> LiturgyDocument {
        crate::ewm::parse(&format!("---\ntitle: T\n---\n{text}")).unwrap().document
    }

    fn cap(lines: f32, chars: usize, pack: bool) -> Capacity {
        Capacity { lines, chars, join: false, rubrics: pack, pack, slack: 1.0 }
    }

    fn texts(s: &Slide) -> Vec<String> {
        s.blocks.iter().map(|b| b.plain_text()).collect()
    }

    #[test]
    fn the_stream_runs_written_slides_together_where_they_fit() {
        let d = doc("[1] One\nline\n\n---\n\n[2] Two\nline\n\n---\n\n[3] Three\nline\n\n---\n\n# Next\n\n[4] Four\n");
        let f = fit(&d, Capacity::AS_WRITTEN, cap(5.0, 40, true), &|_| true);
        // Four sanctuary slides; the stream holds two verses a slide, and
        // starts again at the heading.
        assert_eq!(f.steps.len(), 4);
        assert_eq!(f.stream.len(), 3);
        assert_eq!(f.stream_at, [0, 0, 1, 2]);
        assert_eq!(texts(&f.stream[0]), ["One\nline", "Two\nline"]);
        assert!(f.stream[2].section_start);
    }

    #[test]
    fn same_slides_need_no_stream_list() {
        let d = doc("[Priest] A\n\n---\n\n[People] B\n");
        let f = fit(&d, Capacity::AS_WRITTEN, Capacity::AS_WRITTEN, &|_| true);
        assert_eq!(f.steps.len(), 2);
        assert!(f.stream.is_empty() && f.stream_at.is_empty());
    }

    #[test]
    fn a_long_slide_splits_at_punctuation_and_keeps_the_response() {
        let d = doc(
            "[Priest] Almighty God, unto whom all hearts be open, all desires known, and from whom no secrets are hid; cleanse the thoughts of our hearts by the inspiration of thy Holy Spirit.\n[People] **Amen.**\n",
        );
        // Three lines of 40 characters a slide (35 with the label column).
        let f = fit(&d, Capacity::AS_WRITTEN, cap(3.0, 40, true), &|_| true);
        assert_eq!(f.steps.len(), f.stream_at.len());
        assert!(f.stream.len() >= 3, "{:?}", f.stream);
        for s in &f.stream {
            assert!(
                total(
                    s.blocks.iter().map(|b| Part { block: b.clone(), at: (0, 0, 0) }).collect::<Vec<_>>().iter(),
                    &cap(3.0, 40, true)
                ) <= 3.01
            );
        }
        // Each piece ends at punctuation where there is some.
        let first = f.stream[0].blocks[0].plain_text();
        assert!(first.ends_with(',') || first.ends_with(';'), "{first}");
        // The response is on the slide with the last words.
        let last = f.stream.last().unwrap();
        assert_eq!(last.blocks.last().unwrap().role(), Some(&Role::People));
        assert!(last.blocks.len() == 2, "{last:?}");
        // Nothing is lost.
        let all: Vec<String> = f
            .stream
            .iter()
            .flat_map(|s| s.blocks.iter().filter(|b| b.role() == Some(&Role::Priest)).map(|b| b.plain_text()))
            .collect();
        assert_eq!(all.join(" "), d.blocks[0].plain_text());
        // The sanctuary holds its one slide.
        assert!(f.steps.iter().all(|s| s == &f.steps[0]));
    }

    #[test]
    fn own_breaks_and_title_slides() {
        let d = doc("# The First Reading\n---\n[Reader] The word of the Lord.\n--- stream\n[People] **Thanks be to God.**\n--- sanctuary\n[Reader] x\n");
        let f = fit(&d, Capacity::AS_WRITTEN, cap(10.0, 40, true), &|_| true);
        // Sanctuary: title, then "word … Thanks", then "x"; stream: title,
        // "word", then "Thanks … x" (the sanctuary's break is not the stream's).
        assert_eq!(f.steps.len(), 4);
        assert!(f.steps[0].is_title());
        assert_eq!(texts(&f.steps[1]), ["The word of the Lord.", "Thanks be to God."]);
        assert_eq!(texts(&f.stream[2]), ["Thanks be to God.", "x"]);
        assert_eq!(f.stream_at, [0, 1, 2, 2]);
    }

    #[test]
    fn left_out_text_and_indents() {
        let d = doc("# Reading\n---\nThe text itself.\n\n---\n\n[Reader] The word of the Lord.\n");
        let keep = |b: &Block| !matches!(b, Block::Paragraph { role: None, .. });
        let f = fit(&d, Capacity::AS_WRITTEN, Capacity::AS_WRITTEN, &keep);
        assert_eq!(f.steps.len(), 2);
        let ind = doc("[All] Our Father in heaven,\n\thallowed be your Name,\n      on earth as in heaven.\n");
        let Block::Paragraph { lines, .. } = &ind.blocks[0] else { panic!() };
        assert_eq!(split_indent(&lines[1]).0, 1);
        assert_eq!(split_indent(&lines[2]).0, 2);
        assert_eq!(crate::ewm::to_string(&ind).matches('\t').count(), 3);
    }

    #[test]
    fn capacity_follows_the_layout_tool() {
        let mut t = Templates::new();
        assert_eq!(capacity(Some("stream.lower_third"), &t).lines, 2.0);
        assert_eq!(capacity(Some("sanctuary.title"), &t), Capacity::AS_WRITTEN);
        assert!(capacity(Some("sanctuary.prayer"), &t).lines.is_infinite(), "the sanctuary as written");
        assert_eq!(capacity(Some("stream.left_panel"), &t).lines, 14.0);
        t.insert(
            "stream.lower_third".into(),
            crate::template::SlideTemplate { max_lines: Some(3), ..Default::default() },
        );
        assert_eq!(capacity(Some("stream.lower_third"), &t).lines, 3.0);
        assert!(capacity(Some("stream.hymn"), &t).join);
    }
}

#[cfg(test)]
mod library_texts {
    #[test]
    fn sense_lines_round_trip() {
        for f in ["nicene-creed", "lords-prayer", "confession", "rite1/nicene-creed", "rite1/confession"] {
            let path = format!("{}/../../library/{f}.ewm", env!("CARGO_MANIFEST_DIR"));
            let text = std::fs::read_to_string(&path).unwrap();
            let parsed = crate::ewm::parse(&text).unwrap();
            assert!(parsed.warnings.is_empty(), "{f}: {:?}", parsed.warnings);
            let again = crate::ewm::to_string(&parsed.document);
            assert_eq!(crate::ewm::parse(&again).unwrap().document, parsed.document, "{f}");
        }
    }
}
