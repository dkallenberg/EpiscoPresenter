//! Slide templates: where each piece of a slide goes, and its font.
//!
//! Every *slide type* (`stream.left_panel`, `sanctuary.psalm`, …) has a set of
//! named *elements* — the text box, the title, the reference, the gold tag,
//! the music image and so on — each a rectangle on the 1920×1080 stage with
//! a font, size, colour and alignment. The built-in defaults live with the
//! renderer (`ui/js/templates.js`); a [`Templates`] map holds only what the
//! operator changed in the Layout tool, so a missing value means "default".
//!
//! Templates are app-wide (saved in the app's data folder) and travel inside
//! the [`crate::Service`] snapshot so that every node renders the same way.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Slide type key (e.g. `"stream.bottom_bar"`) → the operator's changes.
pub type Templates = BTreeMap<String, SlideTemplate>;

/// The operator's changes to one slide type.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SlideTemplate {
    /// Element name (`text`, `title`, `tag`, …) → changed properties.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub elements: BTreeMap<String, ElementStyle>,
    /// Lines of text a slide holds (default: what the text box holds at its
    /// type size). Longer slides are split; see [`crate::fit`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_lines: Option<u32>,
    /// `keep`: each line of the text on its own line (the Prayer Book's
    /// sense lines); `join`: a paragraph's lines run together.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_breaks: Option<String>,
    /// Lines too long for the box carry on indented under themselves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hanging: Option<bool>,
    /// Psalms: every other verse is indented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternate_verses: Option<bool>,
}

impl SlideTemplate {
    pub fn is_empty(&self) -> bool {
        self.elements.values().all(ElementStyle::is_empty)
            && self.max_lines.is_none()
            && self.line_breaks.is_none()
            && self.hanging.is_none()
            && self.alternate_verses.is_none()
    }
}

/// One element's position and type style. `None` keeps the default.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ElementStyle {
    /// Left edge, stage pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f32>,
    /// Top edge, stage pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f32>,
    /// Width, stage pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<f32>,
    /// Height, stage pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<f32>,
    /// CSS font family, e.g. `Tahoma` or `"Gill Sans", sans-serif`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// Font size in stage pixels (text shrinks below this to fit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    /// CSS colour, e.g. `#ffffff`. `None` follows the look's text tone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// `left`, `center` or `right`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    /// `top`, `center` or `bottom`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valign: Option<String>,
    /// Hide this element on this slide type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

impl ElementStyle {
    pub fn is_empty(&self) -> bool {
        *self == ElementStyle::default()
    }
}

/// Drops empty elements and templates, so saved files stay small.
pub fn prune(templates: &mut Templates) {
    for t in templates.values_mut() {
        t.elements.retain(|_, e| !e.is_empty());
    }
    templates.retain(|_, t| !t.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_prunes() {
        let mut t = Templates::new();
        let mut slide = SlideTemplate::default();
        slide.elements.insert(
            "title".into(),
            ElementStyle { y: Some(40.0), size: Some(60.0), color: Some("#fff".into()), ..Default::default() },
        );
        slide.elements.insert("text".into(), ElementStyle::default());
        t.insert("stream.left_panel".into(), slide);
        t.insert("sanctuary.title".into(), SlideTemplate::default());
        prune(&mut t);
        assert_eq!(t.len(), 1);
        assert_eq!(t["stream.left_panel"].elements.len(), 1);
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r##"{"stream.left_panel":{"elements":{"title":{"y":40.0,"size":60.0,"color":"#fff"}}}}"##);
        let back: Templates = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }
}
