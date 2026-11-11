//! The church year: names, seasons and colours for a date (Episcopal Church,
//! 1979 BCP with the Revised Common Lectionary).
//!
//! Used to title scheduled services ("The Twenty-Fourth Sunday after
//! Pentecost"), pick the season's colour and backgrounds, and choose seasonal
//! texts (e.g. the Easter opening acclamation).
//!
//! Dates are plain proleptic-Gregorian civil dates; no time zone is involved.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::season::{LiturgicalColor, Season};

/// A calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Date {
        Date { year, month, day }
    }

    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    pub fn to_days(self) -> i64 {
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    }

    pub fn from_days(z: i64) -> Date {
        let z = z + 719468;
        let era = z.div_euclid(146097);
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        Date { year, month, day }
    }

    pub fn add_days(self, n: i64) -> Date {
        Date::from_days(self.to_days() + n)
    }

    /// 0 = Sunday … 6 = Saturday.
    pub fn weekday(self) -> u32 {
        (self.to_days() + 4).rem_euclid(7) as u32
    }

    pub fn is_sunday(self) -> bool {
        self.weekday() == 0
    }

    /// The first Sunday on or after this date.
    pub fn next_sunday(self) -> Date {
        self.add_days(i64::from((7 - self.weekday()) % 7))
    }

    pub fn days_until(self, other: Date) -> i64 {
        other.to_days() - self.to_days()
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for Date {
    type Err = String;

    fn from_str(s: &str) -> Result<Date, String> {
        let bad = || format!("expected a date like 2026-11-08, got `{s}`");
        let mut it = s.trim().splitn(3, '-');
        let year = it.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
        let month: u32 = it.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
        let day: u32 = it.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
        let d = Date::new(year, month, day);
        // Reject impossible dates such as 2026-02-30.
        if !(1..=12).contains(&month) || Date::from_days(d.to_days()) != d {
            return Err(bad());
        }
        Ok(d)
    }
}

/// Easter Day (Anonymous Gregorian algorithm).
pub fn easter(year: i32) -> Date {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    Date::new(year, month as u32, day as u32)
}

/// The First Sunday of Advent: the Sunday from November 27 to December 3.
pub fn advent_sunday(year: i32) -> Date {
    Date::new(year, 11, 27).next_sunday()
}

/// What a day is in the church year.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiturgicalDay {
    /// e.g. "The Twenty-Fourth Sunday after Pentecost".
    pub name: String,
    pub season: Season,
    pub color: LiturgicalColor,
    /// Revised Common Lectionary year: 'A', 'B' or 'C'.
    pub year: char,
    /// Proper number (season after Pentecost), e.g. 27.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proper: Option<u8>,
}

impl LiturgicalDay {
    /// "Year A · Proper 27".
    pub fn short_info(&self) -> String {
        match self.proper {
            Some(p) => format!("Year {} · Proper {p}", self.year),
            None => format!("Year {}", self.year),
        }
    }
}

const ORDINALS: [&str; 30] = [
    "",
    "First",
    "Second",
    "Third",
    "Fourth",
    "Fifth",
    "Sixth",
    "Seventh",
    "Eighth",
    "Ninth",
    "Tenth",
    "Eleventh",
    "Twelfth",
    "Thirteenth",
    "Fourteenth",
    "Fifteenth",
    "Sixteenth",
    "Seventeenth",
    "Eighteenth",
    "Nineteenth",
    "Twentieth",
    "Twenty-First",
    "Twenty-Second",
    "Twenty-Third",
    "Twenty-Fourth",
    "Twenty-Fifth",
    "Twenty-Sixth",
    "Twenty-Seventh",
    "Twenty-Eighth",
    "Twenty-Ninth",
];

fn ordinal(n: i64) -> &'static str {
    ORDINALS.get(n as usize).copied().unwrap_or("")
}

const WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// The lectionary year letter for a date (years begin at Advent; Advent 2022
/// began Year A).
pub fn lectionary_year(date: Date) -> char {
    let start = if date >= advent_sunday(date.year) { date.year } else { date.year - 1 };
    ['A', 'B', 'C'][(start - 2022).rem_euclid(3) as usize]
}

fn day(name: impl Into<String>, season: Season, color: LiturgicalColor, date: Date) -> LiturgicalDay {
    LiturgicalDay { name: name.into(), season, color, year: lectionary_year(date), proper: None }
}

/// Names a date in the church year. Sundays and the principal feasts get
/// their proper names; other days are "Wednesday in Lent" and the like.
pub fn liturgical_day(date: Date) -> LiturgicalDay {
    use LiturgicalColor as C;
    use Season as S;
    let y = date.year;
    let advent = advent_sunday(y);
    let christmas = Date::new(y, 12, 25);
    let east = easter(y);
    let ash = east.add_days(-46);
    let pentecost = east.add_days(49);
    let epiphany = Date::new(y, 1, 6);
    let sunday = date.is_sunday();

    // ---- fixed and moveable feasts -------------------------------------
    let feast = |name: &str, season, color| Some(day(name, season, color, date));
    let named = match (date.month, date.day) {
        // On a Sunday, the morning of December 24 is still the Fourth Sunday of Advent.
        (12, 24) if !sunday => feast("Christmas Eve", S::Christmas, C::White),
        (12, 25) => feast("Christmas Day", S::Christmas, C::White),
        (1, 1) => feast("The Holy Name of Our Lord Jesus Christ", S::Christmas, C::White),
        (1, 6) => feast("The Epiphany", S::Epiphany, C::White),
        (11, 1) => feast("All Saints' Day", S::Pentecost, C::White),
        _ => None,
    }
    .or_else(|| match east.days_until(date) {
        -46 => feast("Ash Wednesday", S::Lent, C::Violet),
        -7 => feast("The Sunday of the Passion: Palm Sunday", S::HolyWeek, C::Red),
        -3 => feast("Maundy Thursday", S::HolyWeek, C::Red),
        -2 => feast("Good Friday", S::HolyWeek, C::Black),
        -1 => feast("Holy Saturday", S::HolyWeek, C::Black),
        0 => feast("Easter Day", S::Easter, C::White),
        39 => feast("Ascension Day", S::Easter, C::White),
        49 => feast("The Day of Pentecost", S::Pentecost, C::Red),
        56 => feast("Trinity Sunday", S::Pentecost, C::White),
        _ => None,
    });
    if let Some(d) = named {
        return d;
    }

    // ---- seasons -------------------------------------------------------
    let weekday = WEEKDAYS[date.weekday() as usize];
    if date >= advent && date < christmas {
        let n = advent.days_until(date) / 7 + 1;
        let name = if sunday { format!("The {} Sunday of Advent", ordinal(n)) } else { format!("{weekday} in Advent") };
        return day(name, S::Advent, C::Blue, date);
    }
    if date > christmas || date < epiphany {
        // Christmas season (Dec 26 – Jan 5).
        let name = if sunday {
            let first = Date::new(if date.month == 12 { y } else { y - 1 }, 12, 26).next_sunday();
            let n = first.days_until(date) / 7 + 1;
            format!("The {} Sunday after Christmas Day", ordinal(n))
        } else {
            format!("{weekday} in Christmastide")
        };
        return day(name, S::Christmas, C::White, date);
    }
    if date > epiphany && date < ash {
        if !sunday {
            return day(format!("{weekday} after the Epiphany"), S::Epiphany, C::Green, date);
        }
        let first = epiphany.add_days(1).next_sunday();
        if date == first {
            return day("The First Sunday after the Epiphany: The Baptism of Our Lord", S::Epiphany, C::White, date);
        }
        if date == ash.add_days(-3) {
            return day("The Last Sunday after the Epiphany", S::Epiphany, C::White, date);
        }
        let n = first.days_until(date) / 7 + 1;
        return day(format!("The {} Sunday after the Epiphany", ordinal(n)), S::Epiphany, C::Green, date);
    }
    if date > ash && date < east {
        if date >= east.add_days(-7) {
            return day(format!("{weekday} in Holy Week"), S::HolyWeek, C::Red, date);
        }
        let name = if sunday {
            let n = ash.add_days(1).next_sunday().days_until(date) / 7 + 1;
            format!("The {} Sunday in Lent", ordinal(n))
        } else {
            format!("{weekday} in Lent")
        };
        return day(name, S::Lent, C::Violet, date);
    }
    if date > east && date < pentecost {
        let name = if sunday {
            format!("The {} Sunday of Easter", ordinal(east.days_until(date) / 7 + 1))
        } else {
            format!("{weekday} in Eastertide")
        };
        return day(name, S::Easter, C::White, date);
    }

    // Season after Pentecost (until Advent).
    let mut d = day(format!("{weekday} after Pentecost"), S::Pentecost, C::Green, date);
    // Proper n is the Sunday closest to May 11 + 7(n - 1).
    let sun = if sunday { date } else { date.add_days(-i64::from(date.weekday())) };
    let p = (Date::new(y, 5, 11).days_until(sun) as f64 / 7.0).round() as i64 + 1;
    if (1..=29).contains(&p) {
        d.proper = Some(p as u8);
    }
    if sunday {
        if date == advent.add_days(-7) {
            d.name = "The Last Sunday after Pentecost: Christ the King".into();
            d.color = C::White;
        } else {
            d.name = format!("The {} Sunday after Pentecost", ordinal(pentecost.days_until(date) / 7));
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    #[test]
    fn dates_round_trip_and_weekdays() {
        for s in ["1970-01-01", "2000-02-29", "2026-11-08", "2027-03-28", "1900-03-01"] {
            assert_eq!(Date::from_days(d(s).to_days()).to_string(), s);
        }
        assert_eq!(d("2026-11-08").weekday(), 0);
        assert_eq!(d("2026-11-11").next_sunday(), d("2026-11-15"));
        assert!("2026-02-30".parse::<Date>().is_err());
    }

    #[test]
    fn easter_dates() {
        for (y, e) in [
            (2024, "2024-03-31"),
            (2025, "2025-04-20"),
            (2026, "2026-04-05"),
            (2027, "2027-03-28"),
            (2038, "2038-04-25"),
        ] {
            assert_eq!(easter(y), d(e));
        }
    }

    #[test]
    fn the_sample_service() {
        let day = liturgical_day(d("2026-11-08"));
        assert_eq!(day.name, "The Twenty-Fourth Sunday after Pentecost");
        assert_eq!(day.proper, Some(27));
        assert_eq!(day.year, 'A');
        assert_eq!(day.color, LiturgicalColor::Green);
        assert_eq!(day.short_info(), "Year A · Proper 27");
    }

    #[test]
    fn names_across_a_year() {
        let cases = [
            ("2026-11-22", "The Last Sunday after Pentecost: Christ the King"),
            ("2026-11-29", "The First Sunday of Advent"),
            ("2026-12-20", "The Fourth Sunday of Advent"),
            ("2026-12-25", "Christmas Day"),
            ("2028-12-24", "The Fourth Sunday of Advent"),
            ("2026-12-24", "Christmas Eve"),
            ("2026-12-27", "The First Sunday after Christmas Day"),
            ("2027-01-03", "The Second Sunday after Christmas Day"),
            ("2027-01-06", "The Epiphany"),
            ("2027-01-10", "The First Sunday after the Epiphany: The Baptism of Our Lord"),
            ("2027-01-17", "The Second Sunday after the Epiphany"),
            ("2027-02-07", "The Last Sunday after the Epiphany"),
            ("2027-02-10", "Ash Wednesday"),
            ("2027-02-14", "The First Sunday in Lent"),
            ("2027-03-14", "The Fifth Sunday in Lent"),
            ("2027-03-21", "The Sunday of the Passion: Palm Sunday"),
            ("2027-03-25", "Maundy Thursday"),
            ("2027-03-28", "Easter Day"),
            ("2027-04-04", "The Second Sunday of Easter"),
            ("2027-05-09", "The Seventh Sunday of Easter"),
            ("2027-05-16", "The Day of Pentecost"),
            ("2027-05-23", "Trinity Sunday"),
            ("2027-05-30", "The Second Sunday after Pentecost"),
        ];
        for (date, name) in cases {
            assert_eq!(liturgical_day(d(date)).name, name, "{date}");
        }
    }

    #[test]
    fn lectionary_years_and_seasons() {
        assert_eq!(lectionary_year(d("2026-11-22")), 'A');
        assert_eq!(lectionary_year(d("2026-11-29")), 'B');
        assert_eq!(lectionary_year(d("2027-12-05")), 'C');
        assert_eq!(liturgical_day(d("2027-03-01")).season, Season::Lent);
        assert_eq!(liturgical_day(d("2027-03-01")).name, "Monday in Lent");
        assert_eq!(liturgical_day(d("2027-05-30")).proper, Some(4));
        assert_eq!(liturgical_day(d("2026-11-22")).proper, Some(29));
    }
}
