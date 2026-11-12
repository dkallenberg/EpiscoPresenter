//! Word (`.docx`) files from a bulletin or script [`Doc`].
//!
//! Written directly as Office Open XML so there is nothing to install. The
//! bulletin is set up for **book fold** printing (Word: Layout → Margins →
//! Custom Margins → Multiple pages: Book fold): each sheet of letter paper,
//! printed on both sides and folded, carries four half-letter pages, and Word
//! puts the pages in booklet order when printing. The script is plain letter
//! paper in large type with the rubrics in red.

use std::io::{Cursor, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;

use crate::bulletin::{escape, Doc, DocKind, ParishInfo, Piece, Run};

/// Layout numbers for one kind of document (twentieths of a point).
struct Page {
    width: u32,
    height: u32,
    landscape: bool,
    margin: u32,
    /// Width of the text column.
    text: u32,
    /// Body size in half-points.
    size: u32,
    font: &'static str,
    rubric_color: Option<&'static str>,
    /// Indent of the text after a speaker label.
    hang: u32,
}

impl Page {
    fn for_kind(kind: DocKind) -> Page {
        match kind {
            // Book fold: letter landscape, each half a page 5.5 × 8.5 in.
            DocKind::Bulletin => Page {
                width: 15840,
                height: 12240,
                landscape: true,
                margin: 720,
                text: 7920 - 2 * 720,
                size: 22,
                font: "Garamond",
                rubric_color: None,
                hang: 1080,
            },
            DocKind::Script => Page {
                width: 12240,
                height: 15840,
                landscape: false,
                margin: 1080,
                text: 12240 - 2 * 1080,
                size: 30,
                font: "Garamond",
                rubric_color: Some("C00000"),
                hang: 1800,
            },
        }
    }
}

/// A picture to embed.
struct Media {
    name: String,
    bytes: Vec<u8>,
}

struct Writer<'a> {
    page: &'a Page,
    body: String,
    media: Vec<Media>,
}

/// Width and height in pixels of a PNG or JPEG.
pub fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.len() >= 24 {
        let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
        let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
        return (w > 0 && h > 0).then_some((w, h));
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        while i + 9 < bytes.len() {
            if bytes[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = bytes[i + 1];
            let len = usize::from(u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]));
            // Start-of-frame markers (not DHT, JPG or DAC).
            if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
                let h = u32::from(u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]));
                let w = u32::from(u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]));
                return (w > 0 && h > 0).then_some((w, h));
            }
            i += 2 + len;
        }
    }
    None
}

fn run_xml(r: &Run, extra: &str) -> String {
    let mut props = String::from(extra);
    if r.bold {
        props.push_str("<w:b/>");
    }
    if r.italic {
        props.push_str("<w:i/>");
    }
    let rpr = if props.is_empty() { String::new() } else { format!("<w:rPr>{props}</w:rPr>") };
    format!("<w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r>", escape(&r.text))
}

fn text_run(text: &str, props: &str) -> String {
    run_xml(&Run { text: text.to_string(), bold: false, italic: false, src: None }, props)
}

/// Lines joined with line breaks.
fn lines_xml(lines: &[Vec<Run>], extra: &str) -> String {
    let mut x = String::new();
    for (i, l) in lines.iter().enumerate() {
        if i > 0 {
            x.push_str("<w:r><w:br/></w:r>");
        }
        for r in l {
            x.push_str(&run_xml(r, extra));
        }
    }
    x
}

impl Writer<'_> {
    fn para(&mut self, style: &str, ppr: &str, content: &str) {
        self.body.push_str(&format!("<w:p><w:pPr><w:pStyle w:val=\"{style}\"/>{ppr}</w:pPr>{content}</w:p>"));
    }

    fn page_break(&mut self) {
        self.body.push_str("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
    }

    /// An inline picture at most `max_w` twips wide (and the page tall).
    fn image(&mut self, path: &Path, max_w: u32, max_h: u32) -> Option<String> {
        let bytes = std::fs::read(path).ok()?;
        let size = image_size(&bytes)?;
        let ext = if bytes.starts_with(&[0xFF, 0xD8]) { "jpeg" } else { "png" };
        let n = self.media.len() + 1;
        let name = format!("image{n}.{ext}");
        // Full width for wide images (lines of music); otherwise 96 dpi.
        let natural_w = size.0 * 15;
        let mut w = natural_w.min(max_w);
        let mut h = (u64::from(w) * u64::from(size.1) / u64::from(size.0)) as u32;
        if h > max_h {
            w = (u64::from(w) * u64::from(max_h) / u64::from(h)) as u32;
            h = max_h;
        }
        let (cx, cy) = (u64::from(w) * 635, u64::from(h) * 635);
        self.media.push(Media { name: name.clone(), bytes });
        let rid = format!("rIdImg{n}");
        Some(format!(
            "<w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">\
<wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{n}\" name=\"Picture {n}\"/>\
<a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">\
<a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:pic xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">\
<pic:nvPicPr><pic:cNvPr id=\"{n}\" name=\"{name}\"/><pic:cNvPicPr/></pic:nvPicPr>\
<pic:blipFill><a:blip r:embed=\"{rid}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>\
<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic>\
</wp:inline></w:drawing></w:r>"
        ))
    }

    fn rubric_props(&self) -> String {
        match self.page.rubric_color {
            Some(c) => format!("<w:color w:val=\"{c}\"/>"),
            None => String::new(),
        }
    }

    fn piece(&mut self, p: &Piece) {
        let page = self.page;
        match p {
            Piece::Section { text } => self.para("Section", "", &text_run(text, "")),
            Piece::Line { title, detail, incipit, reference } => {
                let mut x = text_run(title, "<w:b/>");
                if let Some(d) = detail {
                    x.push_str(&text_run(&format!("  {d}"), ""));
                }
                if let Some(i) = incipit {
                    x.push_str(&text_run(&format!("  {i}"), "<w:i/>"));
                }
                if let Some(r) = reference {
                    x.push_str("<w:r><w:tab/></w:r>");
                    x.push_str(&text_run(r, ""));
                }
                let tabs = format!("<w:tabs><w:tab w:val=\"right\" w:pos=\"{}\"/></w:tabs>", page.text);
                self.para("ItemLine", &tabs, &x);
            }
            Piece::Rubric { text, .. } => {
                let props = self.rubric_props();
                self.para("Rubric", "", &text_run(text, &props));
            }
            Piece::Speech { label, lines } => {
                let mut x = String::new();
                if let Some(l) = label.as_deref().filter(|l| !l.is_empty()) {
                    x.push_str(&text_run(l, "<w:i/>"));
                }
                x.push_str("<w:r><w:tab/></w:r>");
                x.push_str(&lines_xml(lines, ""));
                let ind = format!(
                    "<w:tabs><w:tab w:val=\"left\" w:pos=\"{h}\"/></w:tabs><w:ind w:left=\"{h}\" w:hanging=\"{h}\"/>",
                    h = page.hang
                );
                self.para("Speech", &ind, &x);
            }
            Piece::Verse { number, lines } => {
                let x = format!("{}<w:r><w:tab/></w:r>{}", text_run(number, ""), lines_xml(lines, ""));
                let h = page.hang / 2;
                let ind = format!(
                    "<w:tabs><w:tab w:val=\"left\" w:pos=\"{h}\"/></w:tabs><w:ind w:left=\"{h}\" w:hanging=\"{h}\"/>"
                );
                self.para("Verse", &ind, &x);
            }
            Piece::Prose { runs } => {
                let x: String = runs.iter().map(|r| run_xml(r, "")).collect();
                self.para("Prose", "", &x);
            }
            Piece::Image { path, .. } => {
                if let Some(x) = self.image(path, page.text, (page.height - 2 * page.margin) / 2) {
                    self.para("Picture", "", &x);
                }
            }
            Piece::MissingText { citation } => {
                let text = format!("[The text of {citation} goes here.]");
                self.para("Prose", "", &text_run(&text, "<w:i/><w:highlight w:val=\"yellow\"/>"));
            }
            Piece::PageBreak => self.page_break(),
        }
    }

    fn cover(&mut self, doc: &Doc) {
        if let Some(logo) = &doc.cover.logo {
            if let Some(x) = self.image(logo, self.page.text / 2, 2880) {
                self.para("Cover", "", &x);
            }
        }
        self.para("Title", "", &text_run(&doc.cover.title, ""));
        if let Some(d) = &doc.cover.day {
            self.para("Cover", "", &text_run(d, ""));
        }
        if let Some(w) = &doc.cover.when {
            self.para("Cover", "", &text_run(w, "<w:i/>"));
        }
        if doc.kind == DocKind::Bulletin {
            self.page_break();
        }
    }

    fn back(&mut self, p: &ParishInfo) {
        self.page_break();
        if !p.welcome.trim().is_empty() {
            self.para("Section", "", &text_run("To Our Visitors and Guests\u{2026}", ""));
            for para in p.welcome.split("\n\n").filter(|x| !x.trim().is_empty()) {
                self.para("Prose", "", &text_run(para.trim(), ""));
            }
        }
        if let Some(logo) = &p.logo {
            if let Some(x) = self.image(logo, self.page.text / 3, 1800) {
                self.para("Contact", "<w:spacing w:before=\"480\"/>", &x);
            }
        }
        if !p.name.trim().is_empty() {
            self.para("Contact", "", &text_run(p.name.trim(), "<w:b/>"));
        }
        for v in [&p.place, &p.address, &p.phone, &p.website, &p.email] {
            for line in v.lines().filter(|l| !l.trim().is_empty()) {
                self.para("Contact", "", &text_run(line.trim(), ""));
            }
        }
        if !p.licence.trim().is_empty() {
            self.para("Licence", "", &text_run(p.licence.trim(), ""));
        }
    }
}

fn styles(page: &Page) -> String {
    let s = page.size;
    let style = |id: &str, ppr: &str, rpr: &str| {
        format!(
            "<w:style w:type=\"paragraph\" w:customStyle=\"1\" w:styleId=\"{id}\"><w:name w:val=\"{id}\"/>\
<w:basedOn w:val=\"Normal\"/><w:qFormat/><w:pPr>{ppr}</w:pPr><w:rPr>{rpr}</w:rPr></w:style>"
        )
    };
    let rubric_color = page.rubric_color.map(|c| format!("<w:color w:val=\"{c}\"/>")).unwrap_or_default();
    let mut x = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"{f}\" w:hAnsi=\"{f}\" w:cs=\"{f}\" w:eastAsia=\"{f}\"/>\
<w:sz w:val=\"{s}\"/><w:szCs w:val=\"{s}\"/><w:lang w:val=\"en-US\"/></w:rPr></w:rPrDefault>\
<w:pPrDefault><w:pPr><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults>\
<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:qFormat/></w:style>",
        f = page.font
    );
    x.push_str(&style(
        "Title",
        "<w:jc w:val=\"center\"/><w:spacing w:before=\"1440\" w:after=\"240\"/>",
        &format!("<w:sz w:val=\"{0}\"/><w:szCs w:val=\"{0}\"/>", s * 2),
    ));
    x.push_str(&style(
        "Cover",
        "<w:jc w:val=\"center\"/><w:spacing w:after=\"120\"/>",
        &format!("<w:sz w:val=\"{0}\"/><w:szCs w:val=\"{0}\"/>", s + 4),
    ));
    x.push_str(&style(
        "Section",
        "<w:keepNext/><w:jc w:val=\"center\"/><w:spacing w:before=\"360\" w:after=\"120\"/>",
        &format!("<w:smallCaps/><w:sz w:val=\"{0}\"/><w:szCs w:val=\"{0}\"/>", s + 4),
    ));
    x.push_str(&style("ItemLine", "<w:keepNext/><w:spacing w:before=\"200\" w:after=\"60\"/>", ""));
    x.push_str(&style("Rubric", "<w:spacing w:before=\"60\" w:after=\"60\"/>", &format!("<w:i/>{rubric_color}")));
    x.push_str(&style("Speech", "<w:spacing w:after=\"80\"/>", ""));
    x.push_str(&style("Verse", "<w:spacing w:after=\"60\"/>", ""));
    x.push_str(&style("Prose", "<w:jc w:val=\"both\"/><w:spacing w:after=\"100\"/>", ""));
    x.push_str(&style("Picture", "<w:jc w:val=\"center\"/><w:spacing w:before=\"60\" w:after=\"60\"/>", ""));
    x.push_str(&style("Contact", "<w:jc w:val=\"center\"/>", ""));
    x.push_str(&style(
        "Licence",
        "<w:jc w:val=\"center\"/><w:spacing w:before=\"240\"/>",
        &format!("<w:sz w:val=\"{0}\"/><w:szCs w:val=\"{0}\"/>", s.saturating_sub(4)),
    ));
    x.push_str(&style("Footer", "<w:jc w:val=\"center\"/>", ""));
    x.push_str("</w:styles>");
    x
}

const NS: &str = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" \
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"";

/// The document as the bytes of a `.docx` file.
pub fn to_docx(doc: &Doc) -> Result<Vec<u8>, String> {
    let page = Page::for_kind(doc.kind);
    let mut w = Writer { page: &page, body: String::new(), media: Vec::new() };
    w.cover(doc);
    for e in &doc.entries {
        for p in &e.pieces {
            w.piece(p);
        }
    }
    if let Some(back) = &doc.back {
        w.back(back);
    }

    let orient = if page.landscape { " w:orient=\"landscape\"" } else { "" };
    let sect = format!(
        "<w:sectPr><w:footerReference w:type=\"default\" r:id=\"rIdFooter\"/>\
<w:pgSz w:w=\"{}\" w:h=\"{}\"{orient}/>\
<w:pgMar w:top=\"{m}\" w:right=\"{m}\" w:bottom=\"{m}\" w:left=\"{m}\" w:header=\"360\" w:footer=\"360\" w:gutter=\"0\"/>\
<w:titlePg/></w:sectPr>",
        page.width,
        page.height,
        m = page.margin
    );
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document {NS}><w:body>{}{sect}</w:body></w:document>",
        w.body
    );
    let footer = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:ftr {NS}><w:p><w:pPr><w:pStyle w:val=\"Footer\"/></w:pPr>\
<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> PAGE </w:instrText></w:r>\
<w:r><w:fldChar w:fldCharType=\"separate\"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType=\"end\"/></w:r></w:p></w:ftr>"
    );
    let book_fold = if doc.kind == DocKind::Bulletin { "<w:bookFoldPrinting/>" } else { "" };
    let settings = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:settings xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">{book_fold}\
<w:defaultTabStop w:val=\"720\"/><w:compat><w:compatSetting w:name=\"compatibilityMode\" \
w:uri=\"http://schemas.microsoft.com/office/word\" w:val=\"15\"/></w:compat></w:settings>"
    );
    let mut rels = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>\
<Relationship Id=\"rIdSettings\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings\" Target=\"settings.xml\"/>\
<Relationship Id=\"rIdFooter\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer\" Target=\"footer1.xml\"/>",
    );
    for (i, m) in w.media.iter().enumerate() {
        rels.push_str(&format!(
            "<Relationship Id=\"rIdImg{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"media/{}\"/>",
            i + 1,
            m.name
        ));
    }
    rels.push_str("</Relationships>");
    let content_types = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Default Extension=\"png\" ContentType=\"image/png\"/>\
<Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
<Override PartName=\"/word/settings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml\"/>\
<Override PartName=\"/word/footer1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml\"/>\
</Types>";
    let root_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
</Relationships>";

    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let err = |e: zip::result::ZipError| e.to_string();
    let mut add = |name: &str, bytes: &[u8]| -> Result<(), String> {
        zip.start_file(name, opts).map_err(err)?;
        zip.write_all(bytes).map_err(|e| e.to_string())
    };
    add("[Content_Types].xml", content_types.as_bytes())?;
    add("_rels/.rels", root_rels.as_bytes())?;
    add("word/document.xml", document.as_bytes())?;
    add("word/styles.xml", styles(&page).as_bytes())?;
    add("word/settings.xml", settings.as_bytes())?;
    add("word/footer1.xml", footer.as_bytes())?;
    add("word/_rels/document.xml.rels", rels.as_bytes())?;
    for m in &w.media {
        add(&format!("word/media/{}", m.name), &m.bytes)?;
    }
    Ok(zip.finish().map_err(err)?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_png_and_jpeg_sizes() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend(640u32.to_be_bytes());
        png.extend(120u32.to_be_bytes());
        assert_eq!(image_size(&png), Some((640, 120)));
        // SOI, APP0 (length 4), SOF0 with 100 × 50.
        let jpg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0, 0xFF, 0xC0, 0, 11, 8, 0, 50, 0, 100, 3, 0, 0, 0];
        assert_eq!(image_size(&jpg), Some((100, 50)));
        assert_eq!(image_size(b"GIF89a"), None);
    }
}
