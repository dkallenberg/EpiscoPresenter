//! Service element categories (what kind of item a cue is).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// What kind of service element a cue / liturgy file represents.
///
/// The category picks the default look on both outputs
/// ([`crate::look::default_look`]) and drives the bulletin filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keys_and_aliases() {
        for c in ElementCategory::ALL {
            assert_eq!(c.key().parse(), Ok(c));
        }
        assert_eq!("Great Thanksgiving".parse(), Ok(ElementCategory::EucharisticPrayer));
        assert_eq!("sermon".parse(), Ok(ElementCategory::Homily));
    }
}
