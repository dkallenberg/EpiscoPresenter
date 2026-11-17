//! Scheduled services kept by the app.
//!
//! Each service is a folder in `<app data>/services/`, named after its date
//! and title (`2026-11-15-the-twenty-fifth-sunday-after-pentecost/`), holding
//! `service.toml` and a `liturgies/` folder with its texts. Services there are
//! saved automatically (autosave.rs) and listed on the Services screen.
//!
//! New services come from a template: the standard Rite I or Rite II order
//! (built from the library, adjusted for the season) or a copy of an existing
//! service without its propers. The day's propers (Collect of the Day,
//! readings, psalm and Gospel) are then added from the lectionary.

use std::path::{Path, PathBuf};

use episco_core::bcp::Rite;
use episco_core::bible::BibleVersion;
use episco_core::calendar::{liturgical_day, Date, LiturgicalDay};
use episco_core::regular::{MusicPlace, Weekday};
use episco_core::service::Loaded;
use episco_core::{lectionary, propers};
use episco_core::{
    AnnouncementMode, Block, Cue, CueOverrides, ElementCategory, FrontMatter, LiturgyDocument, Role, Season, Service,
    Span,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::commands::slug_file_name;
use crate::engine::Engine;
use crate::library::builtin_doc;

type CmdResult<T> = Result<T, String>;

pub const PLAN_FILE: &str = "service.toml";

pub fn services_dir(app: &AppHandle) -> CmdResult<PathBuf> {
    crate::storage::data_dir(app, "services")
}

fn parse_date(s: &str) -> CmdResult<Date> {
    s.parse::<Date>()
}

// ---- calendar ------------------------------------------------------------

/// The church-year name, season, colour and lectionary year of a date.
#[tauri::command]
pub fn liturgical_info(date: String) -> CmdResult<LiturgicalDay> {
    Ok(liturgical_day(parse_date(&date)?))
}

// ---- listing -------------------------------------------------------------

#[derive(Clone, Serialize, Deserialize)]
pub struct ServiceSummary {
    path: PathBuf,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    /// Start time, "08:00".
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    day: Option<LiturgicalDay>,
    cues: usize,
    /// No reading or Gospel yet: the propers still need importing.
    needs_propers: bool,
    /// Marked Ready for the presentation computer (EpiscoPlanner).
    #[serde(skip_serializing_if = "Option::is_none")]
    ready: Option<crate::handoff::ReadyMark>,
    /// Open on another computer (EpiscoPlanner).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    in_use: Option<crate::inuse::Mark>,
}

fn summary(path: PathBuf, service: &Service) -> ServiceSummary {
    let day = service.date.as_deref().and_then(|d| d.parse::<Date>().ok()).map(liturgical_day);
    ServiceSummary {
        title: service.title.clone(),
        date: service.date.clone(),
        time: service.time.clone(),
        day,
        cues: service.cues.len(),
        needs_propers: !service
            .cues
            .iter()
            .any(|c| matches!(c.category, ElementCategory::Reading | ElementCategory::Gospel)),
        ready: crate::handoff::ready_mark(&path),
        in_use: None,
        path,
    }
}

/// Every service in the app's folder, by date (undated last). Run apart
/// from the window, so it opens at once even while a shared folder is slow.
#[tauri::command]
pub async fn services_list(app: AppHandle) -> CmdResult<Vec<ServiceSummary>> {
    tauri::async_runtime::spawn_blocking(move || list_services(&app)).await.map_err(|e| e.to_string())?
}

/// [`services_list`]. A service is read again only when a file in its
/// folder has changed since last time (see [`crate::cache`]).
pub fn list_services(app: &AppHandle) -> CmdResult<Vec<ServiceSummary>> {
    let dir = services_dir(app)?;
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(out) };
    let mut cache = crate::cache::Cache::open(app, "services");
    for entry in entries.flatten() {
        let folder = entry.path();
        let plan = folder.join(PLAN_FILE);
        if !plan.is_file() {
            continue;
        }
        let stamp = crate::cache::folder_stamp(&folder);
        let read = || match Service::load_plan(&plan) {
            Ok(Loaded { service, .. }) => Some(summary(plan.clone(), &service)),
            Err(e) => {
                tracing::warn!("skipping service {}: {e}", plan.display());
                None
            }
        };
        if let Some(mut s) = cache.get(&folder, &stamp, read) {
            // Who has it open changes by the minute: never remembered.
            s.in_use = crate::inuse::holder(&plan);
            out.push(s);
        }
    }
    cache.save();
    out.sort_by(|a, b| match (&a.date, &b.date) {
        (Some(x), Some(y)) => x.cmp(y).then(a.time.cmp(&b.time)).then(a.title.cmp(&b.title)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.title.cmp(&b.title),
    });
    Ok(out)
}

// ---- templates -----------------------------------------------------------

/// A hymn slot to be filled in each week.
fn hymn_placeholder(title: &str) -> LiturgyDocument {
    LiturgyDocument {
        meta: FrontMatter { title: title.into(), category: Some(ElementCategory::Hymn), ..Default::default() },
        blocks: vec![Block::Paragraph {
            role: Some(Role::Rubric),
            lines: vec![vec![Span::italic(
                "Choose this hymn: paste the verses here, or replace this item with one from the Library.",
            )]],
        }],
    }
}

/// A Eucharistic Prayer: A–D (Rite II, BCP 361-375), I and II (Rite I,
/// BCP 333-343).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EucharisticPrayer {
    A,
    B,
    C,
    D,
    #[serde(rename = "I")]
    One,
    #[serde(rename = "II")]
    Two,
}

/// The library item of a Eucharistic Prayer in a rite; none for a prayer
/// the rite does not have (Prayer A in Rite I).
fn eucharistic_prayer_item(rite: Rite, prayer: EucharisticPrayer) -> Option<&'static str> {
    use EucharisticPrayer as P;
    match (rite, prayer) {
        (Rite::Two, P::A) => Some("eucharistic-prayer-a"),
        (Rite::Two, P::B) => Some("eucharistic-prayer-b"),
        (Rite::Two, P::C) => Some("eucharistic-prayer-c"),
        (Rite::Two, P::D) => Some("eucharistic-prayer-d"),
        (Rite::One, P::One) => Some("rite1/eucharistic-prayer-1"),
        (Rite::One, P::Two) => Some("rite1/eucharistic-prayer-2"),
        _ => None,
    }
}

/// Puts a whole Eucharistic Prayer in place of the Great Thanksgiving's
/// dialogue (the prayer begins with it), and leaves out the separate Sanctus
/// that follows it (the prayer has the Sanctus in its place).
fn use_eucharistic_prayer(service: &mut Service, rite: Rite, prayer: EucharisticPrayer) {
    let Some(doc) = eucharistic_prayer_item(rite, prayer).and_then(builtin_doc) else { return };
    use_eucharistic_doc(service, doc);
}

/// Whether a cue is the Prayer Book item `name` (in either rite).
fn is_item(c: &Cue, name: &str) -> bool {
    c.document().and_then(|d| d.meta.extra("based_on")).is_some_and(|b| b == name || b == format!("rite1/{name}"))
}

/// Puts `doc` (a whole Eucharistic Prayer, the Prayer Book's or the
/// parish's) in place of the dialogue and the Sanctus after it, or of the
/// Eucharistic Prayer the service has. A parish's prayer gets its place for
/// the Proper Preface marked. Returns whether it found the place.
fn use_eucharistic_doc(service: &mut Service, doc: LiturgyDocument) -> bool {
    let at = service
        .cues
        .iter()
        .position(|c| is_item(c, "sursum-corda"))
        .or_else(|| service.cues.iter().position(|c| c.category == ElementCategory::EucharisticPrayer));
    let Some(at) = at else { return false };
    let doc = if doc.meta.extra("based_on").is_none() { episco_core::bcp::mark_preface_place(&doc) } else { doc };
    replace_cue(service, at, doc);
    if service.cues.get(at + 1).is_some_and(|c| is_item(c, "sanctus")) {
        service.cues.remove(at + 1);
    }
    true
}

/// Puts back the dialogue and the said Sanctus in place of a whole
/// Eucharistic Prayer.
fn use_dialogue(service: &mut Service, rite: Rite) -> bool {
    let whole = |c: &Cue| c.category == ElementCategory::EucharisticPrayer && !is_item(c, "sursum-corda");
    let Some(at) = service.cues.iter().position(whole) else {
        return false;
    };
    let (Some(dialogue), Some(sanctus)) =
        (builtin_doc(&rite_name(rite, "sursum-corda")), builtin_doc(&rite_name(rite, "sanctus")))
    else {
        return false;
    };
    replace_cue(service, at, dialogue);
    let mut sanctus = said(Cue::liturgy("", sanctus, None, CueOverrides::default()));
    sanctus.modified = true;
    service.insert_cue(at + 1, sanctus);
    true
}

/// A new cue with `doc` at `at` (a fresh id, a new item's settings).
fn replace_cue(service: &mut Service, at: usize, doc: LiturgyDocument) {
    let mut cue = Cue::liturgy("", doc, None, CueOverrides::default());
    cue.modified = true;
    service.cues[at] = cue;
    let ids: Vec<String> = service.cues.iter().map(|c| c.id.clone()).collect();
    if service.cues[at].id.is_empty() || ids.iter().filter(|i| **i == service.cues[at].id).count() > 1 {
        let title = service.cues[at].document().map(|d| d.meta.title.clone()).unwrap_or_default();
        service.cues[at].id = service.unique_id(&title);
    }
}

/// Whether a cue is the service's Prayers of the People (a Prayer Book
/// form, or the parish's own).
fn is_prayers(c: &Cue) -> bool {
    c.document().is_some_and(|d| {
        d.meta.extra("based_on").is_some_and(|b| PRAYERS_FORMS.contains(&b))
            || d.meta.extra(crate::library::USES) == Some(crate::library::PRAYERS_OF_THE_PEOPLE)
    })
}

/// Puts `doc` in place of the service's Prayers of the People. Returns
/// whether the service has them.
fn use_prayers_doc(service: &mut Service, doc: LiturgyDocument) -> bool {
    let Some(at) = service.cues.iter().position(is_prayers) else { return false };
    replace_cue(service, at, doc);
    true
}

/// The Sanctus said (by all, in bold, on a prayer's layout), as new
/// services have it; the Editor's Sanctus panel makes it sung (as a hymn).
pub(crate) fn said(mut cue: Cue) -> Cue {
    cue.overrides.category = Some(ElementCategory::Acclamation);
    cue
}

/// A hymn slot.
pub(crate) fn hymn_cue(title: &str) -> Option<Cue> {
    Some(Cue::liturgy("", hymn_placeholder(title), None, CueOverrides::default()))
}

/// The sermon slot.
pub(crate) fn sermon_cue() -> Cue {
    Cue::liturgy("", sermon(), None, CueOverrides::default())
}

fn sermon() -> LiturgyDocument {
    LiturgyDocument {
        meta: FrontMatter { title: "The Sermon".into(), category: Some(ElementCategory::Homily), ..Default::default() },
        blocks: vec![
            Block::Heading { level: 1, text: "The Sermon".into() },
            Block::Paragraph {
                role: None,
                lines: vec![vec![Span::plain("The Preacher's Name")], vec![Span::plain("Title")]],
            },
        ],
    }
}

/// The Prayers of the People for a service: `prayers` is a form of BCP
/// 383-393 ("1" to "6") or "rite1" (the Rite I prayer, BCP 328); by default
/// the Rite I prayer in Rite I and Form I in Rite II.
pub(crate) fn prayers_item(rite: Rite, prayers: Option<PrayersForm>) -> String {
    use PrayersForm::*;
    let form = match prayers {
        Some(RiteOne) => return "rite1/prayers-of-the-people".into(),
        None if rite == Rite::One => return "rite1/prayers-of-the-people".into(),
        None | Some(I) => 1,
        Some(II) => 2,
        Some(III) => 3,
        Some(IV) => 4,
        Some(V) => 5,
        Some(VI) => 6,
    };
    format!("prayers-of-the-people-{form}")
}

/// A form of the Prayers of the People (Roman numerals, as the Prayer Book).
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrayersForm {
    #[serde(rename = "1")]
    I,
    #[serde(rename = "2")]
    II,
    #[serde(rename = "3")]
    III,
    #[serde(rename = "4")]
    IV,
    #[serde(rename = "5")]
    V,
    #[serde(rename = "6")]
    VI,
    /// The Rite I prayer "for the whole state of Christ's Church".
    #[serde(rename = "rite1")]
    RiteOne,
}

/// Every form of the Prayers of the People.
pub(crate) const PRAYERS_FORMS: [&str; 7] = [
    "prayers-of-the-people-1",
    "prayers-of-the-people-2",
    "prayers-of-the-people-3",
    "prayers-of-the-people-4",
    "prayers-of-the-people-5",
    "prayers-of-the-people-6",
    "rite1/prayers-of-the-people",
];

/// The library item for `name` in `rite` (Rite I has its own forms).
pub(crate) fn rite_name(rite: Rite, name: &str) -> String {
    let rite1 = match name {
        "postcommunion-eternal-god" => Some("postcommunion"),
        "acclamation-ordinary"
        | "acclamation-easter"
        | "acclamation-lent"
        | "collect-for-purity"
        | "kyrie"
        | "gloria-in-excelsis"
        | "salutation"
        | "nicene-creed"
        | "confession"
        | "absolution"
        | "the-peace"
        | "sursum-corda"
        | "sanctus"
        | "lords-prayer"
        | "fraction-anthem"
        | "invitation"
        | "humble-access" => Some(name),
        _ => None,
    };
    match (rite, rite1) {
        (Rite::One, Some(n)) => format!("rite1/{n}"),
        _ => name.to_string(),
    }
}

/// The standard order of the Holy Eucharist in `rite`, with seasonal
/// choices for `day`.
fn standard_cues(day: &LiturgicalDay, rite: Rite, prayers: Option<PrayersForm>) -> Vec<Cue> {
    let lib = |name: &str, label: Option<&str>| {
        builtin_doc(&rite_name(rite, name)).map(|doc| {
            let overrides = CueOverrides { title: label.map(String::from), ..Default::default() };
            Cue::liturgy("", doc, None, overrides)
        })
    };
    let hymn = |title: &str| Some(Cue::liturgy("", hymn_placeholder(title), None, CueOverrides::default()));
    let acclamation = match day.season {
        Season::Easter => "acclamation-easter",
        Season::Lent | Season::HolyWeek => "acclamation-lent",
        _ => "acclamation-ordinary",
    };
    // The Gloria is not said in Advent or Lent; the Kyrie is used instead.
    let song_of_praise = if matches!(day.season, Season::Advent | Season::Lent | Season::HolyWeek) {
        lib("kyrie", None)
    } else {
        lib("gloria-in-excelsis", Some("Hymn of Praise"))
    };
    // The Prayer of Humble Access is a Rite I custom.
    let humble_access = (rite == Rite::One).then(|| lib("humble-access", None)).flatten();
    let welcome = Some(Cue::announcements("", AnnouncementMode::SinglePass, CueOverrides::default())).map(|mut c| {
        c.overrides.title = Some("Welcome & Announcements".into());
        c
    });
    [
        hymn("Entrance Hymn"),
        lib(acclamation, None),
        lib("collect-for-purity", None),
        song_of_praise,
        lib("salutation", None),
        Some(Cue::liturgy("", sermon(), None, CueOverrides::default())),
        lib("nicene-creed", None),
        lib(&prayers_item(rite, prayers), None),
        lib("confession", None),
        lib("absolution", None),
        lib("the-peace", None),
        hymn("Offertory Hymn"),
        lib("sursum-corda", None),
        lib("sanctus", None).map(said),
        lib("lords-prayer", None),
        lib("fraction-anthem", None),
        humble_access,
        lib("invitation", None),
        hymn("Communion Hymn"),
        lib("postcommunion-eternal-god", None),
        // Announcements come after Communion, before the closing hymn.
        welcome,
        hymn("Closing Hymn"),
        lib("dismissal-go-forth", None),
        Some(Cue::announcements("", AnnouncementMode::Loop, CueOverrides::default())),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The day's propers from the lectionary: the Collect in `rite`, the
/// readings of `track`, the psalm with its Prayer Book verses.
fn proper_cues(date: Date, rite: Rite, track: u8) -> Vec<Cue> {
    let p = lectionary::for_date(date, None);
    lectionary::default_items(&p.items, rite, track)
        .iter()
        .map(|item| {
            Cue::liturgy("", propers::build_document_in(item, p.title.as_deref(), rite), None, CueOverrides::default())
        })
        .collect()
}

/// Where the propers go: before the sermon (else after the Salutation, else
/// at the end).
fn propers_position(service: &Service) -> usize {
    if let Some(i) = service.cues.iter().position(|c| c.category == ElementCategory::Homily) {
        return i;
    }
    service
        .cues
        .iter()
        .position(|c| c.title.eq_ignore_ascii_case("Salutation"))
        .map(|i| i + 1)
        .unwrap_or(service.cues.len())
}

/// Choices for a new service. Unset values come from the template, else the
/// defaults (Rite II, Track 1, NRSVue).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServiceOptions {
    #[serde(default)]
    pub rite: Option<Rite>,
    #[serde(default)]
    pub track: Option<u8>,
    #[serde(default)]
    pub bible: Option<BibleVersion>,
    /// Add the day's propers from the lectionary (default true).
    #[serde(default)]
    pub propers: Option<bool>,
    /// With no `template`: use the saved template for the date's season,
    /// if there is one, else the standard order.
    #[serde(default)]
    pub by_season: bool,
    /// The form of the Prayers of the People: "1"–"6" (BCP 383-393) or
    /// "rite1" (BCP 328). Default: the rite's usual one.
    #[serde(default)]
    pub prayers: Option<PrayersForm>,
    /// The Eucharistic Prayer: "A"–"D" (Rite II) or "I", "II" (Rite I) in
    /// full, in place of the dialogue (and the separate Sanctus, which the
    /// prayer has). None: the dialogue and the Sanctus only.
    #[serde(default)]
    pub eucharistic_prayer: Option<EucharisticPrayer>,
    /// The parish's own Prayers of the People (a library id, `user:…`), in
    /// place of a form of the Prayer Book's.
    #[serde(default)]
    pub prayers_item: Option<String>,
    /// The parish's own Eucharistic Prayer (a library id), in place of the
    /// dialogue and the Sanctus.
    #[serde(default)]
    pub eucharistic_prayer_item: Option<String>,
    /// The music places it uses (from its regular service, or chosen for
    /// this one); `None`: the order's own.
    #[serde(default)]
    pub music: Option<Vec<MusicPlace>>,
    /// The bulletin's paper, when not the parish's.
    #[serde(default)]
    pub paper: Option<episco_core::bulletin::BulletinPaper>,
    /// The kind of service (Holy Week, ordinations) and the bishop,
    /// baptism, renewal and confirmation options.
    #[serde(default, flatten)]
    pub occasion: crate::occasions::Options,
}

/// Fits the order to the music places used: hymns not sung are left out,
/// a Gradual or Sequence hymn goes before the Gospel and the Offertory
/// Anthem after the Peace, an unsung Hymn of Praise keeps its own name
/// (the Gloria, said), and a prelude or postlude not played is left out of
/// the bulletin.
fn apply_music(service: &mut Service, music: &[MusicPlace]) {
    use episco_core::bulletin::BulletinStyle;
    use MusicPlace as M;
    let has = |p: MusicPlace| music.contains(&p);
    let titled = |s: &Service, t: &str| s.cues.iter().position(|c| c.title.eq_ignore_ascii_case(t));
    for p in [M::EntranceHymn, M::OffertoryHymn, M::CommunionHymn, M::ClosingHymn, M::Gradual, M::Sequence] {
        if has(p) {
            continue;
        }
        let ids: Vec<String> = service
            .cues
            .iter()
            .filter(|c| c.category == ElementCategory::Hymn && c.title.eq_ignore_ascii_case(p.label()))
            .map(|c| c.id.clone())
            .collect();
        for id in ids {
            let _ = service.remove_cue(&id);
        }
    }
    if !has(M::HymnOfPraise) {
        let ids: Vec<String> = service
            .cues
            .iter()
            .filter(|c| c.overrides.title.as_deref() == Some(M::HymnOfPraise.label()))
            .map(|c| c.id.clone())
            .collect();
        for id in ids {
            let mut o = service.cue(&id).map(|c| c.overrides.clone()).unwrap_or_default();
            o.title = None;
            let _ = service.set_overrides(&id, o);
        }
    }
    // Before the Gospel: the Gradual, then the Sequence.
    if let Some(mut at) = service.cues.iter().position(|c| c.category == ElementCategory::Gospel) {
        for p in [M::Gradual, M::Sequence] {
            if has(p) && titled(service, p.label()).is_none() {
                service.insert_cue(at, Cue::liturgy("", hymn_placeholder(p.label()), None, CueOverrides::default()));
                at += 1;
            }
        }
    }
    if has(M::OffertoryAnthem) && titled(service, M::OffertoryAnthem.label()).is_none() {
        let at = titled(service, "The Peace").map(|i| i + 1).or_else(|| titled(service, M::OffertoryHymn.label()));
        if let Some(at) = at {
            let overrides = CueOverrides { title: Some(M::OffertoryAnthem.label().into()), ..Default::default() };
            let doc = episco_core::music::anthem_document("", "");
            service.insert_cue(at, Cue::liturgy("", doc, None, overrides));
        }
    }
    let left_out = || Some(episco_core::music::Music::default().with_bulletin(BulletinStyle::Hidden));
    if !has(M::Prelude) && service.prelude.as_ref().is_none_or(|m| m.is_empty()) {
        service.prelude = left_out();
    }
    if !has(M::Postlude) && service.postlude.as_ref().is_none_or(|m| m.is_empty()) {
        service.postlude = left_out();
    }
}

/// Propers are specific to a day and are not carried into a copy.
fn is_proper(cue: &Cue) -> bool {
    matches!(cue.category, ElementCategory::Reading | ElementCategory::Psalm | ElementCategory::Gospel)
        || cue.title.eq_ignore_ascii_case("Collect of the Day")
}

/// The service for `day`: a copy of `template` without its propers (and with
/// its own, not-yet-saved files), or the standard order.
fn build(date: Date, title: Option<String>, template: Option<Service>, opts: ServiceOptions) -> Service {
    let day = liturgical_day(date);
    let rite = opts.rite.or(template.as_ref().and_then(|t| t.rite)).unwrap_or_default();
    let track = opts.track.or(template.as_ref().and_then(|t| t.track)).unwrap_or(1);
    let bible = opts.bible.or(template.as_ref().and_then(|t| t.bible));
    let special = crate::occasions::order(opts.occasion, rite, opts.prayers);
    let template = if special.is_some() { None } else { template };
    let mut service = match template {
        Some(mut service) => {
            service.cues.retain(|c| !is_proper(c));
            for cue in &mut service.cues {
                if let episco_core::CueContent::Liturgy { source, .. } = &mut cue.content {
                    *source = None;
                }
                cue.modified = true;
            }
            service
        }
        None => {
            let mut s = Service::default();
            let cues = match special {
                // An event has just its blank slides; services end with the loop.
                Some(cues) if opts.occasion.occasion == crate::occasions::Occasion::Event => cues,
                Some(cues) => crate::occasions::with_loop(cues),
                None => standard_cues(&day, rite, opts.prayers),
            };
            for cue in cues {
                let at = s.cues.len();
                s.insert_cue(at, cue);
            }
            s
        }
    };
    if opts.propers.unwrap_or(true) {
        let at = propers_position(&service);
        let propers = crate::occasions::fixed_propers(opts.occasion.occasion, date, rite)
            .unwrap_or_else(|| proper_cues(date, rite, track));
        for (k, cue) in propers.into_iter().enumerate() {
            service.insert_cue(at + k, cue);
        }
    }
    crate::occasions::apply(&mut service, opts.occasion);
    if let Some(prayer) = opts.eucharistic_prayer {
        use_eucharistic_prayer(&mut service, rite, prayer);
    }
    service.rite = (rite == Rite::One).then_some(rite);
    service.track = (track == 2).then_some(2);
    service.bible = bible;
    service.title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .or_else(|| opts.occasion.occasion.title().map(String::from))
        .unwrap_or_else(|| day.name.clone());
    service.date = Some(date.to_string());
    service.season = Some(day.season);
    service.color = Some(day.color);
    service.plan_path = None;
    service.rebuild_all();
    service
}

/// Puts `music` before each Gospel that has none yet.
fn with_gospel_music(service: &mut Service, music: &(String, String)) {
    let ids: Vec<String> = service
        .cues
        .iter()
        .filter(|c| c.category == episco_core::ElementCategory::Gospel)
        .filter(|c| {
            c.document().is_some_and(|d| {
                !d.blocks
                    .iter()
                    .any(|b| matches!(b, Block::Heading { text, .. } if text == episco_core::propers::GOSPEL_MUSIC))
            })
        })
        .map(|c| c.id.clone())
        .collect();
    for id in ids {
        let Some(doc) = service.cue(&id).and_then(|c| c.document()) else { continue };
        let new = episco_core::propers::arrange_gospel(doc, Some(Some(music.clone())));
        let _ = service.set_document(&id, new);
    }
}

/// Builds and saves a service for `date` in the app's folder.
fn create(
    app: &AppHandle,
    date: Date,
    title: Option<String>,
    template: Option<&Path>,
    opts: ServiceOptions,
    time: Option<&str>,
) -> CmdResult<PathBuf> {
    let seasonal = match template {
        None if opts.by_season => season_template(app, liturgical_day(date).season),
        _ => None,
    };
    let template = match template.or(seasonal.as_deref()) {
        Some(path) => Some(Service::load_plan(path).map_err(|e| e.to_string())?.service),
        None => None,
    };
    // The parish's own Prayers of the People and Eucharistic Prayer.
    let parish = |id: &Option<String>| -> CmdResult<Option<LiturgyDocument>> {
        id.as_deref().filter(|i| i.starts_with("user:")).map(|i| crate::library::load(app, i)).transpose()
    };
    let (prayers, prayer) = (parish(&opts.prayers_item)?, parish(&opts.eucharistic_prayer_item)?);
    let (music, paper) = (opts.music.clone(), opts.paper);
    let mut service = build(date, title, template, opts);
    let mut changed = false;
    if let Some(music) = &music {
        apply_music(&mut service, music);
        changed = true;
    }
    service.paper = paper;
    if let Some(doc) = prayers {
        changed |= use_prayers_doc(&mut service, doc);
    }
    if let Some(doc) = prayer {
        changed |= use_eucharistic_doc(&mut service, doc);
    }
    if changed {
        service.rebuild_all();
    }
    service.time = time.map(str::trim).filter(|t| !t.is_empty()).map(String::from);
    service.library_root = crate::storage::root();
    // Reading texts from the parish's Bible file, if it has one.
    crate::scripture::fill_service(app, &mut service);
    // The parish's usual music line before the Gospel.
    if let Some(music) = crate::library::usual_gospel_music(app) {
        with_gospel_music(&mut service, &music);
    }

    let dir = services_dir(app)?;
    // "2026-11-15-0800-the-twenty-fifth-sunday…": two services a day stay apart.
    let when =
        service.time.as_deref().map(|t| format!("{date}-{}", t.replace(':', ""))).unwrap_or_else(|| date.to_string());
    let base = format!("{when}-{}", slug_file_name(&service.title, "x").trim_end_matches(".x"));
    let mut folder = dir.join(&base);
    let mut n = 2;
    while folder.exists() {
        folder = dir.join(format!("{base}-{n}"));
        n += 1;
    }
    let plan = folder.join(PLAN_FILE);
    service.save_to(&plan).map_err(|e| e.to_string())?;
    Ok(plan)
}

/// Creates a service for `date` and opens it. `template` is the plan of a
/// service to copy (without its propers); `None` uses the standard order.
#[tauri::command]
pub fn service_create(
    app: AppHandle,
    engine: State<'_, Engine>,
    date: String,
    title: Option<String>,
    template: Option<PathBuf>,
    options: Option<ServiceOptions>,
    time: Option<String>,
) -> CmdResult<PathBuf> {
    if let Some(t) = time.as_deref().filter(|t| !t.trim().is_empty()) {
        episco_core::bulletin::time_phrase(t).ok_or_else(|| format!("“{t}” is not a time like 08:00 or 10:30"))?;
    }
    let plan =
        create(&app, parse_date(&date)?, title, template.as_deref(), options.unwrap_or_default(), time.as_deref())?;
    let Loaded { service, .. } = Service::load_plan(&plan).map_err(|e| e.to_string())?;
    engine.load_service(Some(service))?;
    Ok(plan)
}

/// One regular service to schedule: its day and time, and its defaults.
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleEntry {
    #[serde(default)]
    pub day: Weekday,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub rite: Option<Rite>,
    #[serde(default)]
    pub music: Option<Vec<MusicPlace>>,
    #[serde(default)]
    pub paper: Option<episco_core::bulletin::BulletinPaper>,
}

/// Creates services for the next `weeks` weeks from `from` (a date) that do
/// not have one yet: one for each of the parish's regular services in
/// `regular` (its day, time, rite, music and paper), or one for each of
/// `times` on Sunday, or one a Sunday. Returns the new plans.
#[tauri::command]
pub fn services_schedule(
    app: AppHandle,
    from: String,
    weeks: u32,
    template: Option<PathBuf>,
    options: Option<ServiceOptions>,
    times: Option<Vec<String>>,
    regular: Option<Vec<ScheduleEntry>>,
) -> CmdResult<Vec<PathBuf>> {
    let options = options.unwrap_or_default();
    let existing: Vec<(String, Option<String>)> =
        list_services(&app)?.into_iter().filter_map(|s| s.date.map(|d| (d, s.time))).collect();
    let entries: Vec<ScheduleEntry> = match (regular.filter(|r| !r.is_empty()), times.filter(|t| !t.is_empty())) {
        (Some(r), _) => r,
        (None, Some(t)) => t
            .into_iter()
            .map(|t| ScheduleEntry { day: Weekday::Sunday, time: Some(t), rite: None, music: None, paper: None })
            .collect(),
        (None, None) => vec![ScheduleEntry { day: Weekday::Sunday, time: None, rite: None, music: None, paper: None }],
    };
    let start = parse_date(&from)?;
    let mut created = Vec::new();
    for w in 0..weeks.min(26) {
        for e in &entries {
            let date = e.day.on_or_after(start).add_days(7 * i64::from(w));
            let day = date.to_string();
            let time = e.time.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(String::from);
            // Already planned: a service that day at that time (or, without
            // a time, any service that day).
            let taken = existing.iter().any(|(d, t)| *d == day && (time.is_none() || *t == time));
            if taken {
                continue;
            }
            let mut opts = options.clone();
            opts.rite = e.rite.or(opts.rite);
            opts.music = e.music.clone().or(opts.music);
            opts.paper = e.paper.or(opts.paper);
            created.push(create(&app, date, None, template.as_deref(), opts, time.as_deref())?);
        }
    }
    Ok(created)
}

// ---- service order templates ---------------------------------------------

/// Named orders of service to start new services from, e.g. "Lent, Rite I",
/// each optionally used automatically for some seasons. A template is a
/// service folder in `<app data>/templates/` (plan and texts, without
/// propers) with a `template.json` naming it.
pub fn templates_dir(app: &AppHandle) -> CmdResult<PathBuf> {
    crate::storage::data_dir(app, "templates")
}

const TEMPLATE_META: &str = "template.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct TemplateMeta {
    name: String,
    #[serde(default)]
    seasons: Vec<Season>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// The template's plan file.
    path: PathBuf,
    name: String,
    seasons: Vec<Season>,
    cues: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    rite: Option<Rite>,
}

fn read_meta(folder: &Path) -> Option<TemplateMeta> {
    serde_json::from_str(&std::fs::read_to_string(folder.join(TEMPLATE_META)).ok()?).ok()
}

fn write_meta(folder: &Path, meta: &TemplateMeta) -> CmdResult<()> {
    let json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    std::fs::write(folder.join(TEMPLATE_META), json).map_err(|e| format!("could not save the template: {e}"))
}

/// A template folder, checked to be inside the templates folder.
fn template_folder(app: &AppHandle, plan: &Path) -> CmdResult<PathBuf> {
    let dir = templates_dir(app)?.canonicalize().map_err(|e| e.to_string())?;
    let folder = plan.parent().ok_or("bad path")?.canonicalize().map_err(|e| e.to_string())?;
    if folder.parent() != Some(dir.as_path()) {
        return Err("not a service order template".into());
    }
    Ok(folder)
}

/// The saved templates, by name (apart from the window; see [`services_list`]).
#[tauri::command]
pub async fn templates_list(app: AppHandle) -> CmdResult<Vec<TemplateInfo>> {
    tauri::async_runtime::spawn_blocking(move || list_templates(&app)).await.map_err(|e| e.to_string())?
}

/// [`templates_list`], read again only where a template has changed.
fn list_templates(app: &AppHandle) -> CmdResult<Vec<TemplateInfo>> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(templates_dir(app)?) else { return Ok(out) };
    let mut cache = crate::cache::Cache::open(app, "templates");
    for entry in entries.flatten() {
        let folder = entry.path();
        let plan = folder.join(PLAN_FILE);
        if !plan.is_file() {
            continue;
        }
        let stamp = crate::cache::folder_stamp(&folder);
        let read = || {
            let meta = read_meta(&folder)?;
            match Service::load_plan(&plan) {
                Ok(Loaded { service, .. }) => Some(TemplateInfo {
                    path: plan.clone(),
                    name: meta.name,
                    seasons: meta.seasons,
                    cues: service.cues.len(),
                    rite: service.rite,
                }),
                Err(e) => {
                    tracing::warn!("skipping template {}: {e}", plan.display());
                    None
                }
            }
        };
        if let Some(t) = cache.get(&folder, &stamp, read) {
            out.push(t);
        }
    }
    cache.save();
    out.sort_by_key(|t| t.name.to_lowercase());
    Ok(out)
}

/// The first template (by name) kept for `season`.
fn season_template(app: &AppHandle, season: Season) -> Option<PathBuf> {
    list_templates(app).ok()?.into_iter().find(|t| t.seasons.contains(&season)).map(|t| t.path)
}

/// A service turned into a template: its order and texts without the day's
/// propers, date or file.
fn as_template(service: &Service, name: &str) -> Service {
    let mut t = service.clone();
    t.cues.retain(|c| !is_proper(c));
    for cue in &mut t.cues {
        if let episco_core::CueContent::Liturgy { source, .. } = &mut cue.content {
            *source = None;
        }
        cue.modified = true;
    }
    t.title = name.to_string();
    t.date = None;
    t.plan_path = None;
    t.modified = true;
    t
}

/// Saves the open service's order as a named template (without its
/// propers). Saving under an existing template's name replaces it.
#[tauri::command]
pub fn template_save(
    app: AppHandle,
    engine: State<'_, Engine>,
    name: String,
    seasons: Vec<Season>,
) -> CmdResult<PathBuf> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("name the template".into());
    }
    let service = engine.service().ok_or("open a service first")?;
    let dir = templates_dir(&app)?;
    let slug = slug_file_name(&name, "x").trim_end_matches(".x").to_string();
    let folder = dir.join(&slug);
    if folder.exists() {
        // Replace: a fresh folder, so texts no longer used do not linger.
        std::fs::remove_dir_all(&folder).map_err(|e| format!("could not replace the template: {e}"))?;
    }
    let plan = folder.join(PLAN_FILE);
    let mut template = as_template(&service, &name);
    template.save_to(&plan).map_err(|e| e.to_string())?;
    write_meta(&folder, &TemplateMeta { name, seasons })?;
    Ok(plan)
}

/// Renames a template and sets the seasons it is used for.
#[tauri::command]
pub fn template_update(app: AppHandle, path: PathBuf, name: String, seasons: Vec<Season>) -> CmdResult<()> {
    let folder = template_folder(&app, &path)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("name the template".into());
    }
    write_meta(&folder, &TemplateMeta { name, seasons })
}

/// Deletes a template.
#[tauri::command]
pub fn template_delete(app: AppHandle, engine: State<'_, Engine>, path: PathBuf) -> CmdResult<()> {
    let folder = template_folder(&app, &path)?;
    if engine.service().and_then(|s| s.plan_path.clone()).is_some_and(|p| p.starts_with(&folder)) {
        engine.load_service(None)?;
    }
    std::fs::remove_dir_all(&folder).map_err(|e| format!("could not delete the template: {e}"))
}

/// Opens a template to edit its order like a service (it saves itself).
#[tauri::command]
pub fn template_open(app: AppHandle, engine: State<'_, Engine>, path: PathBuf) -> CmdResult<()> {
    template_folder(&app, &path)?;
    let Loaded { service, .. } = Service::load_plan(&path).map_err(|e| e.to_string())?;
    engine.load_service(Some(service))
}

/// Deletes a service folder (only inside the app's services folder).
#[tauri::command]
pub fn service_delete(app: AppHandle, engine: State<'_, Engine>, path: PathBuf) -> CmdResult<()> {
    let dir = services_dir(&app)?.canonicalize().map_err(|e| e.to_string())?;
    let folder = path.parent().ok_or("bad path")?.canonicalize().map_err(|e| e.to_string())?;
    if folder.parent() != Some(dir.as_path()) {
        return Err("only services in the app's own folder can be deleted here".into());
    }
    let open_here = engine.service().and_then(|s| s.plan_path.clone()).is_some_and(|p| p.starts_with(&folder));
    if open_here {
        engine.load_service(None)?;
    }
    std::fs::remove_dir_all(&folder).map_err(|e| format!("could not delete {}: {e}", folder.display()))
}

/// Gives the open service (e.g. the example) a home in the app's folder so it
/// is kept and saved automatically from now on.
#[tauri::command]
pub fn service_keep(app: AppHandle, engine: State<'_, Engine>, date: String) -> CmdResult<PathBuf> {
    let date = parse_date(&date)?;
    let dir = services_dir(&app)?;
    let service = engine.service().ok_or("no service is loaded")?;
    let base = format!("{date}-{}", slug_file_name(&service.title, "x").trim_end_matches(".x"));
    let mut folder = dir.join(&base);
    let mut n = 2;
    while folder.exists() {
        folder = dir.join(format!("{base}-{n}"));
        n += 1;
    }
    let plan = folder.join(PLAN_FILE);
    let target = plan.clone();
    engine.edit_service(|s| {
        if s.date.is_none() {
            s.date = Some(date.to_string());
        }
        // Copy the texts into the new folder rather than writing back to
        // wherever they came from.
        for cue in &mut s.cues {
            if let episco_core::CueContent::Liturgy { source, .. } = &mut cue.content {
                *source = None;
            }
        }
        s.save_to(&target).map_err(|e| e.to_string())
    })?;
    Ok(plan)
}

/// The service's Prayers of the People: a Prayer Book form ("1"–"6",
/// "rite1") or the parish's own (a library id, `user:…`).
#[tauri::command]
pub fn service_set_prayers(app: AppHandle, engine: State<'_, Engine>, choice: String) -> CmdResult<()> {
    let doc = if choice.starts_with("user:") {
        crate::library::load(&app, &choice)?
    } else {
        let form: PrayersForm = serde_json::from_value(serde_json::Value::String(choice.clone()))
            .map_err(|_| format!("no form “{choice}”"))?;
        let rite = engine.service().and_then(|s| s.rite).unwrap_or_default();
        builtin_doc(&prayers_item(rite, Some(form))).ok_or("that form is not in the library")?
    };
    engine.edit_service(|s| {
        if !use_prayers_doc(s, doc) {
            return Err("this service has no Prayers of the People to change".to_string());
        }
        s.modified = true;
        s.rebuild_all();
        Ok(())
    })
}

/// The service's Eucharistic Prayer: a Prayer Book prayer ("A"–"D", "I",
/// "II"), the parish's own (`user:…`), or none (the dialogue and the
/// Sanctus only).
#[tauri::command]
pub fn service_set_eucharistic_prayer(
    app: AppHandle,
    engine: State<'_, Engine>,
    choice: Option<String>,
) -> CmdResult<()> {
    let rite = engine.service().and_then(|s| s.rite).unwrap_or_default();
    let doc = match choice.as_deref().filter(|c| !c.is_empty()) {
        None => None,
        Some(id) if id.starts_with("user:") => Some(crate::library::load(&app, id)?),
        Some(name) => {
            let prayer: EucharisticPrayer = serde_json::from_value(serde_json::Value::String(name.into()))
                .map_err(|_| format!("no Eucharistic Prayer “{name}”"))?;
            Some(
                eucharistic_prayer_item(rite, prayer)
                    .and_then(builtin_doc)
                    .ok_or_else(|| format!("Prayer {name} is not in this rite"))?,
            )
        }
    };
    engine.edit_service(|s| {
        let done = match doc {
            Some(doc) => use_eucharistic_doc(s, doc),
            None => use_dialogue(s, rite),
        };
        if !done {
            return Err("this service has no Great Thanksgiving to change".to_string());
        }
        s.modified = true;
        s.rebuild_all();
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(s: &Service) -> Vec<&str> {
        s.cues.iter().map(|c| c.title.as_str()).collect()
    }

    fn with(o: crate::occasions::Options) -> ServiceOptions {
        ServiceOptions { occasion: o, ..Default::default() }
    }

    fn before(t: &[&str], a: &str, b: &str) -> bool {
        let i = t.iter().position(|x| *x == a).unwrap_or_else(|| panic!("no {a} in {t:?}"));
        let j = t.iter().position(|x| *x == b).unwrap_or_else(|| panic!("no {b} in {t:?}"));
        i < j
    }

    #[test]
    fn the_eucharistic_prayer_chosen() {
        let date: Date = "2026-11-08".parse().unwrap();
        let titles = |s: &Service| s.cues.iter().map(|c| c.title.clone()).collect::<Vec<_>>();
        let with = |rite, prayer| {
            build(
                date,
                None,
                None,
                ServiceOptions { rite: Some(rite), eucharistic_prayer: Some(prayer), ..Default::default() },
            )
        };
        let usual = titles(&build(date, None, None, ServiceOptions::default()));
        let gt = usual.iter().position(|t| t == "The Great Thanksgiving").unwrap();
        assert_eq!(usual[gt + 1], "Sanctus");

        let a = with(Rite::Two, EucharisticPrayer::A);
        let t = titles(&a);
        assert_eq!(t[gt], "Eucharistic Prayer A");
        assert_ne!(t[gt + 1], "Sanctus", "the prayer has the Sanctus");
        assert_eq!(t.len(), usual.len() - 1);
        let cue = &a.cues[gt];
        assert_eq!(cue.category, ElementCategory::EucharisticPrayer);
        assert!(a.cues.iter().filter(|c| c.id == cue.id).count() == 1);
        // Its Proper Preface is said (the Lord's Day, after Pentecost).
        let text: String = (0..cue.slide_count())
            .filter_map(|i| cue.slide(i))
            .flat_map(|s| s.blocks.iter().map(episco_core::Block::plain_text))
            .collect();
        assert!(text.contains("For you are the source of light and life"));

        let two = with(Rite::One, EucharisticPrayer::Two);
        assert_eq!(titles(&two)[gt], "Eucharistic Prayer II");
        // A prayer the rite doesn't have: as usual.
        assert_eq!(
            titles(&with(Rite::One, EucharisticPrayer::A)),
            titles(&build(date, None, None, ServiceOptions { rite: Some(Rite::One), ..Default::default() }))
        );
    }

    #[test]
    fn prayers_of_the_people_follow_the_creed() {
        let next = |s: &Service, title: &str| {
            let t = titles(s);
            let i = t.iter().position(|x| *x == title).unwrap();
            t[i + 1].to_string()
        };
        let two = build("2026-11-15".parse().unwrap(), None, None, ServiceOptions::default());
        assert_eq!(next(&two, "The Nicene Creed"), "The Prayers of the People (Form I)");
        assert_eq!(next(&two, "The Prayers of the People (Form I)"), "Confession of Sin");
        let one = build(
            "2026-11-15".parse().unwrap(),
            None,
            None,
            ServiceOptions { rite: Some(Rite::One), ..Default::default() },
        );
        assert!(titles(&one).contains(&"The Prayers of the People (Rite I)"), "{:?}", titles(&one));
        let form3 = build(
            "2026-11-15".parse().unwrap(),
            None,
            None,
            ServiceOptions { prayers: Some(PrayersForm::III), ..Default::default() },
        );
        assert!(titles(&form3).contains(&"The Prayers of the People (Form III)"));
        // Form VI ends with its own confession; the slides and bulletin have it in full.
        let doc = builtin_doc("prayers-of-the-people-6").unwrap();
        assert!(doc.blocks.iter().any(|b| b.role() == Some(&Role::All)));
        assert_eq!(doc.meta.extra("bulletin"), Some("full"));
        let opts: ServiceOptions = serde_json::from_str(r#"{"prayers": "rite1"}"#).unwrap();
        assert_eq!(opts.prayers, Some(PrayersForm::RiteOne));
    }

    #[test]
    fn holy_baptism_replaces_the_creed_prayers_and_confession() {
        use crate::occasions::{Baptismal, Options};
        let s = build(
            "2026-11-15".parse().unwrap(),
            None,
            None,
            with(Options { baptismal: Some(Baptismal::Baptism), ..Default::default() }),
        );
        let t = titles(&s);
        for gone in
            ["The Nicene Creed", "Confession of Sin", "Absolution", "The Peace", "Collect for Purity", "Salutation"]
        {
            assert!(!t.contains(&gone), "{gone} should be gone: {t:?}");
        }
        assert_eq!(t[1], "Opening Acclamation (Holy Baptism)");
        assert!(before(&t, "The Sermon", "Presentation and Examination of the Candidates"));
        assert!(before(&t, "Presentation and Examination of the Candidates", "The Baptismal Covenant"));
        assert!(before(&t, "The Baptismal Covenant", "Prayers for the Candidates"));
        assert!(before(&t, "Thanksgiving over the Water", "The Baptism"));
        assert!(!t.contains(&"Consecration of the Chrism"), "only with a bishop");
        assert!(!s.bishop);
        // The propers are still there, before the sermon.
        assert!(before(&t, "Collect of the Day", "The Sermon"));
    }

    #[test]
    fn baptism_with_a_bishop_and_applying_twice() {
        use crate::occasions::{apply, Baptismal, Occasion, Options};
        let o = Options { bishop: true, baptismal: Some(Baptismal::Baptism), confirmation: true, ..Default::default() };
        let mut s = build("2026-11-15".parse().unwrap(), None, None, with(o));
        let t = titles(&s);
        assert!(before(&t, "Thanksgiving over the Water", "Consecration of the Chrism"));
        assert!(before(&t, "The Baptism", "At Confirmation, Reception, or Reaffirmation"));
        let n = s.cues.len();
        // Adding the same options to the service again changes nothing.
        apply(&mut s, Options { occasion: Occasion::Eucharist, ..o });
        assert_eq!(s.cues.len(), n, "{:?}", titles(&s));
        // Renewal after a baptism is not added either.
        apply(&mut s, Options { baptismal: Some(Baptismal::Renewal), ..Default::default() });
        assert_eq!(s.cues.len(), n);
    }

    #[test]
    fn renewal_of_vows_takes_the_place_of_the_creed() {
        use crate::occasions::{Baptismal, Options};
        let s = build(
            "2026-11-15".parse().unwrap(),
            None,
            None,
            with(Options { baptismal: Some(Baptismal::Renewal), ..Default::default() }),
        );
        let t = titles(&s);
        assert!(!t.contains(&"The Nicene Creed"));
        assert!(before(&t, "The Sermon", "The Renewal of Baptismal Vows"));
        assert!(t.contains(&"Confession of Sin"));
    }

    #[test]
    fn a_bishop_presides_with_confirmation() {
        use crate::occasions::Options;
        let s = build(
            "2026-11-15".parse().unwrap(),
            None,
            None,
            with(Options { bishop: true, confirmation: true, ..Default::default() }),
        );
        assert!(s.bishop);
        let t = titles(&s);
        assert!(before(&t, "The Sermon", "Presentation and Examination of the Candidates (Confirmation)"));
        assert!(before(&t, "The Baptismal Covenant", "Confirmation, Reception, Reaffirmation"));
        assert!(before(&t, "The Bishop's Blessing", "Dismissal (Let us go forth)"));
        assert!(!t.contains(&"The Nicene Creed") && !t.contains(&"Confession of Sin"));
        // The Celebrant's parts are the Bishop's on the slides…
        let roles: Vec<String> = s
            .cues
            .iter()
            .flat_map(|c| (0..c.slide_count()).filter_map(move |i| c.slide(i)))
            .flat_map(|sl| sl.blocks.iter().filter_map(|b| b.role().map(|r| r.label().to_string())))
            .collect();
        assert!(roles.iter().any(|r| r == "Bishop"));
        assert!(!roles.iter().any(|r| r == "Celebrant" || r == "Priest"), "{roles:?}");
        // …but the texts themselves are unchanged.
        let salutation = s.cues.iter().find(|c| c.title == "Salutation").unwrap();
        assert!(salutation.document().unwrap().blocks.iter().any(|b| b.role() == Some(&Role::Celebrant)));
        // The plan keeps the setting.
        let dir = std::env::temp_dir().join(format!("episco-bishop-{}", std::process::id()));
        let mut copy = s.clone();
        copy.save_to(&dir.join("service.toml")).unwrap();
        let again = Service::load_plan(dir.join("service.toml")).unwrap().service;
        assert!(again.bishop);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn new_services_get_the_usual_gospel_music() {
        let mut s = build("2026-11-15".parse().unwrap(), None, None, Default::default());
        with_gospel_music(&mut s, &("library:media/alleluia.png".into(), "Alleluia".into()));
        let g = s.cues.iter().find(|c| c.category == episco_core::ElementCategory::Gospel).unwrap();
        let first = g.slide(0).unwrap();
        assert_eq!(first.heading.as_deref(), Some(episco_core::propers::GOSPEL_MUSIC));
        assert!(matches!(&first.blocks[0], Block::Image { src, .. } if src == "library:media/alleluia.png"));
        assert!(g.slide(1).unwrap().blocks[0].plain_text().starts_with("The Holy Gospel of our Lord"));
        assert!(g.slide(2).unwrap().is_title());
        // Not twice.
        let before = g.document().unwrap().clone();
        with_gospel_music(&mut s, &("library:media/other.png".into(), String::new()));
        let g = s.cues.iter().find(|c| c.category == episco_core::ElementCategory::Gospel).unwrap();
        assert_eq!(g.document().unwrap(), &before);
    }

    #[test]
    fn an_event_starts_with_blank_slides() {
        use crate::occasions::{Occasion, Options};
        let s = build(
            "2026-11-20".parse().unwrap(),
            Some("Parish Dinner".into()),
            None,
            with(Options { occasion: Occasion::Event, ..Default::default() }),
        );
        assert_eq!(s.title, "Parish Dinner");
        assert_eq!(s.cues.len(), 1, "{:?}", titles(&s));
        let doc = s.cues[0].document().unwrap();
        assert!(doc.meta.is_made_slides() && doc.blocks.is_empty());
        assert!(s.liturgy.is_none());
        let untitled = build(
            "2026-11-20".parse().unwrap(),
            None,
            None,
            with(Options { occasion: Occasion::Event, ..Default::default() }),
        );
        assert_eq!(untitled.title, "Event");
    }

    #[test]
    fn no_deacon_gives_the_deacons_parts_to_a_priest() {
        use crate::occasions::Options;
        let s =
            build("2026-11-15".parse().unwrap(), None, None, with(Options { no_deacon: true, ..Default::default() }));
        assert!(s.no_deacon);
        let roles: Vec<String> = s
            .cues
            .iter()
            .flat_map(|c| (0..c.slide_count()).filter_map(move |i| c.slide(i)))
            .flat_map(|sl| sl.blocks.iter().filter_map(|b| b.role().map(|r| r.label().to_string())))
            .collect();
        assert!(roles.iter().any(|r| r == "Priest"));
        assert!(!roles.iter().any(|r| r == "Deacon"), "{roles:?}");
        let dir = std::env::temp_dir().join(format!("episco-deacon-{}", std::process::id()));
        let mut copy = s.clone();
        copy.save_to(&dir.join("service.toml")).unwrap();
        assert!(Service::load_plan(dir.join("service.toml")).unwrap().service.no_deacon);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn holy_week_and_ordinations() {
        use crate::occasions::{suggested, Occasion, Options};
        // Easter 2027 is March 28.
        let day = |d: &str| -> Date { d.parse().unwrap() };
        assert_eq!(suggested(day("2027-02-10")), Occasion::AshWednesday);
        assert_eq!(suggested(day("2027-03-21")), Occasion::PalmSunday);
        assert_eq!(suggested(day("2027-03-25")), Occasion::MaundyThursday);
        assert_eq!(suggested(day("2027-03-26")), Occasion::GoodFriday);
        assert_eq!(suggested(day("2027-03-27")), Occasion::HolySaturday);
        assert_eq!(suggested(day("2027-03-28")), Occasion::Eucharist);

        let cites = |s: &Service, c: &str| {
            s.cues.iter().any(|x| x.document().and_then(|d| d.meta.citation.as_deref()).is_some_and(|x| x.contains(c)))
        };
        let make =
            |date: &str, occasion| build(day(date), None, None, with(Options { occasion, ..Default::default() }));

        let ash = make("2027-02-10", Occasion::AshWednesday);
        let t = titles(&ash);
        assert_eq!(ash.title, "Ash Wednesday");
        assert!(before(&t, "The Sermon", "Invitation to a Holy Lent"));
        assert!(before(&t, "Imposition of Ashes", "Litany of Penitence"));
        assert!(t.iter().any(|x| x.starts_with("Psalm 51")), "{t:?}");

        let palms = make("2027-03-21", Occasion::PalmSunday);
        let t = titles(&palms);
        assert_eq!(t[0], "The Liturgy of the Palms");
        let gospel = palms.cues.iter().position(|c| c.category == ElementCategory::Gospel).unwrap();
        assert_eq!(palms.cues[gospel - 1].title, "The Passion Gospel", "{t:?}");

        let thursday = make("2027-03-25", Occasion::MaundyThursday);
        assert!(before(&titles(&thursday), "The Sermon", "The Washing of Feet"));

        let friday = make("2027-03-26", Occasion::GoodFriday);
        let t = titles(&friday);
        assert_eq!(t[0], "Good Friday: Opening and Collect");
        assert!(!t.contains(&"Collect of the Day"), "the opening has the collect: {t:?}");
        assert!(before(&t, "The Passion Gospel (John)", "The Solemn Collects"));
        assert!(t.contains(&"Good Friday: Concluding Prayer") && !t.contains(&"Dismissal (Let us go forth)"));

        let saturday = make("2027-03-27", Occasion::HolySaturday);
        let t = titles(&saturday);
        assert!(t.contains(&"Collect of the Day") && cites(&saturday, "Job 14:1-14"), "{t:?}");
        assert!(before(&t, "The Sermon", "In the Midst of Life"));

        let vigil = make("2027-03-27", Occasion::EasterVigil);
        let t = titles(&vigil);
        assert_eq!(vigil.title, "The Great Vigil of Easter");
        assert!(before(&t, "The Exsultet", "The Vigil: Liturgy of the Word"));
        assert!(before(&t, "The Vigil: Liturgy of the Word", "The Renewal of Baptismal Vows"));
        assert!(before(&t, "The Renewal of Baptismal Vows", "The Vigil: At the Eucharist"));
        assert!(cites(&vigil, "Mark 16:1-8"), "Year B Gospel at the 2027 Vigil: {t:?}");

        for (occasion, who) in [
            (Occasion::OrdinationDeacon, "Deacon"),
            (Occasion::OrdinationPriest, "Priest"),
            (Occasion::OrdinationBishop, "Bishop"),
        ] {
            let s = make("2027-06-12", occasion);
            let t = titles(&s);
            assert!(s.bishop, "a bishop ordains");
            assert_eq!(
                s.title,
                format!("The Ordination of a {}", who.to_lowercase())
                    .replacen("a deacon", "a Deacon", 1)
                    .replacen("a priest", "a Priest", 1)
                    .replacen("a bishop", "a Bishop", 1)
            );
            assert!(before(&t, &format!("Ordination of a {who}: The Presentation"), "The Litany for Ordinations"));
            assert!(before(&t, "The Litany for Ordinations", "Collect after the Litany (Ordinations)"));
            assert!(before(&t, "The Sermon", &format!("Ordination of a {who}: The Examination")));
            assert!(before(&t, &format!("Ordination of a {who}: The Consecration"), "Offertory Hymn"));
            assert_eq!(t.contains(&"The Nicene Creed"), who != "Bishop");
            assert!(t.contains(&"The Holy Gospel") && cites(&s, "Psalm"), "{t:?}");
            assert!(!t.contains(&"The Bishop's Blessing"), "the ordination has its own blessing");
        }
    }

    /// With EPISCO_SAMPLE_OUT set, writes bulletins and scripts of the
    /// special services there, to look at.
    #[test]
    fn sample_bulletins() {
        use crate::occasions::{Baptismal, Occasion, Options};
        let Ok(out) = std::env::var("EPISCO_SAMPLE_OUT") else { return };
        let parish = episco_core::bulletin::ParishInfo::default();
        for (name, date, o) in [
            (
                "baptism-bishop",
                "2026-11-15",
                Options { bishop: true, baptismal: Some(Baptismal::Baptism), confirmation: true, ..Default::default() },
            ),
            ("good-friday", "2027-03-26", Options { occasion: Occasion::GoodFriday, ..Default::default() }),
            ("ordination-priest", "2027-06-12", Options { occasion: Occasion::OrdinationPriest, ..Default::default() }),
            ("vigil", "2027-03-27", Options { occasion: Occasion::EasterVigil, ..Default::default() }),
            ("holy-saturday", "2027-03-27", Options { occasion: Occasion::HolySaturday, ..Default::default() }),
        ] {
            let s = build(date.parse().unwrap(), None, None, with(o));
            for (kind, doc) in [
                ("bulletin", episco_core::bulletin::bulletin(&s, &parish)),
                ("script", episco_core::bulletin::script(&s, &parish)),
            ] {
                std::fs::write(format!("{out}/{name}-{kind}.html"), episco_core::bulletin::to_html(&doc)).unwrap();
            }
        }
    }

    #[test]
    fn a_regular_service_uses_only_its_music() {
        use MusicPlace as M;
        let date = "2026-11-15".parse().unwrap();
        // The 8:00: the closing hymn and postlude only.
        let mut early = build(date, None, None, ServiceOptions::default());
        apply_music(&mut early, &[M::ClosingHymn, M::Postlude]);
        let t = titles(&early);
        for gone in ["Entrance Hymn", "Offertory Hymn", "Communion Hymn", "Hymn of Praise"] {
            assert!(!t.contains(&gone), "{gone} left out: {t:?}");
        }
        assert!(t.contains(&"Closing Hymn"));
        assert!(t.iter().any(|x| x.contains("Gloria")), "the Gloria said, under its own name: {t:?}");
        assert_eq!(early.prelude.as_ref().map(|m| m.style()), Some(episco_core::bulletin::BulletinStyle::Hidden));
        assert!(early.postlude.is_none(), "the postlude waits to be named");
        // The 10:30: everything, with a Gradual, a Sequence and an anthem.
        let mut main = build(date, None, None, ServiceOptions::default());
        apply_music(&mut main, &M::ALL);
        let t = titles(&main);
        let gospel = main.cues.iter().position(|c| c.category == ElementCategory::Gospel).unwrap();
        assert_eq!(&t[gospel - 2..gospel], ["Gradual Hymn", "Sequence Hymn"]);
        let peace = t.iter().position(|x| *x == "The Peace").unwrap();
        assert_eq!(t[peace + 1], "Offertory Anthem");
        assert_eq!(t[peace + 2], "Offertory Hymn");
        assert!(main.prelude.is_none());
        // No music.
        let mut said = build(date, None, None, ServiceOptions::default());
        apply_music(&mut said, &[]);
        let t = titles(&said);
        assert!(M::ALL.iter().all(|p| !t.contains(&p.label())), "{t:?}");
        assert!(said.prelude.is_some() && said.postlude.is_some(), "both left out of the bulletin");
    }

    #[test]
    fn standard_order_follows_the_season() {
        let advent = build("2026-11-29".parse().unwrap(), None, None, ServiceOptions::default());
        assert_eq!(advent.title, "The First Sunday of Advent");
        assert_eq!(advent.color, Some(episco_core::LiturgicalColor::Blue));
        assert!(titles(&advent).contains(&"Kyrie"));
        assert!(!titles(&advent).contains(&"Hymn of Praise"), "no Gloria in Advent");

        let easter = build("2027-04-04".parse().unwrap(), None, None, ServiceOptions::default());
        assert!(titles(&easter).contains(&"Hymn of Praise"));
        let acclamation = easter.cues.iter().find(|c| c.category == ElementCategory::Acclamation).unwrap();
        assert!(acclamation.document().unwrap().blocks[0].plain_text().contains("Christ is risen"));

        let ordinary = build("2026-11-15".parse().unwrap(), Some("  ".into()), None, ServiceOptions::default());
        assert_eq!(ordinary.title, "The Twenty-Fifth Sunday after Pentecost", "blank title falls back to the day");
        assert!(ordinary.cues.len() > 15);
        let t = titles(&ordinary);
        let post = t.iter().position(|x| x.starts_with("Postcommunion")).unwrap();
        assert_eq!(t[post + 1], "Welcome & Announcements", "announcements follow the Postcommunion Prayer");
        assert_eq!(t[0], "Entrance Hymn");
        assert!(ordinary.cues.iter().all(|c| c.source().is_none()));
    }

    #[test]
    fn copies_drop_propers_and_get_their_own_files() {
        let no_propers = ServiceOptions { propers: Some(false), ..Default::default() };
        let mut template = build("2026-11-15".parse().unwrap(), None, None, no_propers.clone());
        let reading: LiturgyDocument =
            "---\ntitle: First Reading\ntype: reading\ncitation: Joshua 24\n---\n# The First Reading\n"
                .parse()
                .unwrap();
        let collect: LiturgyDocument =
            "---\ntitle: Collect of the Day\ntype: collect\n---\n[Celebrant] O God.\n".parse().unwrap();
        template.insert_cue(5, Cue::liturgy("", reading, Some("/old/reading.ewm".into()), CueOverrides::default()));
        template.insert_cue(5, Cue::liturgy("", collect, Some("/old/collect.ewm".into()), CueOverrides::default()));
        template.cues[2].content = match template.cues[2].content.clone() {
            episco_core::CueContent::Liturgy { document, slides, stream_slides, stream_at, .. } => {
                episco_core::CueContent::Liturgy {
                    source: Some("/old/x.ewm".into()),
                    document,
                    slides,
                    stream_slides,
                    stream_at,
                }
            }
            other => other,
        };
        let n = template.cues.len();

        let copy = build("2026-11-22".parse().unwrap(), None, Some(template), no_propers);
        assert_eq!(copy.cues.len(), n - 2, "reading and Collect of the Day dropped");
        assert!(!titles(&copy).contains(&"Collect of the Day"));
        assert!(titles(&copy).contains(&"Collect for Purity"), "fixed collects are kept");
        assert!(copy.cues.iter().all(|c| c.source().is_none()), "the copy never writes to the old files");
        assert_eq!(copy.title, "The Last Sunday after Pentecost: Christ the King");
        assert_eq!(copy.date.as_deref(), Some("2026-11-22"));
    }

    #[test]
    fn new_services_get_the_days_propers_in_their_rite() {
        // Proper 28, Year A, Rite II, Track 1.
        let s = build("2026-11-15".parse().unwrap(), None, None, ServiceOptions::default());
        let t = titles(&s);
        let sermon = t.iter().position(|x| *x == "The Sermon").unwrap();
        assert_eq!(
            &t[sermon - 5..sermon],
            ["Collect of the Day", "First Reading", "Psalm 123", "Second Reading", "The Holy Gospel"]
        );
        let collect = s.cues.iter().find(|c| c.title == "Collect of the Day").unwrap().document().unwrap();
        assert!(collect
            .blocks
            .iter()
            .any(|b| b.plain_text().starts_with("Blessed Lord, who caused all holy Scriptures")));
        let reading = s.cues.iter().find(|c| c.title == "First Reading").unwrap().document().unwrap();
        assert_eq!(reading.meta.citation.as_deref(), Some("Judges 4:1-7"));
        assert_eq!(s.rite, None);
        let psalm = s.cues.iter().find(|c| c.category == ElementCategory::Psalm).unwrap();
        assert_eq!(propers::psalm_verses(psalm.document().unwrap()).len(), 5);

        // Rite I, Track 2.
        let opts = ServiceOptions {
            rite: Some(Rite::One),
            track: Some(2),
            bible: Some(BibleVersion::Nrsv),
            propers: None,
            by_season: false,
            occasion: Default::default(),
            prayers: None,
            eucharistic_prayer: None,
            ..Default::default()
        };
        let s = build("2026-11-15".parse().unwrap(), None, None, opts);
        let t = titles(&s);
        assert!(t.contains(&"Prayer of Humble Access"));
        let text = |title: &str| {
            s.cues
                .iter()
                .find(|c| c.title == title)
                .unwrap()
                .document()
                .unwrap()
                .blocks
                .iter()
                .map(|b| b.plain_text())
                .collect::<Vec<_>>()
                .join(" ")
        };
        assert!(text("Salutation").contains("And with thy spirit."));
        assert!(text("Collect of the Day").contains("Blessed Lord, who hast caused"));
        assert!(text("The Holy Gospel").contains("Glory be to thee, O Lord."));
        assert_eq!(
            s.cues.iter().find(|c| c.title == "First Reading").unwrap().document().unwrap().meta.citation.as_deref(),
            Some("Zephaniah 1:7, 12-18")
        );
        assert!(t.iter().any(|x| x.starts_with("Psalm 90")));

        // The settings are kept in the saved plan.
        let dir = std::env::temp_dir().join(format!("episco-rite-test-{}", std::process::id()));
        let plan = dir.join(PLAN_FILE);
        let mut s = s;
        s.save_to(&plan).unwrap();
        let back = Service::load_plan(&plan).unwrap().service;
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!((back.rite, back.track, back.bible), (Some(Rite::One), Some(2), Some(BibleVersion::Nrsv)));
        assert_eq!(back.cues.len(), s.cues.len());
    }

    #[test]
    fn templates_keep_the_order_without_propers() {
        let s = build("2026-11-15".parse().unwrap(), None, None, ServiceOptions::default());
        assert!(titles(&s).contains(&"Collect of the Day"));
        let t = as_template(&s, "Ordinary Time");
        assert_eq!(t.title, "Ordinary Time");
        assert!(t.date.is_none());
        assert!(!titles(&t).contains(&"Collect of the Day") && !titles(&t).iter().any(|x| x.starts_with("Psalm")));
        assert!(t.cues.iter().all(|c| c.source().is_none()));

        // A service from the template gets the new day's propers in place.
        let lent = build("2027-03-14".parse().unwrap(), None, Some(t), ServiceOptions::default());
        let titles = titles(&lent);
        let sermon = titles.iter().position(|x| *x == "The Sermon").unwrap();
        assert_eq!(titles[sermon - 1], "The Holy Gospel");
        assert_eq!(lent.title, "The Fifth Sunday in Lent");
    }

    #[test]
    fn a_parish_prayer_takes_the_place_of_the_prayer_books() {
        let mut s = build("2026-11-15".parse().unwrap(), None, None, ServiceOptions::default());
        let n = s.cues.len();
        // The parish's Eucharistic Prayer: the dialogue and the Sanctus give way.
        let ep: LiturgyDocument = "---\ntitle: Eucharistic Prayer 1 (EOW)\ntype: eucharistic_prayer\n---\n[Celebrant] The Lord be with you.\n[People] **And also with you.**\n[Celebrant] Let us give thanks to the Lord our God.\n[People] **It is right to give our thanks and praise.**\n\n[Celebrant] It is truly right to glorify you.\n\n[Celebrant] Therefore we praise you.\n".parse().unwrap();
        assert!(use_eucharistic_doc(&mut s, ep));
        s.rebuild_all();
        assert_eq!(s.cues.len(), n - 1, "the separate Sanctus is left out");
        assert!(!s.cues.iter().any(|c| is_item(c, "sursum-corda") || is_item(c, "sanctus")));
        let cue = s.cues.iter().find(|c| c.category == ElementCategory::EucharisticPrayer).unwrap();
        assert_eq!(cue.title, "Eucharistic Prayer 1 (EOW)");
        // The Proper Preface goes in on the slides (the date's, unless left out).
        let text: String = (0..cue.slide_count())
            .flat_map(|i| cue.slide(i).unwrap().blocks.iter().map(Block::plain_text).collect::<Vec<_>>())
            .collect::<Vec<_>>()
            .join(" ");
        let preface = s.preface_entry().unwrap().text(Rite::Two).unwrap();
        assert!(text.contains(&preface[..20]), "{text}");
        s.preface = Some("none".into());
        s.rebuild_all();
        let cue = s.cues.iter().find(|c| c.category == ElementCategory::EucharisticPrayer).unwrap();
        let text: String =
            (0..cue.slide_count()).flat_map(|i| cue.slide(i).unwrap().blocks.iter().map(Block::plain_text)).collect();
        assert!(!text.contains(&preface[..20]), "left out");
        // Back to the dialogue and the Sanctus.
        assert!(use_dialogue(&mut s, Rite::Two));
        assert_eq!(s.cues.len(), n);
        let at = s.cues.iter().position(|c| is_item(c, "sursum-corda")).unwrap();
        assert!(is_item(&s.cues[at + 1], "sanctus"));

        // The parish's Prayers of the People, then a Prayer Book form again.
        let mut pop: LiturgyDocument =
            "---\ntitle: Prayers of the People (St. James)\ntype: prayer\n---\n[Leader] Let us pray.\n"
                .parse()
                .unwrap();
        pop.meta.extra.push((crate::library::USES.into(), crate::library::PRAYERS_OF_THE_PEOPLE.into()));
        assert!(use_prayers_doc(&mut s, pop));
        assert!(s.cues.iter().any(|c| c.title == "Prayers of the People (St. James)"));
        assert!(use_prayers_doc(&mut s, builtin_doc("prayers-of-the-people-3").unwrap()), "found again");
        assert!(s.cues.iter().any(|c| c.title == "The Prayers of the People (Form III)"));
        assert_eq!(s.cues.len(), n);
    }
}
