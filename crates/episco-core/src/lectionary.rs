//! The Sunday and holy-day propers for a date, from the parish's Revised
//! Common Lectionary reference (`library/lectionary/rcl.md`): Years A, B and
//! C with Tracks 1 and 2, the Principal Feasts and Holy Days, the Common of
//! Saints and the Lesser Feasts.
//!
//! [`for_date`] names the day ([`crate::calendar`]), applies the Prayer Book's
//! rules of precedence, and turns the day's entry into [`ProperItem`]s with
//! the Collect in both rites and the psalm's verses from the 1979 Psalter
//! ([`crate::bcp`]).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::bcp::{self, Rite};
use crate::calendar::{easter, liturgical_day, Date, LiturgicalDay};
use crate::propers::{ProperItem, ProperKind};
use crate::season::Season;

const RCL: &str = include_str!("../../../library/lectionary/rcl.md");

/// Which part of the reference an entry comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    /// Sundays and seasons of Year A, B or C.
    Year,
    /// Principal Feasts and Major Holy Days (fixed dates).
    HolyDay,
    /// The Common of Saints.
    Common,
    /// Lesser Feasts and Fasts.
    Minor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    Palms,
    First,
    Psalm,
    Second,
    Gospel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reading {
    pub slot: Slot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<u8>,
    /// Lectionary year for year-specific readings of a holy day (All Saints).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<char>,
    pub citation: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub section: Section,
    /// 'A', 'B' or 'C' for the Sundays; `None` for holy days.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<char>,
    pub name: String,
    /// (month, day) for fixed-date feasts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<(u32, u32)>,
    /// The collect's name as the reference gives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collect: Option<String>,
    pub readings: Vec<Reading>,
}

impl Entry {
    fn key(&self) -> String {
        name_key(&self.name)
    }
}

// ---------------------------------------------------------------------------
// Parsing the reference
// ---------------------------------------------------------------------------

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// "(November 30)", "January 10 — …", "Christmas Day (December 25)".
fn find_date(s: &str) -> Option<(u32, u32)> {
    let lower = s.to_lowercase();
    for (i, m) in MONTHS.iter().enumerate() {
        let mut from = 0;
        while let Some(at) = lower[from..].find(m) {
            let after = &lower[from + at + m.len()..];
            let digits: String = after.trim_start().chars().take_while(char::is_ascii_digit).collect();
            if let Ok(d) = digits.parse::<u32>() {
                if (1..=31).contains(&d) && after.starts_with(' ') {
                    return Some((i as u32 + 1, d));
                }
            }
            from += at + m.len();
        }
    }
    None
}

/// Lower-case words without "the", parenthetical notes or what follows a
/// dash: "The First Sunday after the Epiphany: The Baptism of Our Lord" →
/// "first sunday after epiphany baptism of our lord".
pub fn name_key(name: &str) -> String {
    let mut s = name.to_string();
    for dash in [" — ", " – ", " - "] {
        if let Some(i) = s.find(dash) {
            s.truncate(i);
        }
    }
    let mut out = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth > 0 => {}
            _ => out.push(c),
        }
    }
    out.to_lowercase()
        .replace(['\u{2019}', '\''], "")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && *w != "the")
        .collect::<Vec<_>>()
        .join(" ")
}

/// `X (or Y; or Z)` → (X, [Y, Z]). A bare alternative like `103:8-14`
/// takes the book of the main citation.
fn split_alternatives(value: &str) -> (String, Vec<String>) {
    let value = value.trim().trim_end_matches('*').trim();
    let (main, rest) = match value.find(" (or ") {
        Some(i) => (&value[..i], value[i + 2..].trim_end_matches(')')),
        None => (value, ""),
    };
    let main = main.trim().to_string();
    let book = main.rsplit_once(' ').map(|(b, _)| b).unwrap_or("").to_string();
    let alts = rest
        .split(';')
        .map(|a| a.trim().trim_start_matches("or ").trim())
        .filter(|a| !a.is_empty())
        .map(|a| {
            // "103:8-14" borrows the book; "2 Esdras 2:42-48" has its own.
            let has_book = a.split_whitespace().any(|w| w.chars().filter(|c| c.is_alphabetic()).count() >= 2);
            if !has_book && !book.is_empty() {
                format!("{book} {a}")
            } else {
                a.into()
            }
        })
        .collect();
    (main, alts)
}

/// `**Label:** value | **Label:** value` → [(label, value)].
fn labelled(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("**") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(":**") else { break };
        let label = after[..end].trim().to_string();
        let body = &after[end + 3..];
        let stop = body.find(" | **").or_else(|| body.find(" | *")).unwrap_or(body.len());
        out.push((label, body[..stop].trim().trim_end_matches('|').trim().to_string()));
        rest = &body[stop..];
    }
    out
}

fn slot_of(label: &str) -> Option<(Slot, Option<u8>)> {
    let l = label.to_lowercase();
    let track = if l.starts_with("track 1") {
        Some(1)
    } else if l.starts_with("track 2") {
        Some(2)
    } else {
        None
    };
    let slot = if l.contains("palms") {
        Slot::Palms
    } else if l.contains("first reading") {
        Slot::First
    } else if l.contains("psalm") {
        Slot::Psalm
    } else if l.contains("second reading") {
        Slot::Second
    } else if l.contains("gospel") {
        Slot::Gospel
    } else {
        return None;
    };
    Some((slot, track))
}

fn reading(slot: Slot, track: Option<u8>, year: Option<char>, value: &str) -> Reading {
    let (citation, alternatives) = split_alternatives(value);
    Reading { slot, track, year, citation, alternatives }
}

/// Parses the lectionary reference.
pub fn parse(src: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    // Entries that say "Same as Year A", resolved once everything is read.
    let mut same_as: Vec<(usize, String)> = Vec::new();
    let mut section = Section::Year;
    let mut year: Option<char> = None;
    for line in src.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            let h = h.to_uppercase();
            year = h.strip_prefix("YEAR ").and_then(|y| y.chars().next());
            section = if year.is_some() {
                Section::Year
            } else if h.contains("COMMON OF SAINTS") {
                Section::Common
            } else if h.contains("MINOR") || h.contains("LESSER") {
                Section::Minor
            } else {
                Section::HolyDay
            };
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("* **") {
            let Some(end) = rest.find("**") else { continue };
            let name = rest[..end].trim().to_string();
            let tail = &rest[end + 2..];
            let date = if section == Section::Year {
                find_date(&name).filter(|_| name.contains('('))
            } else {
                find_date(&name)
            };
            entries.push(Entry { section, year, name: name.clone(), date, collect: None, readings: Vec::new() });
            if tail.contains("Same") && tail.contains("Year A") {
                same_as.push((entries.len() - 1, name));
            }
            continue;
        }
        let Some(item) = line.strip_prefix("  * ") else { continue };
        let Some(entry) = entries.last_mut() else { continue };
        for (label, value) in labelled(item) {
            let l = label.to_lowercase();
            if l == "collect" {
                let v = value.split(" (BCP").next().unwrap_or(&value).trim();
                let v = v.trim_start_matches('*').trim_end_matches('*').trim();
                entry.collect = Some(v.to_string());
            } else if let Some(y) =
                l.strip_prefix("year ").and_then(|r| r.chars().next()).filter(|_| l.ends_with("readings"))
            {
                let y = y.to_ascii_uppercase();
                let slots = [Slot::First, Slot::Psalm, Slot::Second, Slot::Gospel];
                for (slot, v) in slots.iter().zip(value.split(" | ")) {
                    entry.readings.push(reading(*slot, None, Some(y), v));
                }
            } else if let Some((slot, track)) = slot_of(&label) {
                if slot == Slot::Palms {
                    // "Matthew 21:1-11; Psalm 118:1-2, 19-29"
                    let mut parts = value.split(';').map(str::trim);
                    if let Some(g) = parts.next() {
                        entry.readings.push(reading(Slot::Palms, None, None, g));
                    }
                    for p in parts {
                        entry.readings.push(Reading {
                            slot: Slot::Palms,
                            track: None,
                            year: None,
                            citation: p.to_string(),
                            alternatives: Vec::new(),
                        });
                    }
                } else {
                    entry.readings.push(reading(slot, track, None, &value));
                }
            }
        }
    }

    // "Same as Year A" (and "Maundy Thursday & Good Friday — Same as Year A").
    let mut extra = Vec::new();
    for (i, name) in same_as.into_iter().rev() {
        let target_year = entries[i].year;
        let parts: Vec<String> = name.split(" & ").map(name_key).collect();
        let first_part = parts.first().cloned().unwrap_or_default();
        let copies: Vec<Entry> = entries
            .iter()
            .filter(|e| e.year == Some('A') && e.section == Section::Year)
            .filter(|e| {
                let k = e.key();
                parts.iter().any(|p| {
                    k == *p
                        || (p.len() > 4 && k.starts_with(p.as_str()))
                        || (first_part == "christmas day" && k == "christmas day")
                })
            })
            .cloned()
            .map(|mut e| {
                e.year = target_year;
                e
            })
            .collect();
        if !copies.is_empty() {
            entries.remove(i);
            extra.push((i, copies));
        }
    }
    for (i, copies) in extra {
        for (k, e) in copies.into_iter().enumerate() {
            entries.insert(i + k, e);
        }
    }
    entries
}

/// Every entry in the reference.
pub fn entries() -> &'static Vec<Entry> {
    static E: OnceLock<Vec<Entry>> = OnceLock::new();
    E.get_or_init(|| parse(RCL))
}

// ---------------------------------------------------------------------------
// A date's propers
// ---------------------------------------------------------------------------

/// What can be observed on a day, and the propers of the chosen observance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayPropers {
    pub date: String,
    /// The church-year name of the date.
    pub day: LiturgicalDay,
    /// The observance used (an entry name), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observance: Option<String>,
    /// Observances that may be chosen instead, most likely first.
    pub observances: Vec<String>,
    /// Title for the Collect of the Day slide.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub items: Vec<ProperItem>,
}

/// Feasts of Our Lord that replace a Sunday after the Epiphany or after Pentecost.
fn feast_of_our_lord(name: &str) -> bool {
    let k = name_key(name);
    ["presentation", "transfiguration", "holy cross"].iter().any(|w| k.contains(w))
}

fn principal_fixed(name: &str) -> bool {
    let k = name_key(name);
    ["christmas day", "holy name", "epiphany", "all saints"].iter().any(|w| k.starts_with(w))
}

/// The entry key for a liturgical day.
fn day_key(day: &LiturgicalDay, date: Date) -> String {
    if date.is_sunday() && day.season == Season::Pentecost {
        if let Some(p) = day.proper {
            if !name_key(&day.name).starts_with("trinity") && !name_key(&day.name).contains("day of pentecost") {
                return format!("proper {p}");
            }
        }
    }
    name_key(&day.name)
}

fn year_entries(year: char, key: &str) -> Vec<&'static Entry> {
    let exact: Vec<&Entry> =
        entries().iter().filter(|e| e.section == Section::Year && e.year == Some(year) && e.key() == key).collect();
    if !exact.is_empty() {
        return exact;
    }
    // "holy name of our lord jesus christ" ↔ "holy name".
    entries()
        .iter()
        .filter(|e| e.section == Section::Year && e.year == Some(year))
        .filter(|e| {
            let k = e.key();
            k.len() > 6 && (key.starts_with(&k) || k.starts_with(key))
        })
        .collect()
}

fn find_entry(name: &str, year: char) -> Option<&'static Entry> {
    let key = name_key(name);
    entries()
        .iter()
        .filter(|e| e.year.is_none() || e.year == Some(year))
        .find(|e| e.name == name)
        .or_else(|| entries().iter().filter(|e| e.year.is_none() || e.year == Some(year)).find(|e| e.key() == key))
}

/// The observances possible on `date` (most likely first) for the
/// lectionary year.
pub fn observances(date: Date) -> (LiturgicalDay, Vec<&'static Entry>) {
    let day = liturgical_day(date);
    let year = day.year;
    let sunday = date.is_sunday();
    let mut list: Vec<&Entry> = Vec::new();

    let fixed: Vec<&Entry> = entries()
        .iter()
        .filter(|e| e.date == Some((date.month, date.day)) && (e.year.is_none() || e.year == Some(year)))
        .collect();
    let holy: Vec<&Entry> = fixed.iter().copied().filter(|e| e.section != Section::Minor).collect();
    let minor: Vec<&Entry> = fixed.iter().copied().filter(|e| e.section == Section::Minor).collect();
    let the_day = year_entries(year, &day_key(&day, date));

    // Holy Week and Easter Week displace everything.
    let east = easter(date.year);
    let holy_week = (-7..=7).contains(&east.days_until(date));
    let privileged_sunday =
        sunday && matches!(day.season, Season::Advent | Season::Lent | Season::Easter | Season::HolyWeek);

    let principal: Vec<&Entry> = holy.iter().copied().filter(|e| principal_fixed(&e.name)).collect();
    let lord: Vec<&Entry> = holy
        .iter()
        .copied()
        .filter(|e| feast_of_our_lord(&e.name) && sunday && matches!(day.season, Season::Epiphany | Season::Pentecost))
        .collect();

    if !principal.is_empty() && !holy_week {
        list.extend(principal.iter().copied());
        list.extend(the_day.iter().copied());
    } else if !lord.is_empty() {
        list.extend(lord.iter().copied());
        list.extend(the_day.iter().copied());
    } else if sunday || holy_week || privileged_sunday || !the_day.is_empty() {
        list.extend(the_day.iter().copied());
        if !holy_week {
            list.extend(holy.iter().copied());
        }
    } else {
        list.extend(holy.iter().copied());
        list.extend(the_day.iter().copied());
    }
    // All Saints' Day may be kept on the Sunday after November 1.
    if sunday && date.month == 11 && (2..=7).contains(&date.day) {
        if let Some(e) =
            entries().iter().find(|e| e.section == Section::HolyDay && name_key(&e.name).starts_with("all saints"))
        {
            list.push(e);
        }
    }
    list.extend(minor);
    list.extend(entries().iter().filter(|e| e.section == Section::Common));
    let mut seen = Vec::new();
    list.retain(|e| {
        let k = (e.name.clone(), e.year);
        let new = !seen.contains(&k);
        seen.push(k);
        new
    });
    (day, list)
}

/// "Common IV" → "Of a Theologian and Teacher".
fn common_collect(name: &str) -> Option<String> {
    let roman = ["I", "II", "III", "IV", "V", "VI"];
    let titles =
        ["Of a Martyr", "Of a Missionary", "Of a Pastor", "Of a Theologian and Teacher", "Of a Monastic", "Of a Saint"];
    let i = name.find("Common ")?;
    let r: String = name[i + 7..].chars().take_while(|c| c.is_ascii_uppercase()).collect();
    roman.iter().position(|x| *x == r).map(|k| titles[k].to_string())
}

fn kind_of(slot: Slot) -> ProperKind {
    match slot {
        Slot::Palms => ProperKind::Reading,
        Slot::First => ProperKind::FirstReading,
        Slot::Psalm => ProperKind::Psalm,
        Slot::Second => ProperKind::SecondReading,
        Slot::Gospel => ProperKind::Gospel,
    }
}

/// Psalm or canticle verses for a citation, from the Prayer Book.
pub fn psalm_or_canticle(citation: &str) -> Vec<crate::propers::PsalmVerse> {
    if let Some(n) = bcp::canticle_number(citation) {
        return bcp::canticle_verses(n).unwrap_or_default();
    }
    bcp::psalm_selection(citation, true).unwrap_or_default()
}

/// Is this a psalm/canticle citation (rather than e.g. "1 Samuel 2:1-10")?
fn is_psalm_like(citation: &str) -> bool {
    bcp::parse_psalm_ref(citation).is_some() || bcp::canticle_number(citation).is_some()
}

/// The name of an entry's collect, as the Prayer Book titles it.
fn collect_name(entry: &Entry) -> Option<String> {
    entry
        .collect
        .as_deref()
        .and_then(|c| {
            common_collect(c).or_else(|| {
                (!c.contains("BCP p.") && !c.contains("LFF") && !c.contains("Lesser")).then(|| c.to_string())
            })
        })
        .or_else(|| common_collect(&entry.name))
        .or_else(|| {
            (entry.section == Section::Common).then(|| entry.name.split(" (").next().unwrap_or(&entry.name).to_string())
        })
}

/// The Proper Preface for a date (BCP: the note under the day's collect),
/// e.g. "Easter" or "The Lord's Day: Of God the Father"; `None` where none
/// is used. Where the book gives a choice the first is taken. The Lord's Day
/// prefaces are for Sundays only, and "of the Season" is the season's own.
pub fn preface_for(date: Date, observance: Option<&str>) -> Option<String> {
    let (day, list) = observances(date);
    let entry = match observance {
        Some(name) => list.iter().copied().find(|e| e.name == name).or_else(|| find_entry(name, day.year)),
        None => list.first().copied().filter(|e| e.section != Section::Common),
    };
    // Easter Week comes before any lesser feast that falls in it.
    let easter_week = (1..=6).contains(&easter(date.year).days_until(date));
    let entry = entry.filter(|e| !(easter_week && e.section == Section::Minor));
    let collect = entry.and_then(collect_name).and_then(|n| bcp::find_collect(&n));
    let sunday = date.is_sunday();
    let season = || match day.season {
        Season::Advent => Some("Advent"),
        Season::Christmas => Some("Incarnation"),
        Season::Lent => Some("Lent"),
        Season::HolyWeek => Some("Holy Week"),
        Season::Easter => Some("Easter"),
        Season::Epiphany | Season::Pentecost => sunday.then_some("The Lord's Day"),
    };
    let first = match collect {
        // Its collect's note (none: no Eucharistic Prayer that day).
        Some(c) => bcp::preface_names_for(&c.title)?.first().cloned()?,
        // A weekday with no proper of its own: the season's.
        None => season()?.to_string(),
    };
    let name = match first.as_str() {
        "None" => return None,
        "Season" => season()?,
        "The Lord's Day" if !sunday => return None,
        n => n,
    };
    bcp::preface(name).map(|p| p.name.clone())
}

/// The Collect of the Day and the readings of an entry as importable items.
pub fn items(entry: &Entry, year: char) -> Vec<ProperItem> {
    let mut out = Vec::new();
    if let Some(found) = collect_name(entry).as_deref().and_then(bcp::find_collect) {
        for rite in [Rite::Two, Rite::One] {
            for (i, text) in found.texts(rite).iter().enumerate() {
                out.push(ProperItem {
                    kind: ProperKind::Collect,
                    citation: String::new(),
                    text: text.clone(),
                    variant: Some(rite.label().to_string()),
                    alternative: i > 0,
                    verses: Vec::new(),
                    antiphon: None,
                    antiphon_every: 0,
                });
            }
        }
    }
    for r in &entry.readings {
        if r.year.is_some_and(|y| y != year) {
            continue;
        }
        let variant = match (r.slot, r.track) {
            (Slot::Palms, _) => Some("Liturgy of the Palms".to_string()),
            (_, Some(t)) => Some(format!("Track {t}")),
            _ => None,
        };
        for (i, citation) in std::iter::once(&r.citation).chain(r.alternatives.iter()).enumerate() {
            let mut kind = kind_of(r.slot);
            if r.slot == Slot::Palms && is_psalm_like(citation) {
                kind = ProperKind::Psalm;
            }
            if r.slot == Slot::Palms && kind == ProperKind::Reading {
                kind = ProperKind::Gospel;
            }
            let verses = if kind == ProperKind::Psalm { psalm_or_canticle(citation) } else { Vec::new() };
            out.push(ProperItem {
                kind,
                citation: citation.clone(),
                text: String::new(),
                variant: variant.clone(),
                // Alternatives, and the Palms readings (often a separate liturgy), start unticked.
                alternative: i > 0 || r.slot == Slot::Palms,
                verses,
                antiphon: None,
                antiphon_every: 0,
            });
        }
    }
    out
}

/// The propers for `date`: the most likely observance, or `observance` (an
/// entry name from [`DayPropers::observances`]).
pub fn for_date(date: Date, observance: Option<&str>) -> DayPropers {
    let (day, list) = observances(date);
    let chosen = match observance {
        Some(name) => list.iter().copied().find(|e| e.name == name).or_else(|| find_entry(name, day.year)),
        None => list.first().copied().filter(|e| e.section != Section::Common),
    };
    let items = chosen.map(|e| items(e, day.year)).unwrap_or_default();
    let title = chosen.map(|e| {
        if e.section == Section::Year && observance.is_none() {
            day.name.clone()
        } else {
            e.name.split(" (").next().unwrap_or(&e.name).split(" — ").next().unwrap_or(&e.name).trim().to_string()
        }
    });
    DayPropers {
        date: date.to_string(),
        observance: chosen.map(|e| e.name.clone()),
        observances: list.iter().map(|e| e.name.clone()).collect(),
        title,
        day,
        items,
    }
}

/// The items to use by default: the Collect in `rite`, the readings of
/// `track`, no alternatives.
pub fn default_items(items: &[ProperItem], rite: Rite, track: u8) -> Vec<ProperItem> {
    let track = format!("Track {track}");
    items
        .iter()
        .filter(|i| !i.alternative)
        .filter(|i| match (&i.kind, i.variant.as_deref()) {
            (ProperKind::Collect, Some(v)) => v == rite.label(),
            (_, Some(v)) if v.starts_with("Track") => v == track,
            _ => true,
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preface_follows_the_day() {
        let p = |d: &str| preface_for(d.parse().unwrap(), None);
        assert_eq!(p("2026-11-29").as_deref(), Some("Advent"), "Advent 1");
        assert_eq!(p("2026-12-25").as_deref(), Some("Incarnation"), "Christmas Day");
        assert_eq!(p("2027-01-06").as_deref(), Some("Epiphany"), "the Epiphany");
        assert_eq!(p("2027-01-17").as_deref(), Some("Epiphany"), "2 Epiphany: Epiphany, or the Lord's Day");
        assert_eq!(p("2027-02-10").as_deref(), Some("Lent"), "Ash Wednesday");
        assert_eq!(p("2027-03-21").as_deref(), Some("Holy Week"), "Palm Sunday");
        assert_eq!(p("2027-03-28").as_deref(), Some("Easter"), "Easter Day");
        assert_eq!(p("2027-05-06").as_deref(), Some("Ascension"), "Ascension Day");
        assert_eq!(p("2027-05-16").as_deref(), Some("Pentecost"), "Pentecost");
        assert_eq!(p("2027-05-23").as_deref(), Some("Trinity Sunday"), "Trinity Sunday");
        assert_eq!(p("2026-11-08").as_deref(), Some("The Lord's Day: Of God the Father"), "Proper 27");
        assert_eq!(p("2026-11-01").as_deref(), Some("All Saints"), "All Saints' Day");
        assert_eq!(p("2027-03-26").as_deref(), None, "Good Friday");
        assert_eq!(p("2027-04-02").as_deref(), Some("Easter"), "Friday in Easter Week (not the lesser feast)");
        assert_eq!(p("2026-11-10").as_deref(), None, "a weekday after Pentecost");
    }

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    fn cites(p: &DayPropers, kind: ProperKind) -> Vec<String> {
        p.items.iter().filter(|i| i.kind == kind && !i.alternative).map(|i| i.citation.clone()).collect()
    }

    #[test]
    fn parses_every_part_of_the_reference() {
        let e = entries();
        assert!(e.len() > 300, "{}", e.len());
        for y in ['A', 'B', 'C'] {
            for p in 3..=29 {
                assert!(
                    e.iter().any(|x| x.year == Some(y) && x.key() == format!("proper {p}")),
                    "Year {y} Proper {p} missing"
                );
            }
        }
        // "Same as Year A" entries are filled in.
        let epiphany_b = e.iter().find(|x| x.year == Some('B') && x.key() == "epiphany").unwrap();
        assert_eq!(epiphany_b.readings.iter().find(|r| r.slot == Slot::Gospel).unwrap().citation, "Matthew 2:1-12");
        assert_eq!(e.iter().filter(|x| x.year == Some('C') && x.key() == "christmas day").count(), 3);
        assert!(e.iter().any(|x| x.year == Some('B') && x.key() == "good friday"));
        let andrew = e.iter().find(|x| x.name.starts_with("Saint Andrew")).unwrap();
        assert_eq!(andrew.date, Some((11, 30)));
        assert_eq!(andrew.section, Section::HolyDay);
        let alt = e.iter().find(|x| x.year == Some('A') && x.key() == "ash wednesday").unwrap();
        let ps = alt.readings.iter().find(|r| r.slot == Slot::Psalm).unwrap();
        assert_eq!(ps.citation, "Psalm 103");
        assert_eq!(ps.alternatives, vec!["Psalm 103:8-14"]);
    }

    #[test]
    fn sundays_after_pentecost_with_tracks_and_corrections() {
        // Proper 28, Year A (2026-11-15): Judges 4 / Zephaniah 1.
        let p = for_date(d("2026-11-15"), None);
        assert_eq!(p.day.proper, Some(28));
        assert_eq!(p.observance.as_deref(), Some("Proper 28"));
        let first = cites(&p, ProperKind::FirstReading);
        assert_eq!(first, vec!["Judges 4:1-7", "Zephaniah 1:7, 12-18"]);
        assert_eq!(cites(&p, ProperKind::Gospel), vec!["Matthew 25:14-30"]);
        let psalm =
            p.items.iter().find(|i| i.kind == ProperKind::Psalm && i.variant.as_deref() == Some("Track 1")).unwrap();
        assert_eq!(psalm.citation, "Psalm 123");
        assert_eq!(psalm.verses.len(), 5, "verses from the BCP Psalter");

        let chosen = default_items(&p.items, Rite::One, 2);
        let kinds: Vec<(ProperKind, String)> = chosen.iter().map(|i| (i.kind, i.citation.clone())).collect();
        assert_eq!(kinds.len(), 5, "{kinds:?}");
        assert!(chosen[0].text.starts_with("Blessed Lord, who hast caused"), "Rite I collect");
        assert_eq!(kinds[1].1, "Zephaniah 1:7, 12-18");
        assert_eq!(kinds[2].1, "Psalm 90:1-8 [9-11] 12");

        // Christ the King, Year A, and Proper 4 Year C Track 1 (corrected).
        let p = for_date(d("2026-11-22"), None);
        assert_eq!(cites(&p, ProperKind::Gospel), vec!["Matthew 25:31-46"]);
        let p = for_date(d("2046-05-27"), None); // Year C, Proper 3 (an early Easter)
        assert_eq!((p.day.year, p.day.proper), ('C', Some(3)));
        assert_eq!(cites(&p, ProperKind::Gospel), vec!["Luke 6:39-49"]);
        let p = for_date(d("2043-05-31"), None);
        assert_eq!((p.day.year, p.day.proper), ('C', Some(4)));
        assert!(cites(&p, ProperKind::FirstReading).contains(&"1 Kings 18:20-21 [22-29] 30-39".to_string()));
    }

    #[test]
    fn seasons_feasts_and_precedence() {
        // First Sunday of Advent, Year B.
        let p = for_date(d("2026-11-29"), None);
        assert_eq!(p.day.year, 'B');
        assert_eq!(cites(&p, ProperKind::FirstReading), vec!["Isaiah 64:1-9"]);
        assert!(p
            .items
            .iter()
            .any(|i| i.kind == ProperKind::Collect && i.text.starts_with("Almighty God, give us grace to cast away")));

        // Christmas Day: Proper I by default, II and III offered.
        let p = for_date(d("2026-12-25"), None);
        assert!(p.observance.as_deref().unwrap().contains("Proper I"));
        assert!(p.observances.iter().filter(|o| o.starts_with("Christmas Day")).count() >= 3);

        // The Transfiguration on a Sunday (2028-08-06) replaces the Sunday.
        let p = for_date(d("2028-08-06"), None);
        assert!(p.observance.as_deref().unwrap().contains("Transfiguration"), "{:?}", p.observance);
        // Saint James (July 25) on a Sunday (2027-07-25) does not.
        let p = for_date(d("2027-07-25"), None);
        assert!(p.observance.as_deref().unwrap().starts_with("Proper"), "{:?}", p.observance);
        assert!(p.observances.iter().any(|o| o.contains("Saint James")));

        // All Saints' Sunday is offered on the Sunday after November 1.
        let p = for_date(d("2026-11-01"), None);
        assert!(p.observance.as_deref().unwrap().starts_with("All Saints"));
        assert_eq!(cites(&p, ProperKind::Gospel), vec!["Matthew 5:1-12"], "Year A readings");
        let p = for_date(d("2027-11-07"), None);
        assert!(p.observances.iter().any(|o| o.starts_with("All Saints")));
        let p = for_date(d("2027-11-07"), Some(p.observances.iter().find(|o| o.starts_with("All Saints")).unwrap()));
        assert_eq!(cites(&p, ProperKind::Gospel), vec!["John 11:32-44"], "Year B readings");

        // Palm Sunday: the Liturgy of the Palms is offered unticked.
        let p = for_date(d("2027-03-21"), None);
        assert!(p.items.iter().any(|i| i.variant.as_deref() == Some("Liturgy of the Palms")));
        // Canticles in place of the psalm get their verses.
        let p = for_date(d("2026-12-20"), None); // Advent 4, Year B: Canticle 15
        let c = p.items.iter().find(|i| i.kind == ProperKind::Psalm && !i.alternative).unwrap();
        assert!(c.citation.starts_with("Canticle 15"));
        assert!(c.verses.len() >= 8);
        // A weekday with a holy day.
        let p = for_date(d("2026-11-30"), None);
        assert!(p.observance.as_deref().unwrap().starts_with("Saint Andrew"));
    }

    #[test]
    fn every_sunday_for_years_has_a_collect_and_gospel() {
        let mut date = d("2025-11-30");
        let mut missing = Vec::new();
        while date < d("2040-12-01") {
            let p = for_date(date, None);
            let collect = p.items.iter().any(|i| i.kind == ProperKind::Collect);
            let gospel = p.items.iter().any(|i| i.kind == ProperKind::Gospel && !i.alternative);
            let psalm = p.items.iter().any(|i| i.kind == ProperKind::Psalm && !i.alternative && !i.verses.is_empty());
            if !(collect && gospel && psalm) {
                missing.push(format!("{date} {} → {:?} c{collect} g{gospel} p{psalm}", p.day.name, p.observance));
            }
            date = date.add_days(7);
        }
        assert!(missing.is_empty(), "{} Sundays incomplete:\n{}", missing.len(), missing.join("\n"));
    }
}
