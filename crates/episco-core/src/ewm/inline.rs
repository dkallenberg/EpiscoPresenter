//! Inline formatting: `**bold**`, `*italic*` and backslash escapes.

use crate::document::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marker {
    Bold,
    Italic,
}

impl Marker {
    fn literal(self) -> &'static str {
        match self {
            Marker::Bold => "**",
            Marker::Italic => "*",
        }
    }
}

#[derive(Debug)]
enum Tok {
    Text(String),
    Marker(Marker),
}

fn tokenize(src: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut text = String::new();
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.peek() {
                Some(&next) if next.is_ascii_punctuation() => {
                    text.push(next);
                    chars.next();
                }
                _ => text.push('\\'),
            },
            '*' => {
                if !text.is_empty() {
                    toks.push(Tok::Text(std::mem::take(&mut text)));
                }
                if chars.peek() == Some(&'*') {
                    chars.next();
                    toks.push(Tok::Marker(Marker::Bold));
                } else {
                    toks.push(Tok::Marker(Marker::Italic));
                }
            }
            _ => text.push(c),
        }
    }
    if !text.is_empty() {
        toks.push(Tok::Text(text));
    }
    toks
}

/// Parses one line of inline markup into formatted spans.
///
/// Markers act as toggles. An opening marker with no matching closer later in
/// the line is kept as literal text, so stray asterisks never swallow the rest
/// of a line.
pub fn parse_inline(src: &str) -> Vec<Span> {
    let toks = tokenize(src);
    let mut spans: Vec<Span> = Vec::new();
    let (mut bold, mut italic) = (false, false);

    let push = |spans: &mut Vec<Span>, text: &str, bold: bool, italic: bool| {
        if text.is_empty() {
            return;
        }
        match spans.last_mut() {
            Some(last) if last.bold == bold && last.italic == italic => last.text.push_str(text),
            _ => spans.push(Span { text: text.to_string(), bold, italic }),
        }
    };

    for (i, tok) in toks.iter().enumerate() {
        match tok {
            Tok::Text(t) => push(&mut spans, t, bold, italic),
            Tok::Marker(m) => {
                let open = match m {
                    Marker::Bold => &mut bold,
                    Marker::Italic => &mut italic,
                };
                if *open {
                    *open = false;
                } else if toks[i + 1..].iter().any(|t| matches!(t, Tok::Marker(n) if n == m)) {
                    *open = true;
                } else {
                    push(&mut spans, m.literal(), bold, italic);
                }
            }
        }
    }
    spans
}

/// Escapes characters that would otherwise be read as inline markup.
fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c == '*' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Serialises spans back to inline markup. `parse_inline(&write_inline(x)) == x`
/// for any spans produced by [`parse_inline`].
pub fn write_inline(spans: &[Span]) -> String {
    // Emit formatting *transitions* rather than wrapping each span, so that
    // adjacent spans never produce ambiguous runs like `****`. Within one
    // transition the bold toggle always precedes the italic toggle, which is
    // exactly how the tokenizer splits `***` (`**` then `*`).
    let mut out = String::new();
    let (mut bold, mut italic) = (false, false);
    let toggle = |out: &mut String, bold: &mut bool, italic: &mut bool, want_b: bool, want_i: bool| {
        if *bold != want_b {
            out.push_str("**");
            *bold = want_b;
        }
        if *italic != want_i {
            out.push('*');
            *italic = want_i;
        }
    };
    for span in spans.iter().filter(|s| !s.text.is_empty()) {
        toggle(&mut out, &mut bold, &mut italic, span.bold, span.italic);
        out.push_str(&escape_text(&span.text));
    }
    toggle(&mut out, &mut bold, &mut italic, false, false);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text() {
        assert_eq!(parse_inline("The Lord be with you."), vec![Span::plain("The Lord be with you.")]);
    }

    #[test]
    fn bold_and_italic() {
        assert_eq!(
            parse_inline("Say **Amen** *softly*."),
            vec![Span::plain("Say "), Span::bold("Amen"), Span::plain(" "), Span::italic("softly"), Span::plain("."),]
        );
    }

    #[test]
    fn whole_line_bold() {
        assert_eq!(parse_inline("**And also with you.**"), vec![Span::bold("And also with you.")]);
    }

    #[test]
    fn unmatched_markers_are_literal() {
        assert_eq!(parse_inline("5 * 3 = 15"), vec![Span::plain("5 * 3 = 15")]);
        assert_eq!(parse_inline("**open"), vec![Span::plain("**open")]);
    }

    #[test]
    fn escapes() {
        assert_eq!(parse_inline(r"\*not italic\*"), vec![Span::plain("*not italic*")]);
        assert_eq!(parse_inline(r"C:\path"), vec![Span::plain(r"C:\path")]);
    }

    #[test]
    fn round_trips_mixed_formatting() {
        let spans = vec![
            Span::bold("a"),
            Span::italic("b"),
            Span { text: "c*d".into(), bold: true, italic: true },
            Span::plain(r"e\f"),
        ];
        assert_eq!(parse_inline(&write_inline(&spans)), spans);
    }

    #[test]
    fn round_trips_every_three_span_formatting_sequence() {
        let fmts = [(false, false), (true, false), (false, true), (true, true)];
        for a in fmts {
            for b in fmts {
                for c in fmts {
                    let mut spans: Vec<Span> = Vec::new();
                    for (i, (bold, italic)) in [a, b, c].into_iter().enumerate() {
                        let text = format!("t{i}*");
                        match spans.last_mut() {
                            Some(l) if l.bold == bold && l.italic == italic => l.text.push_str(&text),
                            _ => spans.push(Span { text, bold, italic }),
                        }
                    }
                    let written = write_inline(&spans);
                    assert_eq!(parse_inline(&written), spans, "via {written:?}");
                }
            }
        }
    }
}
