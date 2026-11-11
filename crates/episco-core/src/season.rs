//! Liturgical seasons and colours.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A season of the Episcopal church year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Season {
    Advent,
    Christmas,
    Epiphany,
    Lent,
    HolyWeek,
    Easter,
    /// The Day of Pentecost and the Season after Pentecost.
    Pentecost,
}

impl Season {
    pub const ALL: [Season; 7] = [
        Season::Advent,
        Season::Christmas,
        Season::Epiphany,
        Season::Lent,
        Season::HolyWeek,
        Season::Easter,
        Season::Pentecost,
    ];

    /// The customary colour for the season, used when a file omits `color:`.
    pub fn default_color(self) -> LiturgicalColor {
        match self {
            Season::Advent => LiturgicalColor::Blue,
            Season::Christmas | Season::Easter => LiturgicalColor::White,
            Season::Epiphany | Season::Pentecost => LiturgicalColor::Green,
            Season::Lent => LiturgicalColor::Violet,
            Season::HolyWeek => LiturgicalColor::Red,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Season::Advent => "Advent",
            Season::Christmas => "Christmas",
            Season::Epiphany => "Epiphany",
            Season::Lent => "Lent",
            Season::HolyWeek => "Holy Week",
            Season::Easter => "Easter",
            Season::Pentecost => "Pentecost",
        }
    }
}

impl fmt::Display for Season {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for Season {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();
        Ok(match key.as_str() {
            "advent" => Season::Advent,
            "christmas" | "christmastide" => Season::Christmas,
            "epiphany" | "afterepiphany" | "seasonafterepiphany" => Season::Epiphany,
            "lent" => Season::Lent,
            "holyweek" => Season::HolyWeek,
            "easter" | "eastertide" => Season::Easter,
            "pentecost" | "afterpentecost" | "seasonafterpentecost" | "ordinarytime" => Season::Pentecost,
            _ => return Err(format!("unknown liturgical season `{s}`")),
        })
    }
}

/// A liturgical colour; selects the stained-glass background family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LiturgicalColor {
    Green,
    Violet,
    White,
    Red,
    Blue,
    Black,
}

impl LiturgicalColor {
    pub const ALL: [LiturgicalColor; 6] = [
        LiturgicalColor::Green,
        LiturgicalColor::Violet,
        LiturgicalColor::White,
        LiturgicalColor::Red,
        LiturgicalColor::Blue,
        LiturgicalColor::Black,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LiturgicalColor::Green => "Green",
            LiturgicalColor::Violet => "Violet",
            LiturgicalColor::White => "White",
            LiturgicalColor::Red => "Red",
            LiturgicalColor::Blue => "Blue",
            LiturgicalColor::Black => "Black",
        }
    }

    /// Default stained-glass background image file for this colour,
    /// e.g. `green_stained_glass.png`.
    pub fn default_background(self) -> String {
        format!("{}_stained_glass.png", self.label().to_ascii_lowercase())
    }

    /// A representative swatch for UI status icons.
    pub fn swatch(self) -> &'static str {
        match self {
            LiturgicalColor::Green => "#2e7d32",
            LiturgicalColor::Violet => "#5e2a84",
            LiturgicalColor::White => "#f5f1e6",
            LiturgicalColor::Red => "#b71c1c",
            LiturgicalColor::Blue => "#1f4e9c",
            LiturgicalColor::Black => "#111111",
        }
    }
}

impl fmt::Display for LiturgicalColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for LiturgicalColor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_ascii_lowercase().as_str() {
            "green" => LiturgicalColor::Green,
            "violet" | "purple" => LiturgicalColor::Violet,
            "white" | "gold" => LiturgicalColor::White,
            "red" => LiturgicalColor::Red,
            "blue" => LiturgicalColor::Blue,
            "black" => LiturgicalColor::Black,
            _ => return Err(format!("unknown liturgical color `{s}`")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_season_variants() {
        assert_eq!("Pentecost".parse(), Ok(Season::Pentecost));
        assert_eq!("Season after Pentecost".parse(), Ok(Season::Pentecost));
        assert_eq!("holy week".parse(), Ok(Season::HolyWeek));
        assert!("Summer".parse::<Season>().is_err());
    }

    #[test]
    fn colors_and_backgrounds() {
        assert_eq!("Purple".parse(), Ok(LiturgicalColor::Violet));
        assert_eq!(LiturgicalColor::Green.default_background(), "green_stained_glass.png");
        assert_eq!(Season::Lent.default_color(), LiturgicalColor::Violet);
    }
}
