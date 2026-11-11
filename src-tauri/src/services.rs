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
use episco_core::service::Loaded;
use episco_core::{lectionary, propers};
use episco_core::{
    AnnouncementMode, Block, Cue, CueOverrides, ElementCategory, FrontMatter, LiturgyDocument, Role, Season, Service,
    Span,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::commands::slug_file_name;
use crate::engine::Engine;
use crate::library::builtin_doc;

type CmdResult<T> = Result<T, String>;

pub const PLAN_FILE: &str = "service.toml";

pub fn services_dir(app: &AppHandle) -> CmdResult<PathBuf> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join("services"))
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

#[derive(Serialize)]
pub struct ServiceSummary {
    path: PathBuf,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    day: Option<LiturgicalDay>,
    cues: usize,
    /// No reading or Gospel yet: the propers still need importing.
    needs_propers: bool,
}

fn summary(path: PathBuf, service: &Service) -> ServiceSummary {
    let day = service.date.as_deref().and_then(|d| d.parse::<Date>().ok()).map(liturgical_day);
    ServiceSummary {
        path,
        title: service.title.clone(),
        date: service.date.clone(),
        day,
        cues: service.cues.len(),
        needs_propers: !service
            .cues
            .iter()
            .any(|c| matches!(c.category, ElementCategory::Reading | ElementCategory::Gospel)),
    }
}

/// Every service in the app's folder, by date (undated last).
#[tauri::command]
pub fn services_list(app: AppHandle) -> CmdResult<Vec<ServiceSummary>> {
    let dir = services_dir(&app)?;
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(out) };
    for entry in entries.flatten() {
        let plan = entry.path().join(PLAN_FILE);
        if !plan.is_file() {
            continue;
        }
        match Service::load_plan(&plan) {
            Ok(Loaded { service, .. }) => out.push(summary(plan, &service)),
            Err(e) => tracing::warn!("skipping service {}: {e}", plan.display()),
        }
    }
    out.sort_by(|a, b| match (&a.date, &b.date) {
        (Some(x), Some(y)) => x.cmp(y).then(a.title.cmp(&b.title)),
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

/// The library item for `name` in `rite` (Rite I has its own forms).
fn rite_name(rite: Rite, name: &str) -> String {
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
fn standard_cues(day: &LiturgicalDay, rite: Rite) -> Vec<Cue> {
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
    [
        Some(Cue::announcements("", AnnouncementMode::SinglePass, CueOverrides::default())).map(|mut c| {
            c.overrides.title = Some("Welcome & Announcements".into());
            c
        }),
        hymn("Entrance Hymn"),
        lib(acclamation, None),
        lib("collect-for-purity", None),
        song_of_praise,
        lib("salutation", None),
        Some(Cue::liturgy("", sermon(), None, CueOverrides::default())),
        lib("nicene-creed", None),
        lib("confession", None),
        lib("absolution", None),
        lib("the-peace", None),
        hymn("Offertory Hymn"),
        lib("sursum-corda", None),
        lib("sanctus", None),
        lib("lords-prayer", None),
        lib("fraction-anthem", None),
        humble_access,
        lib("invitation", None),
        hymn("Communion Hymn"),
        lib("postcommunion-eternal-god", None),
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
#[derive(Debug, Clone, Copy, Default, Deserialize)]
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
            for cue in standard_cues(&day, rite) {
                let at = s.cues.len();
                s.insert_cue(at, cue);
            }
            s
        }
    };
    if opts.propers.unwrap_or(true) {
        let at = propers_position(&service);
        for (k, cue) in proper_cues(date, rite, track).into_iter().enumerate() {
            service.insert_cue(at + k, cue);
        }
    }
    service.rite = (rite == Rite::One).then_some(rite);
    service.track = (track == 2).then_some(2);
    service.bible = bible;
    service.title = title.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).unwrap_or_else(|| day.name.clone());
    service.date = Some(date.to_string());
    service.season = Some(day.season);
    service.color = Some(day.color);
    service.plan_path = None;
    service.rebuild_all();
    service
}

/// Builds and saves a service for `date` in the app's folder.
fn create(
    app: &AppHandle,
    date: Date,
    title: Option<String>,
    template: Option<&Path>,
    opts: ServiceOptions,
) -> CmdResult<PathBuf> {
    let template = match template {
        Some(path) => Some(Service::load_plan(path).map_err(|e| e.to_string())?.service),
        None => None,
    };
    let mut service = build(date, title, template, opts);

    let dir = services_dir(app)?;
    let base = format!("{date}-{}", slug_file_name(&service.title, "x").trim_end_matches(".x"));
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
) -> CmdResult<PathBuf> {
    let plan = create(&app, parse_date(&date)?, title, template.as_deref(), options.unwrap_or_default())?;
    let Loaded { service, .. } = Service::load_plan(&plan).map_err(|e| e.to_string())?;
    engine.load_service(Some(service))?;
    Ok(plan)
}

/// Creates services for the next `weeks` Sundays from `from` (a date) that
/// do not have one yet. Returns the new plans.
#[tauri::command]
pub fn services_schedule(
    app: AppHandle,
    from: String,
    weeks: u32,
    template: Option<PathBuf>,
    options: Option<ServiceOptions>,
) -> CmdResult<Vec<PathBuf>> {
    let options = options.unwrap_or_default();
    let existing: Vec<String> = services_list(app.clone())?.into_iter().filter_map(|s| s.date).collect();
    let first = parse_date(&from)?.next_sunday();
    let mut created = Vec::new();
    for w in 0..weeks.min(26) {
        let date = first.add_days(7 * i64::from(w));
        if existing.contains(&date.to_string()) {
            continue;
        }
        created.push(create(&app, date, None, template.as_deref(), options)?);
    }
    Ok(created)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(s: &Service) -> Vec<&str> {
        s.cues.iter().map(|c| c.title.as_str()).collect()
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
        assert!(ordinary.cues.iter().all(|c| c.source().is_none()));
    }

    #[test]
    fn copies_drop_propers_and_get_their_own_files() {
        let no_propers = ServiceOptions { propers: Some(false), ..Default::default() };
        let mut template = build("2026-11-15".parse().unwrap(), None, None, no_propers);
        let reading: LiturgyDocument =
            "---\ntitle: First Reading\ntype: reading\ncitation: Joshua 24\n---\n# The First Reading\n"
                .parse()
                .unwrap();
        let collect: LiturgyDocument =
            "---\ntitle: Collect of the Day\ntype: collect\n---\n[Celebrant] O God.\n".parse().unwrap();
        template.insert_cue(5, Cue::liturgy("", reading, Some("/old/reading.ewm".into()), CueOverrides::default()));
        template.insert_cue(5, Cue::liturgy("", collect, Some("/old/collect.ewm".into()), CueOverrides::default()));
        template.cues[2].content = match template.cues[2].content.clone() {
            episco_core::CueContent::Liturgy { document, slides, .. } => {
                episco_core::CueContent::Liturgy { source: Some("/old/x.ewm".into()), document, slides }
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
        let opts =
            ServiceOptions { rite: Some(Rite::One), track: Some(2), bible: Some(BibleVersion::Nrsv), propers: None };
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
}
