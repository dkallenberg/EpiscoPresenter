//! Service element categories and the output layouts they select.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// What kind of service element a cue / liturgy file represents.
///
/// The category drives layout selection on both outputs (see
/// [`ElementCategory::stream_zone`] and [`ElementCategory::sanctuary_layout`])
/// and the bulletin filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementCategory {
    Announcements,
    Acclamation,
    Hymn,
    /// General prayers and Celebrant/People dialogues.
    #[default]
    Prayer,
    Collect,
    Reading,
    Psalm,
    Gospel,
    EucharisticPrayer,
    Homily,
    /// Choir anthems and other choral music.
    Anthem,
    Other,
}

impl ElementCategory {
    pub const ALL: [ElementCategory; 12] = [
        ElementCategory::Announcements,
        ElementCategory::Acclamation,
        ElementCategory::Hymn,
        ElementCategory::Prayer,
        ElementCategory::Collect,
        ElementCategory::Reading,
        ElementCategory::Psalm,
        ElementCategory::Gospel,
        ElementCategory::EucharisticPrayer,
        ElementCategory::Homily,
        ElementCategory::Anthem,
        ElementCategory::Other,
    ];

    /// The snake_case key used in `.ewm` front matter (`type: eucharistic_prayer`).
    pub fn key(self) -> &'static str {
        match self {
            ElementCategory::Announcements => "announcements",
            ElementCategory::Acclamation => "acclamation",
            ElementCategory::Hymn => "hymn",
            ElementCategory::Prayer => "prayer",
            ElementCategory::Collect => "collect",
            ElementCategory::Reading => "reading",
            ElementCategory::Psalm => "psalm",
            ElementCategory::Gospel => "gospel",
            ElementCategory::EucharisticPrayer => "eucharistic_prayer",
            ElementCategory::Homily => "homily",
            ElementCategory::Anthem => "anthem",
            ElementCategory::Other => "other",
        }
    }

    /// Overlay zone used on the live-stream output.
    pub fn stream_zone(self) -> StreamZone {
        match self {
            ElementCategory::Acclamation
            | ElementCategory::Prayer
            | ElementCategory::Collect
            | ElementCategory::Hymn => StreamZone::LeftPanel,
            ElementCategory::Psalm | ElementCategory::Gospel | ElementCategory::EucharisticPrayer => {
                StreamZone::BottomBar
            }
            ElementCategory::Reading
            | ElementCategory::Homily
            | ElementCategory::Anthem
            | ElementCategory::Announcements
            | ElementCategory::Other => StreamZone::LowerThird,
        }
    }

    /// Layout used on the sanctuary display.
    pub fn sanctuary_layout(self) -> SanctuaryLayout {
        match self {
            ElementCategory::Hymn => SanctuaryLayout::LyricsOnBlack,
            _ => SanctuaryLayout::SeasonalBackground,
        }
    }
}

impl fmt::Display for ElementCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

impl FromStr for ElementCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key: String =
            s.trim().to_ascii_lowercase().chars().map(|c| if c == ' ' || c == '-' { '_' } else { c }).collect();
        Ok(match key.as_str() {
            "announcements" | "announcement" => ElementCategory::Announcements,
            "acclamation" | "opening_acclamation" => ElementCategory::Acclamation,
            "hymn" | "song" => ElementCategory::Hymn,
            "prayer" | "prayers" | "dialogue" | "liturgy" => ElementCategory::Prayer,
            "collect" => ElementCategory::Collect,
            "reading" | "lesson" | "epistle" => ElementCategory::Reading,
            "psalm" => ElementCategory::Psalm,
            "gospel" => ElementCategory::Gospel,
            "eucharistic_prayer" | "great_thanksgiving" => ElementCategory::EucharisticPrayer,
            "homily" | "sermon" => ElementCategory::Homily,
            "anthem" | "choir" => ElementCategory::Anthem,
            "other" => ElementCategory::Other,
            _ => return Err(format!("unknown element type `{s}`")),
        })
    }
}

/// Live-stream overlay zone (keeps the camera feed clear).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamZone {
    /// Text restricted to the left 45% of the canvas; right 55% left open.
    LeftPanel,
    /// Full-width bar across the bottom 25-30% of the canvas.
    BottomBar,
    /// Minimal lower-third citation / info badge.
    LowerThird,
}

/// Sanctuary display layout family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SanctuaryLayout {
    /// Centered text on the seasonal stained-glass background.
    SeasonalBackground,
    /// Centered white text on solid black (hymn lyrics).
    LyricsOnBlack,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_follow_spec() {
        assert_eq!(ElementCategory::Prayer.stream_zone(), StreamZone::LeftPanel);
        assert_eq!(ElementCategory::Hymn.stream_zone(), StreamZone::LeftPanel);
        assert_eq!(ElementCategory::Psalm.stream_zone(), StreamZone::BottomBar);
        assert_eq!(ElementCategory::EucharisticPrayer.stream_zone(), StreamZone::BottomBar);
        assert_eq!(ElementCategory::Reading.stream_zone(), StreamZone::LowerThird);
        assert_eq!(ElementCategory::Anthem.stream_zone(), StreamZone::LowerThird);
        assert_eq!(ElementCategory::Hymn.sanctuary_layout(), SanctuaryLayout::LyricsOnBlack);
    }

    #[test]
    fn parses_keys_and_aliases() {
        for c in ElementCategory::ALL {
            assert_eq!(c.key().parse(), Ok(c));
        }
        assert_eq!("Great Thanksgiving".parse(), Ok(ElementCategory::EucharisticPrayer));
        assert_eq!("sermon".parse(), Ok(ElementCategory::Homily));
    }
}
