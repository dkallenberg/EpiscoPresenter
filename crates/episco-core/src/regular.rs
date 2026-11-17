//! The parish's regular services ("8:00 on Sunday, Rite I, said; closing
//! hymn and postlude only"), set once in Parish details. Each one gives a
//! new service its time and defaults: the rite, the music places it uses
//! and the bulletin paper. All but the day and time can still be changed
//! for one service when it is made.

use serde::{Deserialize, Serialize};

use crate::bcp::Rite;
use crate::bulletin::BulletinPaper;

/// A day of the week.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    #[default]
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Sunday,
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
    ];

    /// 0 = Sunday … 6 = Saturday, as [`crate::calendar::Date::weekday`].
    pub fn number(self) -> u32 {
        Weekday::ALL.iter().position(|d| *d == self).unwrap_or(0) as u32
    }

    /// The first date on or after `from` that falls on this day.
    pub fn on_or_after(self, from: crate::calendar::Date) -> crate::calendar::Date {
        from.add_days(i64::from((7 + self.number() - from.weekday()) % 7))
    }
}

/// A place in the service for music.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MusicPlace {
    Prelude,
    EntranceHymn,
    /// The Gloria (or Kyrie) sung; unticked, it is said.
    HymnOfPraise,
    /// A hymn after the Epistle.
    Gradual,
    /// A hymn just before the Gospel.
    Sequence,
    OffertoryAnthem,
    OffertoryHymn,
    CommunionHymn,
    ClosingHymn,
    Postlude,
}

impl MusicPlace {
    pub const ALL: [MusicPlace; 10] = [
        MusicPlace::Prelude,
        MusicPlace::EntranceHymn,
        MusicPlace::HymnOfPraise,
        MusicPlace::Gradual,
        MusicPlace::Sequence,
        MusicPlace::OffertoryAnthem,
        MusicPlace::OffertoryHymn,
        MusicPlace::CommunionHymn,
        MusicPlace::ClosingHymn,
        MusicPlace::Postlude,
    ];

    /// The places the standard order has.
    pub const USUAL: [MusicPlace; 7] = [
        MusicPlace::Prelude,
        MusicPlace::EntranceHymn,
        MusicPlace::HymnOfPraise,
        MusicPlace::OffertoryHymn,
        MusicPlace::CommunionHymn,
        MusicPlace::ClosingHymn,
        MusicPlace::Postlude,
    ];

    /// The item's name in the service.
    pub fn label(self) -> &'static str {
        match self {
            MusicPlace::Prelude => "Prelude",
            MusicPlace::EntranceHymn => "Entrance Hymn",
            MusicPlace::HymnOfPraise => "Hymn of Praise",
            MusicPlace::Gradual => "Gradual Hymn",
            MusicPlace::Sequence => "Sequence Hymn",
            MusicPlace::OffertoryAnthem => "Offertory Anthem",
            MusicPlace::OffertoryHymn => "Offertory Hymn",
            MusicPlace::CommunionHymn => "Communion Hymn",
            MusicPlace::ClosingHymn => "Closing Hymn",
            MusicPlace::Postlude => "Postlude",
        }
    }
}

fn usual_music() -> Vec<MusicPlace> {
    MusicPlace::USUAL.to_vec()
}

/// One of the parish's regular services.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RegularService {
    /// What the parish calls it ("Early service"); may be empty.
    pub name: String,
    pub day: Weekday,
    /// "08:00" (24-hour).
    pub time: String,
    pub rite: Rite,
    /// The music places it uses; empty: no music.
    pub music: Vec<MusicPlace>,
    /// Its bulletin's paper; `None`: the parish's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper: Option<BulletinPaper>,
}

impl Default for RegularService {
    fn default() -> Self {
        RegularService {
            name: String::new(),
            day: Weekday::Sunday,
            time: String::new(),
            rite: Rite::Two,
            music: usual_music(),
            paper: None,
        }
    }
}

impl RegularService {
    /// A Sunday service at `time` with the usual music (an older parish
    /// file's service times).
    pub fn sunday_at(time: &str) -> RegularService {
        RegularService { time: time.to_string(), ..Default::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::Date;

    #[test]
    fn days_and_defaults() {
        let wed = "2026-11-18".parse::<Date>().unwrap();
        assert_eq!(Weekday::Sunday.on_or_after(wed).to_string(), "2026-11-22");
        assert_eq!(Weekday::Wednesday.on_or_after(wed), wed);
        assert_eq!(Weekday::Tuesday.on_or_after(wed).to_string(), "2026-11-24");
        let r: RegularService =
            serde_json::from_str(r#"{"time": "08:00", "rite": "I", "music": ["closing_hymn", "postlude"]}"#).unwrap();
        assert_eq!((r.day, r.rite, r.music.len()), (Weekday::Sunday, Rite::One, 2));
        let usual: RegularService = serde_json::from_str(r#"{"time": "10:30"}"#).unwrap();
        assert_eq!(usual.music, MusicPlace::USUAL);
        let none: RegularService = serde_json::from_str(r#"{"time": "07:30", "music": []}"#).unwrap();
        assert!(none.music.is_empty(), "no music");
    }
}
