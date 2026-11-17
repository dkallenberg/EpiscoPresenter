//! Services other than the Sunday Eucharist, and the options that change a
//! service: a bishop presiding, Holy Baptism, the Renewal of Baptismal Vows,
//! and Confirmation.
//!
//! Every text comes from the built-in library (`library/occasions/`, from
//! the 1979 Prayer Book). The orders follow the Prayer Book's rubrics:
//! Holy Baptism (BCP 299) replaces the Creed, the Prayers of the People and
//! the Confession with the Presentation, the Baptismal Covenant, the
//! Prayers for the Candidates, the Thanksgiving over the Water and the
//! Baptism; the Renewal of Baptismal Vows (BCP 292) takes the place of the
//! Creed; Confirmation (BCP 413) follows the same pattern with the bishop.
//! Marriage (BCP 423) and the Burial of the Dead (BCP 469, 491) have their
//! own orders, with or without the Holy Communion.

use episco_core::bcp::{self, Rite};
use episco_core::calendar::{easter, liturgical_day, Date};
use episco_core::propers::{self, ProperItem, ProperKind};
use episco_core::{AnnouncementMode, Cue, CueOverrides, ElementCategory, Service};
use serde::{Deserialize, Serialize};

use crate::library::builtin_doc;
use crate::services::{hymn_cue, rite_name, sermon_cue};

/// What kind of service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Occasion {
    /// The Holy Eucharist of the day (Sundays and holy days).
    #[default]
    Eucharist,
    AshWednesday,
    PalmSunday,
    MaundyThursday,
    GoodFriday,
    HolySaturday,
    EasterVigil,
    OrdinationDeacon,
    OrdinationPriest,
    OrdinationBishop,
    /// The Celebration and Blessing of a Marriage (BCP 423).
    Marriage,
    /// The Burial of the Dead, in the service's rite (BCP 469, 491).
    Burial,
    /// Not a service (a meeting, a concert, a parish dinner): one blank
    /// item of slides to make.
    Event,
}

impl Occasion {
    pub const ALL: [Occasion; 13] = [
        Occasion::Eucharist,
        Occasion::AshWednesday,
        Occasion::PalmSunday,
        Occasion::MaundyThursday,
        Occasion::GoodFriday,
        Occasion::HolySaturday,
        Occasion::EasterVigil,
        Occasion::OrdinationDeacon,
        Occasion::OrdinationPriest,
        Occasion::OrdinationBishop,
        Occasion::Marriage,
        Occasion::Burial,
        Occasion::Event,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Occasion::Eucharist => "The Holy Eucharist",
            Occasion::AshWednesday => "Ash Wednesday",
            Occasion::PalmSunday => "Palm Sunday",
            Occasion::MaundyThursday => "Maundy Thursday",
            Occasion::GoodFriday => "Good Friday",
            Occasion::HolySaturday => "Holy Saturday",
            Occasion::EasterVigil => "The Great Vigil of Easter",
            Occasion::OrdinationDeacon => "The Ordination of a Deacon",
            Occasion::OrdinationPriest => "The Ordination of a Priest",
            Occasion::OrdinationBishop => "The Ordination of a Bishop",
            Occasion::Marriage => "The Celebration and Blessing of a Marriage",
            Occasion::Burial => "The Burial of the Dead",
            Occasion::Event => "Event (not a service)",
        }
    }

    pub fn is_ordination(self) -> bool {
        matches!(self, Occasion::OrdinationDeacon | Occasion::OrdinationPriest | Occasion::OrdinationBishop)
    }

    /// A pastoral office (Marriage, Burial): the Holy Communion is optional.
    pub fn is_pastoral(self) -> bool {
        matches!(self, Occasion::Marriage | Occasion::Burial)
    }

    /// The title for a new service (None: the day's name).
    pub fn title(self) -> Option<&'static str> {
        match self {
            Occasion::Event => Some("Event"),
            Occasion::Eucharist | Occasion::PalmSunday => None,
            _ => Some(self.label()),
        }
    }
}

/// The occasion a date usually calls for: the Holy Week days and Ash
/// Wednesday by their date, else the Eucharist. (The Easter Vigil falls on
/// Holy Saturday night and ordinations on any day, so they are chosen.)
pub fn suggested(date: Date) -> Occasion {
    let e = easter(date.year);
    match date.days_until(e) {
        46 => Occasion::AshWednesday,
        7 => Occasion::PalmSunday,
        3 => Occasion::MaundyThursday,
        2 => Occasion::GoodFriday,
        1 => Occasion::HolySaturday,
        _ => Occasion::Eucharist,
    }
}

/// Holy Baptism or the Renewal of Baptismal Vows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Baptismal {
    Baptism,
    Renewal,
}

/// The options for a service besides its order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Options {
    #[serde(default)]
    pub occasion: Occasion,
    /// A bishop presides.
    #[serde(default)]
    pub bishop: bool,
    #[serde(default)]
    pub baptismal: Option<Baptismal>,
    /// Confirmation, Reception and Reaffirmation (with a bishop).
    #[serde(default)]
    pub confirmation: bool,
    /// No deacon is serving: a priest says the Deacon's parts.
    #[serde(default)]
    pub no_deacon: bool,
    /// A Marriage or Burial with the Holy Communion.
    #[serde(default)]
    pub communion: bool,
}

fn lib(rite: Rite, name: &str) -> Option<Cue> {
    builtin_doc(&rite_name(rite, name)).map(|doc| Cue::liturgy("", doc, None, CueOverrides::default()))
}

fn occ(name: &str) -> Option<Cue> {
    builtin_doc(&format!("occasions/{name}")).map(|doc| Cue::liturgy("", doc, None, CueOverrides::default()))
}

/// The Eucharist from the Offertory to the end, as in the standard order.
fn table(rite: Rite, postcommunion: Option<Cue>, dismissal: bool) -> Vec<Option<Cue>> {
    let mut v = vec![
        hymn_cue("Offertory Hymn"),
        lib(rite, "sursum-corda"),
        lib(rite, "sanctus").map(crate::services::said),
        lib(rite, "lords-prayer"),
        lib(rite, "fraction-anthem"),
        (rite == Rite::One).then(|| lib(rite, "humble-access")).flatten(),
        lib(rite, "invitation"),
        hymn_cue("Communion Hymn"),
        postcommunion.or_else(|| lib(rite, "postcommunion-eternal-god")),
        hymn_cue("Closing Hymn"),
    ];
    if dismissal {
        v.push(lib(rite, "dismissal-go-forth"));
    }
    v
}

/// The order of a service that is not the Sunday Eucharist (None for the
/// Eucharist and Maundy Thursday, which use the standard or saved order).
/// Propers are added afterwards before the sermon.
pub fn order(o: Options, rite: Rite, prayers: Option<crate::services::PrayersForm>) -> Option<Vec<Cue>> {
    let pop = || lib(rite, &crate::services::prayers_item(rite, prayers));
    let sermon = Some(sermon_cue());
    let mut v: Vec<Option<Cue>> = match o.occasion {
        Occasion::Eucharist | Occasion::MaundyThursday => return None,
        // A blank item of slides to make (nothing to show yet).
        Occasion::Event => {
            let mut doc = crate::made::slides_document(Some("Slides".into()), Some(Default::default()));
            doc.blocks.clear();
            return Some(vec![crate::made::slides_cue(doc)]);
        }
        Occasion::AshWednesday => {
            let mut v = vec![
                hymn_cue("Entrance Hymn"),
                lib(rite, "salutation"),
                sermon,
                occ("ash-wednesday-invitation"),
                occ("ash-wednesday-ashes"),
                Some(psalm_cue("Psalm 51:1-18", rite)),
                occ("litany-of-penitence"),
                lib(rite, "the-peace"),
            ];
            v.extend(table(rite, None, true));
            v
        }
        Occasion::PalmSunday => {
            let mut v = vec![
                occ("liturgy-of-the-palms"),
                hymn_cue("Procession Hymn"),
                lib(rite, "salutation"),
                sermon,
                lib(rite, "nicene-creed"),
                pop(),
                lib(rite, "confession"),
                lib(rite, "absolution"),
                lib(rite, "the-peace"),
            ];
            v.extend(table(rite, None, true));
            v
        }
        Occasion::GoodFriday => vec![
            occ("good-friday-opening"),
            sermon,
            occ("solemn-collects"),
            occ("good-friday-anthems"),
            hymn_cue("Hymn"),
            lib(rite, "confession"),
            lib(rite, "lords-prayer"),
            occ("good-friday-final-prayer"),
        ],
        Occasion::HolySaturday => vec![sermon, occ("in-the-midst-of-life")],
        Occasion::EasterVigil => {
            let mut v = vec![
                occ("vigil-service-of-light"),
                occ("exsultet"),
                occ("vigil-liturgy-of-the-word"),
                // Holy Baptism or the Renewal of Vows goes here (see `apply`).
                occ("vigil-first-eucharist"),
                sermon,
                // "The celebration continues with the Prayers of the People."
                pop(),
                lib(rite, "the-peace"),
            ];
            v.extend(table(rite, None, true));
            v
        }
        Occasion::OrdinationDeacon | Occasion::OrdinationPriest | Occasion::OrdinationBishop => {
            let who = match o.occasion {
                Occasion::OrdinationDeacon => "deacon",
                Occasion::OrdinationPriest => "priest",
                _ => "bishop",
            };
            let part = |p: &str| occ(&format!("ordination-{who}-{p}"));
            let mut v = vec![
                hymn_cue("Entrance Hymn"),
                part("opening"),
                part("presentation"),
                occ("litany-for-ordinations"),
                occ("ordination-collect"),
                sermon,
                // At a bishop's ordination the Creed is in the Examination.
                (o.occasion != Occasion::OrdinationBishop).then(|| lib(rite, "nicene-creed")).flatten(),
                part("examination"),
                part("consecration"),
            ];
            let mut t = table(rite, part("after-communion"), false);
            // The new minister's blessing and dismissal end the service.
            let closing = t.pop().flatten();
            v.extend(t);
            v.push(closing);
            v
        }
        Occasion::Marriage => {
            let mut v = vec![
                hymn_cue("Entrance Hymn"),
                occ("marriage-opening"),
                occ("marriage-consent"),
                occ("marriage-collect"),
                // The readings go here.
                sermon,
                occ("marriage-vows"),
                occ("marriage-prayers"),
                // The Blessing of the Marriage ends with the Peace.
                occ("marriage-blessing"),
            ];
            match o.communion {
                true => v.extend(table(rite, occ("marriage-postcommunion"), true)),
                // Without Communion the wedding party leaves after the Peace.
                false => v.push(hymn_cue("Closing Hymn")),
            }
            v
        }
        Occasion::Burial => {
            let n = if rite == Rite::One { 1 } else { 2 };
            let part = |p: &str| occ(&format!("burial-{n}-{p}"));
            let mut v = vec![
                part("anthems"),
                part("collects"),
                // The readings go here.
                sermon,
                part("creed"),
                // "If there is not to be a Communion, the Lord's Prayer is
                // said here" (BCP 480, 495).
                (!o.communion).then(|| lib(rite, "lords-prayer")).flatten(),
                part("prayers"),
            ];
            if o.communion {
                v.push(lib(rite, "the-peace"));
                let mut t = table(rite, part("postcommunion"), false);
                // The Commendation follows the postcommunion prayer.
                t.pop();
                v.extend(t);
            }
            v.extend([part("commendation"), part("procession"), part("committal")]);
            v
        }
    };
    v.retain(Option::is_some);
    Some(v.into_iter().flatten().collect())
}

fn psalm_cue(citation: &str, rite: Rite) -> Cue {
    let item = ProperItem {
        kind: ProperKind::Psalm,
        citation: citation.into(),
        text: String::new(),
        variant: None,
        alternative: false,
        verses: bcp::psalm_selection(citation, true).unwrap_or_default(),
        antiphon: None,
        antiphon_every: 0,
    };
    Cue::liturgy("", propers::build_document_in(&item, None, rite), None, CueOverrides::default())
}

fn reading(kind: ProperKind, citation: &str) -> ProperItem {
    ProperItem {
        kind,
        citation: citation.into(),
        text: String::new(),
        variant: None,
        alternative: false,
        verses: if kind == ProperKind::Psalm {
            bcp::psalm_selection(citation, true).unwrap_or_default()
        } else {
            Vec::new()
        },
        antiphon: None,
        antiphon_every: 0,
    }
}

/// Readings the lectionary does not give for an occasion: the
/// ordinations' (the first of the Prayer Book's choices, BCP 515, 528,
/// 540), Holy Saturday's (BCP 283) and the Vigil's Epistle and Gospel.
/// None: use the lectionary.
pub fn fixed_propers(o: Occasion, date: Date, rite: Rite) -> Option<Vec<Cue>> {
    use ProperKind::*;
    let collect = |name: &str| {
        bcp::find_collect(name).and_then(|c| c.texts(rite).first().cloned()).map(|text| ProperItem {
            kind: Collect,
            citation: String::new(),
            text,
            variant: None,
            alternative: false,
            verses: Vec::new(),
            antiphon: None,
            antiphon_every: 0,
        })
    };
    let year = liturgical_day(date).year;
    if o == Occasion::Event {
        return Some(Vec::new());
    }
    let items: Vec<ProperItem> = match o {
        Occasion::OrdinationDeacon => vec![
            reading(FirstReading, "Jeremiah 1:4-9"),
            reading(Psalm, "Psalm 84"),
            reading(SecondReading, "2 Corinthians 4:1-6"),
            reading(Gospel, "Luke 12:35-38"),
        ],
        Occasion::OrdinationPriest => vec![
            reading(FirstReading, "Isaiah 6:1-8"),
            reading(Psalm, "Psalm 43"),
            reading(SecondReading, "1 Peter 5:1-4"),
            reading(Gospel, "Matthew 9:35-38"),
        ],
        // Marriage (BCP 426) and Burial (BCP 470, 494): one of the
        // Prayer Book's choices for each; the readings are the family's choice.
        Occasion::Marriage => vec![
            reading(FirstReading, "Genesis 1:26-28"),
            reading(Psalm, "Psalm 128"),
            reading(SecondReading, "1 Corinthians 13:1-13"),
            reading(Gospel, "John 15:9-12"),
        ],
        Occasion::Burial => vec![
            reading(FirstReading, "Isaiah 25:6-9"),
            reading(Psalm, "Psalm 23"),
            reading(SecondReading, "Revelation 21:2-7"),
            reading(Gospel, "John 14:1-6"),
        ],
        Occasion::OrdinationBishop => vec![
            reading(FirstReading, "Isaiah 61:1-8"),
            reading(Psalm, "Psalm 99"),
            reading(SecondReading, "Hebrews 5:1-10"),
            reading(Gospel, "John 20:19-23"),
        ],
        Occasion::HolySaturday => collect("Holy Saturday")
            .into_iter()
            .chain([
                reading(FirstReading, "Job 14:1-14"),
                reading(Psalm, "Psalm 130"),
                reading(SecondReading, "1 Peter 4:1-8"),
                reading(Gospel, "Matthew 27:57-66"),
            ])
            .collect(),
        Occasion::EasterVigil => vec![
            reading(SecondReading, "Romans 6:3-11"),
            reading(Psalm, "Psalm 114"),
            reading(
                Gospel,
                match year {
                    'B' => "Mark 16:1-8",
                    'C' => "Luke 24:1-12",
                    _ => "Matthew 28:1-10",
                },
            ),
        ],
        _ => return None,
    };
    Some(
        items
            .iter()
            .map(|i| Cue::liturgy("", propers::build_document_in(i, None, rite), None, CueOverrides::default()))
            .collect(),
    )
}

// ---- changing an order ----------------------------------------------------

/// Whether `cue` is the library item `name` (in either rite).
fn is_item(cue: &Cue, name: &str) -> bool {
    let Some(doc) = cue.document() else { return false };
    [Rite::Two, Rite::One].iter().any(|&r| {
        builtin_doc(&rite_name(r, name))
            .is_some_and(|d| d.meta.title == doc.meta.title && d.meta.citation == doc.meta.citation)
    })
}

fn position(s: &Service, names: &[&str]) -> Option<usize> {
    s.cues.iter().position(|c| names.iter().any(|n| is_item(c, n)))
}

fn remove(s: &mut Service, names: &[&str]) {
    s.cues.retain(|c| !names.iter().any(|n| is_item(c, n)));
}

fn insert_all(s: &mut Service, at: usize, cues: Vec<Option<Cue>>) {
    for (k, cue) in cues.into_iter().flatten().enumerate() {
        s.insert_cue(at + k, cue);
    }
}

/// Where the Creed would be: the Creed, else after the sermon.
fn creed_slot(s: &mut Service) -> usize {
    if let Some(i) = position(s, &["nicene-creed"]) {
        s.cues.remove(i);
        return i;
    }
    s.cues.iter().position(|c| c.category == ElementCategory::Homily).map(|i| i + 1).unwrap_or(s.cues.len())
}

const ACCLAMATIONS: [&str; 3] = ["acclamation-ordinary", "acclamation-easter", "acclamation-lent"];
const DISMISSALS: [&str; 4] =
    ["dismissal-go-forth", "dismissal-go-in-peace", "dismissal-rejoicing", "dismissal-bless-the-lord"];

/// Applies the bishop, baptism, renewal and confirmation options to a
/// service's order (after its propers are in place).
pub fn apply(s: &mut Service, o: Options) {
    if o.occasion == Occasion::Event {
        return;
    }
    let bishop = o.bishop || o.confirmation || o.occasion.is_ordination();
    s.bishop = bishop;
    s.no_deacon = o.no_deacon;
    // The cover names the liturgy (Palm Sunday and Maundy Thursday are
    // the Eucharist of the day).
    if !matches!(o.occasion, Occasion::Eucharist | Occasion::PalmSunday | Occasion::MaundyThursday) {
        s.liturgy = Some(o.occasion.label().to_string());
    }
    let vigil = o.occasion == Occasion::EasterVigil;
    // The Proper Preface these services appoint (else the date's).
    s.preface = if vigil {
        Some("Easter".into())
    } else if o.occasion.is_ordination() {
        Some("Apostles and Ordinations".into())
    } else if o.occasion == Occasion::Marriage {
        Some("Marriage".into())
    } else if o.occasion == Occasion::Burial {
        Some("Commemoration of the Dead".into())
    } else if o.baptismal == Some(Baptismal::Baptism) || o.confirmation {
        Some("Baptism".into())
    } else {
        s.preface.take()
    };

    // The Passion Gospel is announced in its own way (Palm Sunday, Good Friday).
    let passion = match o.occasion {
        Occasion::PalmSunday => occ("passion-announcement"),
        Occasion::GoodFriday => occ("good-friday-passion"),
        _ => None,
    };
    if let Some(p) = passion {
        let at = s.cues.iter().position(|c| c.category == ElementCategory::Gospel).unwrap_or(s.cues.len());
        s.insert_cue(at, p);
    }
    // Good Friday's opening has the Collect of the Day already.
    if o.occasion == Occasion::GoodFriday {
        s.cues.retain(|c| !c.title.eq_ignore_ascii_case("Collect of the Day"));
    }
    if o.occasion == Occasion::MaundyThursday {
        let at =
            s.cues.iter().position(|c| c.category == ElementCategory::Homily).map(|i| i + 1).unwrap_or(s.cues.len());
        if let Some(f) = occ("foot-washing") {
            s.insert_cue(at, f);
        }
    }

    // At the Vigil, Baptism or the Renewal of Vows follows the Liturgy of the Word.
    let vigil_slot = |s: &Service| {
        s.cues
            .iter()
            .position(|c| c.document().is_some_and(|d| d.meta.title == "The Vigil: Liturgy of the Word"))
            .map(|i| i + 1)
    };

    // Already there (the options applied to an existing service twice).
    let has = |s: &Service, n: &str| position(s, &[&format!("occasions/{n}")]).is_some();
    let baptism_done = has(s, "baptism-presentation");
    let confirmation_done = has(s, "confirmation") || has(s, "baptism-confirmation");
    let renewal_done = has(s, "renewal-of-vows");
    let pastoral = o.occasion.is_pastoral();
    let o = Options {
        baptismal: match o.baptismal {
            _ if pastoral => None,
            Some(Baptismal::Baptism) if baptism_done => None,
            Some(Baptismal::Renewal) if renewal_done || baptism_done => None,
            b => b,
        },
        confirmation: o.confirmation && !confirmation_done && !pastoral,
        ..o
    };
    match (o.baptismal, o.confirmation) {
        (Some(Baptismal::Baptism), _) => {
            if !vigil {
                if let Some(i) = position(s, &ACCLAMATIONS) {
                    s.cues.remove(i);
                    insert_all(s, i, vec![occ("baptism-acclamation")]);
                }
                // The baptismal acclamation ends with the salutation.
                remove(s, &["collect-for-purity", "salutation"]);
            }
            let at = match vigil {
                true => vigil_slot(s).unwrap_or(s.cues.len()),
                false => creed_slot(s),
            };
            insert_all(
                s,
                at,
                vec![
                    occ("baptism-presentation"),
                    occ("baptismal-covenant"),
                    occ("baptism-prayers"),
                    occ("baptism-water"),
                    bishop.then(|| occ("baptism-chrism")).flatten(),
                    occ("baptism-the-baptism"),
                    o.confirmation.then(|| occ("baptism-confirmation")).flatten(),
                ],
            );
            // The Prayers for the Candidates take the place of the Prayers of
            // the People; the Baptism ends with the Peace; there is no Confession.
            remove(s, &crate::services::PRAYERS_FORMS);
            remove(s, &["confession", "absolution", "the-peace"]);
        }
        (_, true) => {
            let at = match vigil {
                true => vigil_slot(s).unwrap_or(s.cues.len()),
                false => creed_slot(s),
            };
            insert_all(
                s,
                at,
                vec![
                    occ("confirmation-presentation"),
                    occ("baptismal-covenant"),
                    occ("confirmation-prayers"),
                    occ("confirmation"),
                ],
            );
            remove(s, &crate::services::PRAYERS_FORMS);
            remove(s, &["confession", "absolution", "the-peace"]);
        }
        (Some(Baptismal::Renewal), false) => {
            let at = match vigil {
                true => vigil_slot(s).unwrap_or(s.cues.len()),
                false => creed_slot(s),
            };
            insert_all(s, at, vec![occ("renewal-of-vows")]);
        }
        (None, false) => {
            // The Vigil renews the vows when there is no Baptism (BCP 292).
            if vigil && !baptism_done && !renewal_done {
                if let Some(at) = vigil_slot(s) {
                    insert_all(s, at, vec![occ("renewal-of-vows")]);
                }
            }
        }
    }

    // The bishop blesses the people (BCP 523), before the dismissal.
    // (At a Marriage or a Burial the bishop may bless the people where the
    // rubrics allow; the Bishop's Blessing is in the Library.)
    if bishop
        && !o.occasion.is_ordination()
        && !o.occasion.is_pastoral()
        && !matches!(o.occasion, Occasion::GoodFriday | Occasion::HolySaturday)
    {
        add_bishops_blessing(s);
    }
}

/// Adds Holy Baptism, the Renewal of Vows or Confirmation to the open
/// service (in place of the Creed, as at creation). Confirmation needs a
/// bishop, so it also sets "a bishop presides".
#[tauri::command]
pub fn service_add_baptismal(
    engine: tauri::State<'_, crate::engine::Engine>,
    baptismal: Option<Baptismal>,
    confirmation: bool,
) -> Result<(), String> {
    engine.edit_service(|s| {
        let bishop = s.bishop || confirmation;
        apply(
            s,
            Options {
                occasion: Occasion::Eucharist,
                bishop,
                baptismal,
                confirmation,
                no_deacon: s.no_deacon,
                communion: false,
            },
        );
        s.modified = true;
        s.rebuild_all();
        Ok(())
    })
}

/// Adds the Bishop's Blessing before the dismissal (once). Returns whether
/// it was added.
pub fn add_bishops_blessing(s: &mut Service) -> bool {
    if position(s, &["occasions/episcopal-blessing"]).is_some() {
        return false;
    }
    let at = position(s, &DISMISSALS).unwrap_or_else(|| {
        s.cues
            .iter()
            .rposition(|c| !matches!(c.content, episco_core::CueContent::Announcements { .. }))
            .map(|i| i + 1)
            .unwrap_or(s.cues.len())
    });
    insert_all(s, at, vec![occ("episcopal-blessing")]);
    true
}

#[derive(Serialize)]
pub struct OccasionInfo {
    key: Occasion,
    label: &'static str,
}

#[derive(Serialize)]
pub struct OccasionList {
    occasions: Vec<OccasionInfo>,
    /// The one the date usually calls for.
    suggested: Occasion,
}

/// The kinds of service, and the one `date` usually calls for.
#[tauri::command]
pub fn occasions_list(date: Option<String>) -> OccasionList {
    let suggested = date.as_deref().and_then(|d| d.parse::<Date>().ok()).map(suggested).unwrap_or_default();
    OccasionList {
        occasions: Occasion::ALL.iter().map(|&o| OccasionInfo { key: o, label: o.label() }).collect(),
        suggested,
    }
}

/// Turns "a bishop presides" on or off for the open service: the
/// Celebrant's and Priest's parts are labelled Bishop, and turning it on
/// adds the Bishop's Blessing before the dismissal.
#[tauri::command]
pub fn set_service_bishop(engine: tauri::State<'_, crate::engine::Engine>, bishop: bool) -> Result<bool, String> {
    engine.edit_service(|s| {
        s.bishop = bishop;
        let added = bishop && add_bishops_blessing(s);
        s.modified = true;
        s.rebuild_all();
        Ok(added)
    })
}

/// Says whether a deacon is serving at the open service: with none, the
/// Deacon's parts are labelled Priest on the slides, bulletin and script.
#[tauri::command]
pub fn set_service_deacon(engine: tauri::State<'_, crate::engine::Engine>, deacon: bool) -> Result<(), String> {
    engine.edit_service(|s| {
        s.no_deacon = !deacon;
        s.modified = true;
        s.rebuild_all();
        Ok(())
    })
}

/// The pre- and post-service loop of announcements, for orders built here.
pub fn with_loop(mut cues: Vec<Cue>) -> Vec<Cue> {
    cues.push(Cue::announcements("", AnnouncementMode::Loop, CueOverrides::default()));
    cues
}
