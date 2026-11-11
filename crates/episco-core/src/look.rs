//! How a cue looks on each output: background, text layout, text tone, fit.
//!
//! A [`Look`] is a partial specification (every field optional) so that looks
//! can be layered:
//!
//! 1. built-in default for the cue's [`ElementCategory`] ([`default_look`]),
//! 2. the service's default for that category (`[looks.<type>]` in the plan),
//! 3. the document's `default_background:` (sanctuary only),
//! 4. the cue's own override (`[cue.sanctuary]` / `[cue.stream]`).
//!
//! The result is a [`ResolvedLook`] with every field filled in.

use serde::{Deserialize, Serialize};

use crate::category::ElementCategory;

/// An output surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Output {
    /// The 85" sanctuary screen.
    Sanctuary,
    /// The live-stream graphics output.
    Stream,
}

impl Output {
    pub const ALL: [Output; 2] = [Output::Sanctuary, Output::Stream];
}

/// Where text is placed on the canvas. Each layout matches a family of
/// background designs (see `docs/backgrounds.md` for the exact text boxes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// Text block between top and bottom borders, role labels in a column.
    Centered,
    /// Centered lyrics, bold, no role labels (hymns on the sanctuary screen).
    Lyrics,
    /// Text in a panel on the left 3.5–46% of the canvas (stream prayers).
    LeftPanel,
    /// Text in a wider panel, 3.5–53% (stream hymns).
    LeftPanelWide,
    /// Text in a bar across the bottom 61–94% (stream psalms, Eucharistic Prayer).
    BottomBar,
    /// A framed bar at the bottom-left; everything else is key colour.
    LowerThird,
}

impl Layout {
    pub const ALL: [Layout; 6] = [
        Layout::Centered,
        Layout::Lyrics,
        Layout::LeftPanel,
        Layout::LeftPanelWide,
        Layout::BottomBar,
        Layout::LowerThird,
    ];

    /// Text colour that suits this layout's usual background.
    pub fn default_tone(self) -> TextTone {
        match self {
            Layout::Centered => TextTone::Dark,
            _ => TextTone::Light,
        }
    }
}

/// Text colour family: dark text for light backgrounds or vice versa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextTone {
    /// Dark grey / black text (for light backgrounds).
    Dark,
    /// Cream / white text (for dark backgrounds).
    Light,
}

/// How a background image is fitted to the 16:9 canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    /// Scale to cover the canvas, cropping if needed.
    #[default]
    Fill,
    /// Scale to fit inside the canvas, letterboxing if needed.
    Fit,
    /// Stretch to exactly the canvas size.
    Stretch,
}

/// A (partial) look. `None` fields inherit from the layer below.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Look {
    /// An image path (relative to the service plan's folder, or absolute), a
    /// built-in background (`builtin:glass-light`, …), a colour (`#000000`)
    /// or `key` for the stream key colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<TextTone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fit: Option<Fit>,
}

impl Look {
    pub fn is_empty(&self) -> bool {
        *self == Look::default()
    }

    /// `self` with every field that `over` sets replaced.
    ///
    /// A background and text tone belong to their layout's artwork, so when
    /// `over` switches to a different layout without naming a background or
    /// tone, those revert to the new layout's own defaults rather than
    /// carrying over (a lower third over a glass-bar background is never
    /// what anyone wants).
    pub fn layered(&self, over: &Look) -> Look {
        let layout_changed = over.layout.is_some() && over.layout != self.layout;
        let inherit = |mine: &Option<String>| if layout_changed { None } else { mine.clone() };
        Look {
            background: over.background.clone().or_else(|| inherit(&self.background)),
            layout: over.layout.or(self.layout),
            tone: over.tone.or(if layout_changed { None } else { self.tone }),
            fit: over.fit.or(self.fit),
        }
    }
}

/// Looks for both outputs (a cue override or a service default).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CueLooks {
    #[serde(default, skip_serializing_if = "Look::is_empty")]
    pub sanctuary: Look,
    #[serde(default, skip_serializing_if = "Look::is_empty")]
    pub stream: Look,
}

impl CueLooks {
    pub fn is_empty(&self) -> bool {
        self.sanctuary.is_empty() && self.stream.is_empty()
    }

    pub fn get(&self, output: Output) -> &Look {
        match output {
            Output::Sanctuary => &self.sanctuary,
            Output::Stream => &self.stream,
        }
    }

    pub fn get_mut(&mut self, output: Output) -> &mut Look {
        match output {
            Output::Sanctuary => &mut self.sanctuary,
            Output::Stream => &mut self.stream,
        }
    }
}

/// A fully specified look, ready to render.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedLook {
    pub background: String,
    pub layout: Layout,
    pub tone: TextTone,
    pub fit: Fit,
}

impl ResolvedLook {
    /// Fills any gaps in `look` from the layout's own defaults.
    pub fn complete(look: &Look) -> ResolvedLook {
        let layout = look.layout.unwrap_or(Layout::Centered);
        ResolvedLook {
            background: look.background.clone().unwrap_or_else(|| builtin_for(layout).to_string()),
            layout,
            tone: look.tone.unwrap_or_else(|| layout.default_tone()),
            fit: look.fit.unwrap_or_default(),
        }
    }
}

/// Built-in backgrounds, drawn by the renderer in the cue's liturgical colour.
pub const BUILTIN_BACKGROUNDS: [&str; 6] = [
    "builtin:glass-light",
    "builtin:glass-dark",
    "builtin:glass-charcoal",
    "builtin:panel-dark",
    "builtin:panel-light",
    "builtin:bar-bottom",
];

/// The stream key colour (configured per service; blue by default).
pub const KEY_BACKGROUND: &str = "key";
pub const DEFAULT_KEY_COLOR: &str = "#0000ff";

fn builtin_for(layout: Layout) -> &'static str {
    match layout {
        Layout::Centered => "builtin:glass-light",
        Layout::Lyrics => "#000000",
        Layout::LeftPanel => "builtin:panel-dark",
        Layout::LeftPanelWide => "builtin:panel-light",
        Layout::BottomBar => "builtin:bar-bottom",
        Layout::LowerThird => KEY_BACKGROUND,
    }
}

fn look(background: &str, layout: Layout, tone: TextTone) -> Look {
    Look { background: Some(background.into()), layout: Some(layout), tone: Some(tone), fit: Some(Fit::Fill) }
}

/// Built-in look for a category on an output.
pub fn default_look(category: ElementCategory, output: Output) -> Look {
    use ElementCategory as C;
    use Layout as L;
    use TextTone as T;
    match output {
        Output::Sanctuary => match category {
            C::Hymn => look("#000000", L::Lyrics, T::Light),
            C::Reading | C::Gospel | C::Homily | C::Anthem | C::Announcements => {
                look("builtin:glass-dark", L::Centered, T::Light)
            }
            C::Psalm => look("builtin:glass-charcoal", L::Centered, T::Light),
            C::Prayer | C::Collect | C::Acclamation | C::EucharisticPrayer | C::Other => {
                look("builtin:glass-light", L::Centered, T::Dark)
            }
        },
        Output::Stream => match category {
            C::Hymn => look("builtin:panel-light", L::LeftPanelWide, T::Dark),
            C::Prayer | C::Collect | C::Acclamation | C::Other => look("builtin:panel-dark", L::LeftPanel, T::Light),
            C::Psalm | C::EucharisticPrayer | C::Gospel => look("builtin:bar-bottom", L::BottomBar, T::Light),
            C::Reading | C::Homily | C::Anthem | C::Announcements => look(KEY_BACKGROUND, L::LowerThird, T::Light),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layering_overrides_only_set_fields() {
        let base = default_look(ElementCategory::Prayer, Output::Stream);
        let over = Look { background: Some("mine.png".into()), ..Default::default() };
        let r = ResolvedLook::complete(&base.layered(&over));
        assert_eq!(r.background, "mine.png");
        assert_eq!(r.layout, Layout::LeftPanel);
        assert_eq!(r.tone, TextTone::Light);
    }

    #[test]
    fn changing_layout_resets_background_and_tone() {
        let base = default_look(ElementCategory::Psalm, Output::Stream); // bar-bottom
        let r = ResolvedLook::complete(&base.layered(&Look { layout: Some(Layout::LowerThird), ..Default::default() }));
        assert_eq!(r.background, KEY_BACKGROUND);
        // …unless the same layer names them.
        let over = Look { layout: Some(Layout::LowerThird), background: Some("lt.png".into()), ..Default::default() };
        assert_eq!(ResolvedLook::complete(&base.layered(&over)).background, "lt.png");
        // Re-stating the same layout keeps the inherited background.
        let same = Look { layout: Some(Layout::BottomBar), ..Default::default() };
        assert_eq!(ResolvedLook::complete(&base.layered(&same)).background, "builtin:bar-bottom");
    }

    #[test]
    fn complete_fills_from_layout() {
        let r = ResolvedLook::complete(&Look { layout: Some(Layout::LowerThird), ..Default::default() });
        assert_eq!(r.background, KEY_BACKGROUND);
        assert_eq!(r.tone, TextTone::Light);
        assert_eq!(r.fit, Fit::Fill);
    }

    #[test]
    fn defaults_follow_the_sample_decks() {
        assert_eq!(default_look(ElementCategory::Hymn, Output::Sanctuary).layout, Some(Layout::Lyrics));
        assert_eq!(default_look(ElementCategory::Reading, Output::Stream).layout, Some(Layout::LowerThird));
        assert_eq!(default_look(ElementCategory::Psalm, Output::Stream).layout, Some(Layout::BottomBar));
        assert_eq!(default_look(ElementCategory::Collect, Output::Stream).layout, Some(Layout::LeftPanel));
    }

    #[test]
    fn toml_shape() {
        let looks: CueLooks =
            toml::from_str("[sanctuary]\nbackground = \"bg/prayer.png\"\n[stream]\nlayout = \"lower_third\"\n")
                .unwrap();
        assert_eq!(looks.sanctuary.background.as_deref(), Some("bg/prayer.png"));
        assert_eq!(looks.stream.layout, Some(Layout::LowerThird));
        assert!(toml::to_string(&CueLooks::default()).unwrap().trim().is_empty());
    }
}
