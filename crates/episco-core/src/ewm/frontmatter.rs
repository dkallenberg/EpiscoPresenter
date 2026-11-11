//! Front matter: a flat `key: value` header between `---` fences.
//!
//! This is deliberately a small, strict subset of YAML (scalar values only,
//! optional single or double quotes, `#` comment lines) so that it can be
//! parsed without a YAML dependency and with precise line diagnostics.

use crate::document::FrontMatter;

use super::Warning;

/// Parses one front-matter line body. `line_no` is 1-based in the source file.
pub(super) fn apply_line(meta: &mut FrontMatter, raw: &str, line_no: usize, warnings: &mut Vec<Warning>) {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
        return;
    }
    let Some((key, value)) = line.split_once(':') else {
        warnings.push(Warning::new(line_no, format!("expected `key: value` in front matter, found `{line}`")));
        return;
    };
    let key = key.trim().to_ascii_lowercase();
    let value = match unquote(value.trim()) {
        Ok(v) => v,
        Err(msg) => {
            warnings.push(Warning::new(line_no, msg));
            value.trim().to_string()
        }
    };
    let opt = |v: String| if v.is_empty() { None } else { Some(v) };

    match key.as_str() {
        "title" => meta.title = value,
        "season" => meta.season = parse_enum(key, value, line_no, &mut meta.extra, warnings),
        "color" | "colour" => meta.color = parse_enum(key, value, line_no, &mut meta.extra, warnings),
        "type" | "category" => meta.category = parse_enum(key, value, line_no, &mut meta.extra, warnings),
        "default_background" | "background" => meta.default_background = opt(value),
        "citation" => meta.citation = opt(value),
        "hymn_number" | "hymn" => meta.hymn_number = opt(value),
        "tune" => meta.tune = opt(value),
        _ => meta.extra.push((key, value)),
    }
}

/// Parses an enum-valued key. Invalid values produce a warning and are kept in
/// `extra` (under the key as written) so that saving the file does not lose them.
fn parse_enum<T: std::str::FromStr<Err = String>>(
    key: String,
    value: String,
    line_no: usize,
    extra: &mut Vec<(String, String)>,
    warnings: &mut Vec<Warning>,
) -> Option<T> {
    if value.is_empty() {
        return None;
    }
    match value.parse() {
        Ok(v) => Some(v),
        Err(e) => {
            warnings.push(Warning::new(line_no, e));
            extra.push((key, value));
            None
        }
    }
}

fn unquote(v: &str) -> Result<String, String> {
    let bytes = v.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        let inner = &v[1..v.len() - 1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some(other) => out.push(other),
                    None => return Err("dangling `\\` in quoted value".into()),
                }
            } else {
                out.push(c);
            }
        }
        Ok(out)
    } else if bytes.len() >= 2 && bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\'' {
        Ok(v[1..v.len() - 1].replace("''", "'"))
    } else if v.starts_with('"') || v.starts_with('\'') {
        Err(format!("unterminated quoted value `{v}`"))
    } else {
        Ok(v.to_string())
    }
}

fn needs_quotes(v: &str) -> bool {
    v.is_empty()
        || v != v.trim()
        || v.starts_with(['"', '\'', '#', '[', '{', '&', '*', '!', '|', '>', '%', '@', '`'])
        || v.contains(": ")
        || v.contains(" #")
        || v.contains('\n')
}

fn quote(v: &str) -> String {
    if !needs_quotes(v) {
        return v.to_string();
    }
    let mut out = String::from("\"");
    for c in v.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Serialises front matter (without the `---` fences).
pub(super) fn write(meta: &FrontMatter, out: &mut String) {
    let mut kv = |k: &str, v: &str| {
        out.push_str(k);
        out.push_str(": ");
        out.push_str(&quote(v));
        out.push('\n');
    };
    kv("title", &meta.title);
    if let Some(c) = meta.category {
        kv("type", c.key());
    }
    if let Some(s) = meta.season {
        kv("season", s.label());
    }
    if let Some(c) = meta.color {
        kv("color", c.label());
    }
    if let Some(b) = &meta.default_background {
        kv("default_background", b);
    }
    if let Some(c) = &meta.citation {
        kv("citation", c);
    }
    if let Some(n) = &meta.hymn_number {
        kv("hymn_number", n);
    }
    if let Some(t) = &meta.tune {
        kv("tune", t);
    }
    for (k, v) in &meta.extra {
        kv(k, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unquotes_values() {
        assert_eq!(unquote(r#""a \"b\" c""#).unwrap(), r#"a "b" c"#);
        assert_eq!(unquote("'it''s'").unwrap(), "it's");
        assert_eq!(unquote("plain").unwrap(), "plain");
        assert!(unquote("\"open").is_err());
    }

    #[test]
    fn quotes_only_when_needed() {
        assert_eq!(quote("Great Thanksgiving - Rite II"), "Great Thanksgiving - Rite II");
        assert_eq!(quote("Hymn: 436"), "\"Hymn: 436\"");
        assert_eq!(quote(""), "\"\"");
    }
}
