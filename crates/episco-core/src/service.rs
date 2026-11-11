//! A service: the ordered list of cues the operator steps through.
//!
//! On disk a service is a small TOML *plan* that references `.ewm` files:
//!
//! ```toml
//! title = "Holy Eucharist, Rite II"
//! date = "2026-11-15"
//! season = "Pentecost"
//! announcements_deck = "https://docs.google.com/presentation/d/.../pub"
//!
//! [[cue]]
//! file = "liturgies/opening-acclamation.ewm"
//!
//! [[cue]]
//! title = "Announcements"
//! announcements = "single_pass"
//! ```
//!
//! [`Service::load_plan`] resolves every file into a self-contained [`Service`]
//! which is what the operator UI renders and what the master node sends to
//! other nodes.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::category::{ElementCategory, SanctuaryLayout, StreamZone};
use crate::document::{LiturgyDocument, Slide};
use crate::ewm;
use crate::season::{LiturgicalColor, Season};

/// Conventional file extension for service plans.
pub const PLAN_EXTENSION: &str = "toml";

/// A fully resolved service, ready to present.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Service {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<Season>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<LiturgicalColor>,
    /// Published Google Slides URL used for announcement loops.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announcements_deck: Option<String>,
    pub cues: Vec<Cue>,
}

/// One entry in the service timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cue {
    /// Stable identifier, unique within the service.
    pub id: String,
    pub title: String,
    pub category: ElementCategory,
    /// Effective colour (cue file, else service, else season default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<LiturgicalColor>,
    /// Effective background image (cue override, else file, else colour default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// Lower-third badge text, e.g. `Hymn 436: Lift Up Your Heads`.
    pub caption: String,
    /// Derived from `category`; kept in the data so every renderer agrees.
    pub stream_zone: StreamZone,
    pub sanctuary_layout: SanctuaryLayout,
    pub content: CueContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CueContent {
    /// Text / music slides from an `.ewm` file.
    Liturgy {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<PathBuf>,
        document: Box<LiturgyDocument>,
        slides: Vec<Slide>,
    },
    /// The Google Slides announcements deck.
    Announcements { mode: AnnouncementMode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementMode {
    /// 9-second auto-advance loop until the operator moves on.
    Loop,
    /// One pass, advanced manually, with a quick exit to the next cue.
    SinglePass,
}

impl Cue {
    /// Recomputes `caption`, `stream_zone` and `sanctuary_layout` after the
    /// title or category changed.
    pub fn refresh_derived(&mut self) {
        self.stream_zone = self.category.stream_zone();
        self.sanctuary_layout = self.category.sanctuary_layout();
        self.caption = match &self.content {
            CueContent::Liturgy { document, .. } => {
                // The caption names the item itself (the hymn's real title),
                // not the cue's timeline label ("Processional Hymn").
                let mut meta = document.meta.clone();
                if meta.title.is_empty() {
                    meta.title = self.title.clone();
                }
                meta.category = Some(self.category);
                meta.lower_third_caption()
            }
            CueContent::Announcements { .. } => self.title.clone(),
        };
    }

    /// Number of navigable positions in this cue. Always at least 1 so that an
    /// empty or non-slide cue (announcements) can still be selected.
    pub fn slide_count(&self) -> usize {
        match &self.content {
            CueContent::Liturgy { slides, .. } => slides.len().max(1),
            CueContent::Announcements { .. } => 1,
        }
    }

    pub fn slide(&self, index: usize) -> Option<&Slide> {
        match &self.content {
            CueContent::Liturgy { slides, .. } => slides.get(index),
            CueContent::Announcements { .. } => None,
        }
    }

    pub fn document(&self) -> Option<&LiturgyDocument> {
        match &self.content {
            CueContent::Liturgy { document, .. } => Some(document),
            CueContent::Announcements { .. } => None,
        }
    }

    /// Builds a cue from a parsed document, inheriting service-level colour.
    pub fn from_document(
        id: impl Into<String>,
        document: LiturgyDocument,
        source: Option<PathBuf>,
        service_color: Option<LiturgicalColor>,
    ) -> Cue {
        let meta = &document.meta;
        let color = meta.effective_color().or(service_color);
        let background = meta.default_background.clone().or_else(|| color.map(LiturgicalColor::default_background));
        let slides = document.slides();
        let category = meta.effective_category();
        let mut cue = Cue {
            id: id.into(),
            title: meta.title.clone(),
            category,
            color,
            background,
            caption: String::new(),
            stream_zone: category.stream_zone(),
            sanctuary_layout: category.sanctuary_layout(),
            content: CueContent::Liturgy { source, document: Box::new(document), slides },
        };
        cue.refresh_derived();
        cue
    }
}

impl Service {
    /// Slide counts per cue — the shape the navigation state machine needs.
    pub fn outline(&self) -> Vec<usize> {
        self.cues.iter().map(Cue::slide_count).collect()
    }

    /// Wraps a single document as a one-cue service (quick preview / editing).
    pub fn from_document(document: LiturgyDocument, source: Option<PathBuf>) -> Service {
        let season = document.meta.season;
        let color = document.meta.effective_color();
        let title = document.meta.title.clone();
        Service {
            title,
            season,
            color,
            cues: vec![Cue::from_document("c01", document, source, color)],
            ..Default::default()
        }
    }

    /// Reads a TOML plan and every `.ewm` file it references (paths are
    /// relative to the plan's directory).
    pub fn load_plan(path: impl AsRef<Path>) -> Result<Loaded, ServiceError> {
        let path = path.as_ref();
        let src = std::fs::read_to_string(path)
            .map_err(|e| ServiceError::Io { path: path.display().to_string(), message: e.to_string() })?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        Self::from_plan_str(&src, base)
    }

    /// Like [`Service::load_plan`] but from TOML text with an explicit base directory.
    pub fn from_plan_str(toml_src: &str, base_dir: &Path) -> Result<Loaded, ServiceError> {
        let plan: ServicePlan = toml::from_str(toml_src).map_err(|e| ServiceError::Plan(e.message().to_string()))?;
        let mut warnings = Vec::new();

        let season = plan.season.as_deref().map(str::parse::<Season>).transpose().map_err(ServiceError::Plan)?;
        let color = match plan.color.as_deref().map(str::parse::<LiturgicalColor>).transpose() {
            Ok(c) => c.or_else(|| season.map(Season::default_color)),
            Err(e) => return Err(ServiceError::Plan(e)),
        };

        let mut cues = Vec::with_capacity(plan.cues.len());
        for (i, pc) in plan.cues.into_iter().enumerate() {
            let n = i + 1;
            let category_override = pc
                .category
                .as_deref()
                .map(str::parse::<ElementCategory>)
                .transpose()
                .map_err(|e| ServiceError::Cue { cue: n, message: e })?;

            let mut cue = match (&pc.file, pc.announcements) {
                (Some(_), Some(_)) => {
                    return Err(ServiceError::Cue {
                        cue: n,
                        message: "a cue may have `file` or `announcements`, not both".into(),
                    })
                }
                (None, None) => {
                    return Err(ServiceError::Cue { cue: n, message: "a cue needs `file` or `announcements`".into() })
                }
                (None, Some(mode)) => Cue {
                    id: format!("c{n:02}-announcements"),
                    title: "Announcements".into(),
                    category: ElementCategory::Announcements,
                    color,
                    background: None,
                    caption: "Announcements".into(),
                    stream_zone: StreamZone::LowerThird,
                    sanctuary_layout: SanctuaryLayout::SeasonalBackground,
                    content: CueContent::Announcements { mode },
                },
                (Some(file), None) => {
                    let full = base_dir.join(file);
                    let parsed =
                        ewm::load_file(&full).map_err(|e| ServiceError::Cue { cue: n, message: e.to_string() })?;
                    warnings.extend(
                        parsed.warnings.iter().map(|w| format!("{}:{}: {}", file.display(), w.line, w.message)),
                    );
                    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("cue");
                    Cue::from_document(format!("c{n:02}-{}", slug(stem)), parsed.document, Some(file.clone()), color)
                }
            };
            if let Some(t) = pc.title {
                cue.title = t;
            }
            if let Some(c) = category_override {
                cue.category = c;
            }
            if let Some(b) = pc.background {
                cue.background = Some(b);
            }
            if cue.title.is_empty() {
                cue.title = format!("Cue {n}");
            }
            cue.refresh_derived();
            cues.push(cue);
        }

        Ok(Loaded {
            service: Service {
                title: plan.title,
                date: plan.date,
                season,
                color,
                announcements_deck: plan.announcements_deck,
                cues,
            },
            warnings,
        })
    }
}

/// Result of loading a plan: the service plus non-fatal file warnings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Loaded {
    pub service: Service,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ServiceError {
    #[error("could not read {path}: {message}")]
    Io { path: String, message: String },
    #[error("invalid service plan: {0}")]
    Plan(String),
    #[error("cue {cue}: {message}")]
    Cue { cue: usize, message: String },
}

/// On-disk TOML plan.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServicePlan {
    #[serde(default)]
    title: String,
    date: Option<String>,
    season: Option<String>,
    color: Option<String>,
    announcements_deck: Option<String>,
    #[serde(default, rename = "cue")]
    cues: Vec<PlanCue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanCue {
    file: Option<PathBuf>,
    title: Option<String>,
    #[serde(rename = "type")]
    category: Option<String>,
    background: Option<String>,
    announcements: Option<AnnouncementMode>,
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("episco-core-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn loads_plan_with_files_and_announcements() {
        let dir = tempdir("plan");
        write(&dir, "sursum.ewm", "---\ntitle: Sursum Corda\ntype: eucharistic_prayer\n---\n[Priest] Lift up your hearts.\n---\n[People] **We lift them.**\n");
        write(
            &dir,
            "hymn.ewm",
            "---\ntitle: Lift Up Your Heads\ntype: hymn\nhymn_number: 436\ncolor: Red\n---\nVerse\n",
        );
        let plan = r#"
            title = "Eucharist"
            season = "Pentecost"
            [[cue]]
            file = "hymn.ewm"
            title = "Processional Hymn"
            [[cue]]
            file = "sursum.ewm"
            background = "custom.png"
            type = "psalm"
            [[cue]]
            announcements = "single_pass"
        "#;
        let loaded = Service::from_plan_str(plan, &dir).unwrap();
        assert!(loaded.warnings.is_empty());
        let s = loaded.service;
        assert_eq!(s.color, Some(LiturgicalColor::Green));
        assert_eq!(s.outline(), vec![1, 2, 1]);
        assert_eq!(s.cues[0].id, "c01-hymn");
        assert_eq!(s.cues[0].color, Some(LiturgicalColor::Red));
        assert_eq!(s.cues[0].background.as_deref(), Some("red_stained_glass.png"));
        assert_eq!(s.cues[0].title, "Processional Hymn");
        assert_eq!(s.cues[0].caption, "Hymn 436: Lift Up Your Heads");
        assert_eq!(s.cues[0].stream_zone, StreamZone::LeftPanel);
        // Plan override wins over the file's `type:` and re-derives the zone.
        assert_eq!(s.cues[1].category, ElementCategory::Psalm);
        assert_eq!(s.cues[1].stream_zone, StreamZone::BottomBar);
        assert_eq!(s.cues[1].color, Some(LiturgicalColor::Green));
        assert_eq!(s.cues[1].background.as_deref(), Some("custom.png"));
        assert!(matches!(s.cues[2].content, CueContent::Announcements { mode: AnnouncementMode::SinglePass }));

        // The resolved service is what travels over the network.
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Service>(&json).unwrap(), s);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_errors_name_the_cue() {
        let dir = tempdir("err");
        let err = Service::from_plan_str("[[cue]]\nfile = \"missing.ewm\"\n", &dir).unwrap_err();
        assert!(matches!(err, ServiceError::Cue { cue: 1, .. }), "{err}");
        let err = Service::from_plan_str("[[cue]]\ntitle = \"x\"\n", &dir).unwrap_err();
        assert!(matches!(err, ServiceError::Cue { cue: 1, .. }));
        let err = Service::from_plan_str("bogus = 1\n", &dir).unwrap_err();
        assert!(matches!(err, ServiceError::Plan(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("Great Thanksgiving (Rite II)"), "great-thanksgiving-rite-ii");
    }
}
