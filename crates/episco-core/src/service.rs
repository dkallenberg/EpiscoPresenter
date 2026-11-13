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
//! other nodes. [`Service::to_plan_toml`] writes it back.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::category::ElementCategory;
use crate::document::{Block, LiturgyDocument, Slide};
use crate::ewm;
use crate::look::{default_look, CueLooks, Look, Output, ResolvedLook, DEFAULT_KEY_COLOR};
use crate::season::{LiturgicalColor, Season};
use crate::state::CuePosition;

/// Conventional file extension for service plans.
pub const PLAN_EXTENSION: &str = "toml";

/// Front-matter key: show a reading's scripture text on the slides too
/// (`slide_text: true`); by default only its title and responses.
pub const SLIDE_TEXT: &str = "slide_text";

/// A fully resolved service, ready to present.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Service {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// A bishop presides (a visitation, an ordination): the Celebrant's and
    /// Priest's parts are labelled Bishop on the slides, bulletin and script.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bishop: bool,
    /// No deacon is serving: the Deacon's parts are labelled Priest.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_deacon: bool,
    /// The liturgy's own name for the bulletin cover ("Good Friday", "The
    /// Ordination of a Priest"); `None` for the Holy Eucharist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub liturgy: Option<String>,
    /// Start time, `"08:00"` (24-hour), for the bulletin cover.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<Season>,
    /// Service-level colour (explicit, else the season's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<LiturgicalColor>,
    /// Published Google Slides URL used for announcement loops.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announcements_deck: Option<String>,
    /// Number of slides in the announcements deck (lets a single pass be
    /// stepped through slide by slide).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck_slides: Option<u32>,
    /// Colour the stream's lower thirds are keyed on (default blue).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_key: Option<String>,
    /// Rite I (traditional) or Rite II (contemporary); `None` is Rite II.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rite: Option<crate::bcp::Rite>,
    /// Season after Pentecost: lectionary Track 1 or 2; `None` is Track 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<u8>,
    /// Translation for reading texts; `None` is the NRSVue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bible: Option<crate::bible::BibleVersion>,
    /// Default looks per item type (`[looks.<type>]` in the plan).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub looks: BTreeMap<ElementCategory, CueLooks>,
    /// The operator's slide layouts (app-wide; filled in by the app when a
    /// service is loaded, never written to the plan).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub templates: crate::template::Templates,
    /// The plan file this service was loaded from / last saved to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_path: Option<PathBuf>,
    /// The parish library folder on this computer, for `library:…` paths
    /// (runtime only, not saved in the plan).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_root: Option<PathBuf>,
    /// Cue order or service details changed since the plan was loaded / saved.
    #[serde(default)]
    pub modified: bool,
    pub cues: Vec<Cue>,
}

/// One entry in the service timeline.
///
/// Everything except `id`, `overrides`, `modified` and `content` is derived
/// by [`Cue::rebuild`]; edit the document or overrides, then rebuild.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cue {
    /// Stable identifier, unique within the service. Survives reordering.
    pub id: String,
    /// Timeline label: the plan's `title` override, else the document title.
    pub title: String,
    pub category: ElementCategory,
    /// Effective colour (document, else service, else none). Built-in
    /// backgrounds are drawn in this colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<LiturgicalColor>,
    /// Lower-third badge text, e.g. `Hymn 436: Lift Up Your Heads`.
    pub caption: String,
    /// Effective look on the sanctuary screen.
    pub sanctuary: ResolvedLook,
    /// Effective look on the live stream.
    pub stream: ResolvedLook,
    /// Values set on the cue in the plan, which win over the document.
    #[serde(default)]
    pub overrides: CueOverrides,
    /// The document changed since it was loaded / saved.
    #[serde(default)]
    pub modified: bool,
    pub content: CueContent,
}

/// Plan-level settings for one cue (`title`, `type`, `[cue.sanctuary]`,
/// `[cue.stream]` in TOML).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CueOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<ElementCategory>,
    #[serde(default, skip_serializing_if = "CueLooks::is_empty")]
    pub looks: CueLooks,
    /// How the item appears in the bulletin; `None` for its default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bulletin: Option<crate::bulletin::BulletinStyle>,
}

/// What a cue needs from its service to resolve its derived fields.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildContext<'a> {
    pub color: Option<LiturgicalColor>,
    pub looks: Option<&'a BTreeMap<ElementCategory, CueLooks>>,
    /// Slides in the announcements deck, if the operator gave the count.
    pub deck_slides: Option<u32>,
    /// A bishop presides: Celebrant and Priest parts are shown as Bishop.
    pub bishop: bool,
    /// No deacon: Deacon parts are shown as Priest.
    pub no_deacon: bool,
    /// The slide layouts (how much text each slide type holds).
    pub templates: Option<&'a crate::template::Templates>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CueContent {
    /// Text / music slides from an `.ewm` file.
    Liturgy {
        /// Absolute path of the `.ewm` file; `None` for an unsaved item.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<PathBuf>,
        document: Box<LiturgyDocument>,
        /// The steps through the item, each with the sanctuary's slide.
        slides: Vec<Slide>,
        /// The live stream's slides, when they differ (see [`crate::fit`]).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        stream_slides: Vec<Slide>,
        /// The stream's slide at each step.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        stream_at: Vec<usize>,
    },
    /// The Google Slides announcements deck.
    Announcements {
        mode: AnnouncementMode,
        /// Positions to step through: the deck's slide count for a single
        /// pass when the service knows it, else 1 (derived).
        #[serde(default = "one")]
        slides: usize,
    },
}

fn one() -> usize {
    1
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
    /// A cue showing an `.ewm` document.
    ///
    /// Derived fields are computed without service context; inserting the
    /// cue into a [`Service`] recomputes them with the service's colour and
    /// default looks.
    pub fn liturgy(
        id: impl Into<String>,
        document: LiturgyDocument,
        source: Option<PathBuf>,
        overrides: CueOverrides,
    ) -> Cue {
        Cue::assemble(
            id.into(),
            overrides,
            CueContent::Liturgy {
                source,
                document: Box::new(document),
                slides: Vec::new(),
                stream_slides: Vec::new(),
                stream_at: Vec::new(),
            },
        )
    }

    /// A cue that runs the announcements deck.
    pub fn announcements(id: impl Into<String>, mode: AnnouncementMode, overrides: CueOverrides) -> Cue {
        Cue::assemble(id.into(), overrides, CueContent::Announcements { mode, slides: 1 })
    }

    fn assemble(id: String, overrides: CueOverrides, content: CueContent) -> Cue {
        let placeholder = ResolvedLook::complete(&Look::default());
        let mut cue = Cue {
            id,
            title: String::new(),
            category: ElementCategory::Other,
            color: None,
            caption: String::new(),
            sanctuary: placeholder.clone(),
            stream: placeholder,
            overrides,
            modified: false,
            content,
        };
        cue.rebuild(&BuildContext::default());
        cue
    }

    /// Recomputes every derived field from the content, the overrides and
    /// the service context.
    pub fn rebuild(&mut self, ctx: &BuildContext) {
        let o = &self.overrides;
        let mut file_background = None;
        match &mut self.content {
            CueContent::Liturgy { document, .. } => {
                let meta = &document.meta;
                self.category = o.category.unwrap_or_else(|| meta.effective_category());
                self.color = meta.effective_color().or(ctx.color);
                self.title = o.title.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| meta.title.clone());
                file_background = meta.default_background.clone();
                // The caption names the item itself (the hymn's real title),
                // not the timeline label ("Processional Hymn").
                let mut caption_meta = meta.clone();
                if caption_meta.title.is_empty() {
                    caption_meta.title = self.title.clone();
                }
                caption_meta.category = Some(self.category);
                self.caption = caption_meta.lower_third_caption();
            }
            CueContent::Announcements { mode, slides } => {
                // A single pass steps through the deck when its length is known.
                *slides = match (mode, ctx.deck_slides) {
                    (AnnouncementMode::SinglePass, Some(n)) if n > 0 => n as usize,
                    _ => 1,
                };
                self.category = o.category.unwrap_or(ElementCategory::Announcements);
                self.color = ctx.color;
                self.title = o.title.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| "Announcements".into());
                self.caption = self.title.clone();
            }
        }
        if self.title.is_empty() {
            self.title = "Untitled".into();
        }

        // Looks: built-in default < service default < file < cue override.
        let service_default = ctx.looks.and_then(|m| m.get(&self.category));
        for output in Output::ALL {
            let mut look = default_look(self.category, output);
            if let Some(d) = service_default {
                look = look.layered(d.get(output));
            }
            if output == Output::Sanctuary {
                if let Some(bg) = &file_background {
                    look = look.layered(&Look { background: Some(bg.clone()), ..Default::default() });
                }
            }
            look = look.layered(self.overrides.looks.get(output));
            let resolved = ResolvedLook::complete(&look);
            match output {
                Output::Sanctuary => self.sanctuary = resolved,
                Output::Stream => self.stream = resolved,
            }
        }
        self.fit_slides(ctx);
    }

    /// The slides of both screens, fitted to their slide types.
    fn fit_slides(&mut self, ctx: &BuildContext) {
        let category = self.category;
        let sanctuary_key = crate::fit::slide_type(Output::Sanctuary, self.sanctuary.layout, category);
        let stream_key = crate::fit::slide_type(Output::Stream, self.stream.layout, category);
        let CueContent::Liturgy { document, slides, stream_slides, stream_at, .. } = &mut self.content else {
            return;
        };
        // A reading's slides show its title and responses; the scripture
        // itself only when the item asks (`slide_text: true`). The bulletin
        // and script print it in full either way.
        let reading = matches!(category, ElementCategory::Reading | ElementCategory::Gospel)
            && document.meta.extra(SLIDE_TEXT) != Some("true");
        let keep = |b: &Block| !(reading && matches!(b, Block::Paragraph { role: None, .. }));
        let none = crate::template::Templates::new();
        let templates = ctx.templates.unwrap_or(&none);
        // Made slides (and imported pages) stay as made on both screens.
        let made = document.meta.is_made_slides();
        let fitted_for =
            |key| if made { crate::fit::Capacity::AS_WRITTEN } else { crate::fit::capacity(key, templates) };
        let sanctuary = fitted_for(sanctuary_key);
        let stream = fitted_for(stream_key);
        let fitted = crate::fit::fit(document, sanctuary, stream, &keep);
        *slides = fitted.steps;
        *stream_slides = fitted.stream;
        *stream_at = fitted.stream_at;
        if ctx.bishop || ctx.no_deacon {
            for slide in slides.iter_mut().chain(stream_slides.iter_mut()) {
                for block in &mut slide.blocks {
                    if let Block::Paragraph { role: Some(r), .. } = block {
                        *r = r.at_service(ctx.bishop, ctx.no_deacon);
                    }
                }
            }
        }
    }

    /// Number of navigable positions in this cue. Always at least 1 so that an
    /// empty or non-slide cue (announcements) can still be selected.
    pub fn slide_count(&self) -> usize {
        match &self.content {
            CueContent::Liturgy { slides, .. } => slides.len().max(1),
            CueContent::Announcements { slides, .. } => (*slides).max(1),
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

    pub fn source(&self) -> Option<&Path> {
        match &self.content {
            CueContent::Liturgy { source, .. } => source.as_deref(),
            CueContent::Announcements { .. } => None,
        }
    }
}

impl Service {
    /// Slide counts per cue — the shape the navigation state machine needs.
    pub fn outline(&self) -> Vec<usize> {
        self.cues.iter().map(Cue::slide_count).collect()
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.cues.iter().position(|c| c.id == id)
    }

    pub fn cue(&self, id: &str) -> Option<&Cue> {
        self.cues.iter().find(|c| c.id == id)
    }

    /// The context cues are resolved in.
    pub fn context(&self) -> BuildContext<'_> {
        BuildContext {
            color: self.color,
            looks: Some(&self.looks),
            deck_slides: self.deck_slides,
            bishop: self.bishop,
            no_deacon: self.no_deacon,
            templates: Some(&self.templates),
        }
    }

    /// Recomputes every cue's derived fields (after service-wide changes).
    pub fn rebuild_all(&mut self) {
        let ctx = BuildContext {
            color: self.color,
            looks: Some(&self.looks),
            deck_slides: self.deck_slides,
            bishop: self.bishop,
            no_deacon: self.no_deacon,
            templates: Some(&self.templates),
        };
        for cue in &mut self.cues {
            cue.rebuild(&ctx);
        }
    }

    /// The colour lower thirds are keyed on.
    pub fn key_color(&self) -> &str {
        self.stream_key.as_deref().unwrap_or(DEFAULT_KEY_COLOR)
    }

    /// Sets (or with an empty look, clears) the service default look for
    /// every item of `category` on `output`.
    pub fn set_default_look(&mut self, category: ElementCategory, output: Output, look: Look) {
        let entry = self.looks.entry(category).or_default();
        *entry.get_mut(output) = look;
        if entry.is_empty() {
            self.looks.remove(&category);
        }
        self.modified = true;
        self.rebuild_all();
    }

    /// True if anything needs saving.
    pub fn has_unsaved_changes(&self) -> bool {
        self.modified || self.cues.iter().any(|c| c.modified)
    }

    /// An id derived from `stem` that no cue uses yet.
    pub fn unique_id(&self, stem: &str) -> String {
        let base = match slug(stem) {
            s if s.is_empty() => "item".to_string(),
            s => s,
        };
        if self.index_of(&base).is_none() {
            return base;
        }
        (2..).map(|n| format!("{base}-{n}")).find(|id| self.index_of(id).is_none()).expect("unbounded")
    }

    /// Wraps a single document as a one-cue service (quick preview / editing).
    pub fn from_document(document: LiturgyDocument, source: Option<PathBuf>) -> Service {
        let mut service = Service {
            title: document.meta.title.clone(),
            season: document.meta.season,
            color: document.meta.effective_color(),
            ..Default::default()
        };
        let id = service.unique_id(&document.meta.title);
        service.cues.push(Cue::liturgy(id, document, source, CueOverrides::default()));
        service.rebuild_all();
        service
    }

    // ---- editing ---------------------------------------------------------

    /// Replaces a liturgy cue's document and marks the cue modified.
    pub fn set_document(&mut self, id: &str, document: LiturgyDocument) -> Result<(), ServiceError> {
        let i = self.position(id)?;
        let ctx = BuildContext {
            color: self.color,
            looks: None,
            deck_slides: self.deck_slides,
            bishop: self.bishop,
            no_deacon: self.no_deacon,
            templates: None,
        };
        let Service { cues, looks, templates, .. } = self;
        let cue = &mut cues[i];
        match &mut cue.content {
            CueContent::Liturgy { document: d, .. } => **d = document,
            CueContent::Announcements { .. } => {
                return Err(ServiceError::Edit(format!("`{}` is an announcements cue and has no text", cue.title)))
            }
        }
        cue.modified = true;
        cue.rebuild(&BuildContext { looks: Some(looks), templates: Some(templates), ..ctx });
        Ok(())
    }

    /// Sets a cue's plan-level overrides.
    pub fn set_overrides(&mut self, id: &str, overrides: CueOverrides) -> Result<(), ServiceError> {
        let i = self.position(id)?;
        let ctx = BuildContext {
            color: self.color,
            looks: None,
            deck_slides: self.deck_slides,
            bishop: self.bishop,
            no_deacon: self.no_deacon,
            templates: None,
        };
        let Service { cues, looks, templates, .. } = self;
        cues[i].overrides = overrides;
        cues[i].rebuild(&BuildContext { looks: Some(looks), templates: Some(templates), ..ctx });
        self.modified = true;
        Ok(())
    }

    /// Records that a liturgy cue's document is now stored at `path`.
    pub fn mark_cue_saved(&mut self, id: &str, path: PathBuf) -> Result<(), ServiceError> {
        let cue = self.cue_mut(id)?;
        if let CueContent::Liturgy { source, .. } = &mut cue.content {
            if source.as_deref() != Some(path.as_path()) {
                *source = Some(path);
                self.modified = true;
            }
        }
        // Re-borrow: `self.modified` above ended the first borrow.
        self.cue_mut(id)?.modified = false;
        Ok(())
    }

    /// Inserts `cue` at `index` (clamped), giving it a unique id. Returns the id.
    pub fn insert_cue(&mut self, index: usize, mut cue: Cue) -> String {
        if cue.id.is_empty() {
            cue.id = self.unique_id(&cue.title);
        } else if self.index_of(&cue.id).is_some() {
            cue.id = self.unique_id(&cue.id);
        }
        cue.rebuild(&self.context());
        let id = cue.id.clone();
        self.cues.insert(index.min(self.cues.len()), cue);
        self.modified = true;
        id
    }

    pub fn remove_cue(&mut self, id: &str) -> Result<Cue, ServiceError> {
        let i = self.index_of(id).ok_or_else(|| ServiceError::Edit(format!("no cue `{id}`")))?;
        self.modified = true;
        Ok(self.cues.remove(i))
    }

    /// Moves a cue so that it ends up at index `to` (clamped).
    pub fn move_cue(&mut self, id: &str, to: usize) -> Result<(), ServiceError> {
        let from = self.index_of(id).ok_or_else(|| ServiceError::Edit(format!("no cue `{id}`")))?;
        let to = to.min(self.cues.len() - 1);
        if from != to {
            let cue = self.cues.remove(from);
            self.cues.insert(to, cue);
            self.modified = true;
        }
        Ok(())
    }

    /// Where the slide at `pos` in `self` ended up in `after`, following the
    /// cue by id. The slide index is clamped if the cue lost slides.
    pub fn remap_position(&self, pos: CuePosition, after: &Service) -> Option<CuePosition> {
        let id = &self.cues.get(pos.cue)?.id;
        let cue = after.index_of(id)?;
        let slide = pos.slide.min(after.cues[cue].slide_count() - 1);
        Some(CuePosition { cue, slide })
    }

    fn position(&self, id: &str) -> Result<usize, ServiceError> {
        self.index_of(id).ok_or_else(|| ServiceError::Edit(format!("no cue `{id}`")))
    }

    fn cue_mut(&mut self, id: &str) -> Result<&mut Cue, ServiceError> {
        self.cues.iter_mut().find(|c| c.id == id).ok_or_else(|| ServiceError::Edit(format!("no cue `{id}`")))
    }

    // ---- plan files ------------------------------------------------------

    /// Reads a TOML plan and every `.ewm` file it references (paths are
    /// relative to the plan's directory).
    pub fn load_plan(path: impl AsRef<Path>) -> Result<Loaded, ServiceError> {
        let path = path.as_ref();
        let src = std::fs::read_to_string(path)
            .map_err(|e| ServiceError::Io { path: path.display().to_string(), message: e.to_string() })?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        let mut loaded = Self::from_plan_str(&src, base)?;
        loaded.service.plan_path = Some(path.to_path_buf());
        Ok(loaded)
    }

    /// Like [`Service::load_plan`] but from TOML text with an explicit base directory.
    pub fn from_plan_str(toml_src: &str, base_dir: &Path) -> Result<Loaded, ServiceError> {
        Self::from_plan_with(toml_src, |file| {
            let full = base_dir.join(file);
            let src = std::fs::read_to_string(&full).map_err(|e| format!("could not read {}: {e}", full.display()))?;
            Ok((src, Some(full)))
        })
    }

    /// Builds a service from TOML text, fetching each cue's `.ewm` source
    /// through `read(file) -> (text, resolved path)`. Lets callers load plans
    /// from somewhere other than the file system (e.g. embedded examples).
    pub fn from_plan_with(
        toml_src: &str,
        mut read: impl FnMut(&Path) -> Result<(String, Option<PathBuf>), String>,
    ) -> Result<Loaded, ServiceError> {
        let plan: ServicePlan = toml::from_str(toml_src).map_err(|e| ServiceError::Plan(e.message().to_string()))?;
        let mut warnings = Vec::new();

        let season = plan.season.as_deref().map(str::parse::<Season>).transpose().map_err(ServiceError::Plan)?;
        let color = plan
            .color
            .as_deref()
            .map(str::parse::<LiturgicalColor>)
            .transpose()
            .map_err(ServiceError::Plan)?
            .or_else(|| season.map(Season::default_color));

        let mut service = Service {
            title: plan.title,
            date: plan.date,
            time: plan.time,
            bishop: plan.bishop,
            no_deacon: plan.no_deacon,
            liturgy: plan.liturgy,
            season,
            color,
            announcements_deck: plan.announcements_deck,
            stream_key: plan.stream_key,
            deck_slides: plan.deck_slides.filter(|n| *n > 0),
            looks: plan.looks,
            rite: plan.rite.as_deref().and_then(crate::bcp::Rite::parse),
            track: plan.track.filter(|t| matches!(t, 1 | 2)),
            bible: plan.bible.as_deref().and_then(crate::bible::BibleVersion::parse),
            ..Default::default()
        };

        for (i, pc) in plan.cues.into_iter().enumerate() {
            let n = i + 1;
            let cue_err = |message: String| ServiceError::Cue { cue: n, message };
            let mut looks = CueLooks { sanctuary: pc.sanctuary, stream: pc.stream };
            // `background = "…"` (older plans) is the sanctuary background.
            if looks.sanctuary.background.is_none() {
                looks.sanctuary.background = pc.background;
            }
            let overrides = CueOverrides {
                title: pc.title,
                category: pc.category.as_deref().map(str::parse::<ElementCategory>).transpose().map_err(cue_err)?,
                looks,
                bulletin: pc.bulletin.as_deref().and_then(crate::bulletin::BulletinStyle::parse),
            };
            let cue = match (&pc.file, pc.announcements) {
                (Some(_), Some(_)) => return Err(cue_err("a cue may have `file` or `announcements`, not both".into())),
                (None, None) => return Err(cue_err("a cue needs `file` or `announcements`".into())),
                (None, Some(mode)) => Cue::announcements(service.unique_id("announcements"), mode, overrides),
                (Some(file), None) => {
                    let (src, resolved) = read(file).map_err(cue_err)?;
                    let parsed = ewm::parse(&src).map_err(|e| cue_err(format!("{}: {e}", file.display())))?;
                    warnings.extend(
                        parsed.warnings.iter().map(|w| format!("{}:{}: {}", file.display(), w.line, w.message)),
                    );
                    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("item");
                    Cue::liturgy(service.unique_id(stem), parsed.document, resolved, overrides)
                }
            };
            service.cues.push(cue);
        }
        service.rebuild_all();
        Ok(Loaded { service, warnings })
    }

    /// Saves the service to `plan_path`: items that have never been saved get
    /// a file in `liturgies/` beside the plan, every modified item is written,
    /// then the plan itself. Afterwards nothing is marked modified.
    pub fn save_to(&mut self, plan_path: &Path) -> Result<(), ServiceError> {
        // Paths inside the parish library folder are saved relative to it.
        if let Some(root) = self.library_root.clone() {
            self.make_portable(&root);
        }
        let io = |path: &Path, e: std::io::Error| ServiceError::Io {
            path: path.display().to_string(),
            message: e.to_string(),
        };
        let dir = plan_path.parent().unwrap_or_else(|| Path::new(".")).join("liturgies");

        let mut taken: Vec<PathBuf> = self.cues.iter().filter_map(|c| c.source().map(Path::to_path_buf)).collect();
        for cue in &mut self.cues {
            let title = cue.title.clone();
            if let CueContent::Liturgy { source: source @ None, .. } = &mut cue.content {
                std::fs::create_dir_all(&dir).map_err(|e| io(&dir, e))?;
                let stem = match slug(&title) {
                    s if s.is_empty() => "item".to_string(),
                    s => s,
                };
                let mut file = dir.join(format!("{stem}.{}", ewm::EXTENSION));
                let mut n = 2;
                while file.exists() || taken.contains(&file) {
                    file = dir.join(format!("{stem}-{n}.{}", ewm::EXTENSION));
                    n += 1;
                }
                taken.push(file.clone());
                *source = Some(file);
                cue.modified = true;
            }
        }

        let plan = self.to_plan_toml(plan_path)?;
        for cue in self.cues.iter().filter(|c| c.modified) {
            if let (Some(src), Some(doc)) = (cue.source(), cue.document()) {
                std::fs::write(src, ewm::to_string(doc)).map_err(|e| io(src, e))?;
            }
        }
        if let Some(parent) = plan_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
        }
        std::fs::write(plan_path, plan).map_err(|e| io(plan_path, e))?;

        self.plan_path = Some(plan_path.to_path_buf());
        self.modified = false;
        for cue in &mut self.cues {
            cue.modified = false;
        }
        Ok(())
    }

    /// Serialises the service as a TOML plan to be stored at `plan_path`.
    /// Cue files are written relative to the plan's directory where possible.
    ///
    /// Fails if a liturgy cue has never been saved to an `.ewm` file.
    pub fn to_plan_toml(&self, plan_path: &Path) -> Result<String, ServiceError> {
        let base = plan_path.parent().unwrap_or_else(|| Path::new(""));
        let cues = self
            .cues
            .iter()
            .map(|cue| {
                let (file, announcements) = match &cue.content {
                    CueContent::Liturgy { source: Some(src), .. } => {
                        (Some(src.strip_prefix(base).map(Path::to_path_buf).unwrap_or_else(|_| src.clone())), None)
                    }
                    CueContent::Liturgy { source: None, .. } => {
                        return Err(ServiceError::Edit(format!(
                            "save “{}” to an .ewm file before saving the service",
                            cue.title
                        )))
                    }
                    CueContent::Announcements { mode, .. } => (None, Some(*mode)),
                };
                Ok(PlanCue {
                    file,
                    title: cue.overrides.title.clone(),
                    category: cue.overrides.category.map(|c| c.key().to_string()),
                    background: None,
                    announcements,
                    bulletin: cue.overrides.bulletin.map(|b| b.key().to_string()),
                    sanctuary: cue.overrides.looks.sanctuary.clone(),
                    stream: cue.overrides.looks.stream.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let plan = ServicePlan {
            title: self.title.clone(),
            date: self.date.clone(),
            time: self.time.clone(),
            bishop: self.bishop,
            no_deacon: self.no_deacon,
            liturgy: self.liturgy.clone(),
            season: self.season.map(|s| s.label().to_string()),
            // Only write a colour that differs from the season default.
            color: self.color.filter(|c| Some(*c) != self.season.map(Season::default_color)).map(|c| c.label().into()),
            announcements_deck: self.announcements_deck.clone(),
            stream_key: self.stream_key.clone(),
            deck_slides: self.deck_slides,
            rite: self.rite.filter(|r| *r == crate::bcp::Rite::One).map(|_| "I".into()),
            track: self.track.filter(|t| *t == 2),
            bible: self.bible.map(|b| b.initials().into()),
            looks: self.looks.clone(),
            cues,
        };
        toml::to_string(&plan).map_err(|e| ServiceError::Plan(e.to_string()))
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
    #[error("{0}")]
    Edit(String),
}

/// On-disk TOML plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServicePlan {
    #[serde(default)]
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    time: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    bishop: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    no_deacon: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    liturgy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    season: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    announcements_deck: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deck_slides: Option<u32>,
    /// `"I"` or `"II"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rite: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    track: Option<u8>,
    /// `"NRSV"` or `"NRSVue"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bible: Option<String>,
    // Tables must follow plain values in TOML.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    looks: BTreeMap<ElementCategory, CueLooks>,
    #[serde(default, rename = "cue")]
    cues: Vec<PlanCue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanCue {
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    announcements: Option<AnnouncementMode>,
    /// `reference`, `full`, `title` or `hidden`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bulletin: Option<String>,
    #[serde(default, skip_serializing_if = "Look::is_empty")]
    sanctuary: Look,
    #[serde(default, skip_serializing_if = "Look::is_empty")]
    stream: Look,
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readings_show_their_responses_unless_the_text_is_asked_for() {
        let src = "---\ntitle: First Reading\ntype: reading\ncitation: Job 19:23-27a\n---\n# The First Reading\n\n---\n\nO that my words were written down!\n\n---\n\nFor I know that my Redeemer lives. (NRSVue)\n\n---\n\n[Reader] The word of the Lord.\n[People] **Thanks be to God.**\n";
        let doc: LiturgyDocument = src.parse().unwrap();
        let cue = Cue::liturgy("r", doc.clone(), None, CueOverrides::default());
        assert_eq!(cue.slide_count(), 2, "title and responses only");
        assert!(cue.slide(0).unwrap().blocks.is_empty(), "the title slide stays");
        assert!(cue.slide(1).unwrap().blocks.iter().all(|b| b.role().is_some()));
        // Asked for: the text is on the slides too.
        let mut with = doc;
        with.meta.extra.push((SLIDE_TEXT.into(), "true".into()));
        let cue = Cue::liturgy("r", with, None, CueOverrides::default());
        assert_eq!(cue.slide_count(), 4);
        // Not a reading: untouched.
        let prayer: LiturgyDocument =
            "---\ntitle: P\ntype: prayer\n---\nSome words.\n\n---\n\nMore.\n".parse().unwrap();
        assert_eq!(Cue::liturgy("p", prayer, None, CueOverrides::default()).slide_count(), 2);
    }
    use crate::document::Block;
    use crate::look::{Layout, TextTone};
    use crate::role::Role;

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("episco-core-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const PLAN: &str = r#"
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

    fn fixture(tag: &str) -> PathBuf {
        let dir = tempdir(tag);
        write(&dir, "sursum.ewm", "---\ntitle: Sursum Corda\ntype: eucharistic_prayer\n---\n[Priest] Lift up your hearts.\n---\n[People] **We lift them.**\n");
        write(
            &dir,
            "hymn.ewm",
            "---\ntitle: Lift Up Your Heads\ntype: hymn\nhymn_number: 436\ncolor: Red\n---\nVerse\n",
        );
        write(&dir, "service.toml", PLAN);
        dir
    }

    #[test]
    fn loads_plan_with_files_and_announcements() {
        let dir = fixture("plan");
        let loaded = Service::load_plan(dir.join("service.toml")).unwrap();
        assert!(loaded.warnings.is_empty());
        let s = loaded.service;
        assert_eq!(s.plan_path.as_deref(), Some(dir.join("service.toml").as_path()));
        assert!(!s.has_unsaved_changes());
        assert_eq!(s.color, Some(LiturgicalColor::Green));
        assert_eq!(s.outline(), vec![1, 2, 1]);
        assert_eq!(s.cues[0].id, "hymn");
        assert_eq!(s.cues[0].source(), Some(dir.join("hymn.ewm").as_path()));
        assert_eq!(s.cues[0].color, Some(LiturgicalColor::Red));
        assert_eq!(s.cues[0].sanctuary.layout, Layout::Lyrics);
        assert_eq!(s.cues[0].title, "Processional Hymn");
        assert_eq!(s.cues[0].caption, "Hymn 436: Lift Up Your Heads");
        assert_eq!(s.cues[0].stream.layout, Layout::LeftPanelWide);
        // Plan override wins over the file's `type:` and re-derives the zone.
        assert_eq!(s.cues[1].category, ElementCategory::Psalm);
        assert_eq!(s.cues[1].stream.layout, Layout::BottomBar);
        assert_eq!(s.cues[1].color, Some(LiturgicalColor::Green));
        // `background = …` in a cue is the sanctuary background.
        assert_eq!(s.cues[1].sanctuary.background, "custom.png");
        assert_eq!(s.cues[2].id, "announcements");
        assert!(matches!(s.cues[2].content, CueContent::Announcements { mode: AnnouncementMode::SinglePass, .. }));

        // The resolved service is what travels over the network.
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Service>(&json).unwrap(), s);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_keeps_overrides_and_tracks_changes() {
        let dir = fixture("edit");
        let mut s = Service::load_plan(dir.join("service.toml")).unwrap().service;

        let mut doc = s.cues[1].document().unwrap().clone();
        doc.meta.title = "Lift Up Your Hearts".into();
        doc.blocks.push(Block::SlideBreak);
        doc.blocks.push(Block::speech(Role::Priest, ["Let us give thanks."]));
        s.set_document("sursum", doc).unwrap();

        let cue = &s.cues[1];
        assert!(cue.modified && !s.modified);
        assert_eq!(cue.slide_count(), 3);
        assert_eq!(cue.title, "Lift Up Your Hearts");
        assert_eq!(cue.category, ElementCategory::Psalm, "override survives edits");
        assert_eq!(cue.sanctuary.background, "custom.png");
        assert!(s.set_document("announcements", LiturgyDocument::default()).is_err());

        s.mark_cue_saved("sursum", dir.join("sursum.ewm")).unwrap();
        assert!(!s.has_unsaved_changes());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn insert_move_remove_and_remap() {
        let dir = fixture("order");
        let before = Service::load_plan(dir.join("service.toml")).unwrap().service;
        let mut s = before.clone();

        let doc: LiturgyDocument = "---\ntitle: Lift Up Your Heads\n---\nx\n".parse().unwrap();
        let id = s.insert_cue(0, Cue::liturgy("hymn", doc, None, CueOverrides::default()));
        assert_eq!(id, "hymn-2", "ids stay unique");
        assert_eq!(s.cues[1].id, "hymn");
        s.move_cue("announcements", 0).unwrap();
        assert_eq!(
            s.cues.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            ["announcements", "hymn-2", "hymn", "sursum"]
        );
        assert!(s.modified);

        // The on-air slide follows its cue through the reorder.
        let pos = before.remap_position(CuePosition { cue: 1, slide: 1 }, &s);
        assert_eq!(pos, Some(CuePosition { cue: 3, slide: 1 }));
        s.remove_cue("sursum").unwrap();
        assert_eq!(before.remap_position(CuePosition { cue: 1, slide: 1 }, &s), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_round_trips_through_toml() {
        let dir = fixture("save");
        let s = Service::load_plan(dir.join("service.toml")).unwrap().service;
        let toml = s.to_plan_toml(&dir.join("service.toml")).unwrap();
        assert!(toml.contains("file = \"hymn.ewm\""), "{toml}");
        assert!(!toml.contains("color"), "season default colour is implied: {toml}");
        let again = Service::from_plan_str(&toml, &dir).unwrap().service;
        assert_eq!(again.cues, s.cues);

        // An unsaved item blocks saving the plan, with a clear message.
        let mut s2 = s.clone();
        s2.insert_cue(0, Cue::liturgy("", LiturgyDocument::default(), None, CueOverrides::default()));
        let err = s2.to_plan_toml(&dir.join("service.toml")).unwrap_err();
        assert!(err.to_string().contains("Untitled"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn looks_layer_service_defaults_and_cue_overrides() {
        let dir = fixture("looks");
        let plan = r##"
            title = "Eucharist"
            season = "Pentecost"
            stream_key = "#00ff00"
            [looks.psalm.sanctuary]
            background = "backgrounds/prayer.png"
            [looks.hymn.stream]
            background = "backgrounds/hymn-panel.png"
            [[cue]]
            file = "hymn.ewm"
            [[cue]]
            file = "sursum.ewm"
            background = "custom.png"
            type = "psalm"
            [cue.stream]
            tone = "dark"
        "##;
        std::fs::write(dir.join("service.toml"), plan).unwrap();
        let mut s = Service::load_plan(dir.join("service.toml")).unwrap().service;
        assert_eq!(s.key_color(), "#00ff00");

        // Service default for hymns on the stream, layout kept from the built-in.
        assert_eq!(s.cues[0].stream.background, "backgrounds/hymn-panel.png");
        assert_eq!(s.cues[0].stream.layout, Layout::LeftPanelWide);
        // Cue 2 is a psalm (plan override): the service psalm default is
        // overridden by the cue's own `background` on the sanctuary.
        assert_eq!(s.cues[1].sanctuary.background, "custom.png");
        assert_eq!(s.cues[1].stream.tone, TextTone::Dark);

        // Changing a service default re-resolves every cue of that type.
        s.set_default_look(
            ElementCategory::Psalm,
            Output::Stream,
            Look { layout: Some(Layout::LowerThird), ..Default::default() },
        );
        assert_eq!(s.cues[1].stream.layout, Layout::LowerThird);
        assert_eq!(s.cues[1].stream.background, "key");
        assert_eq!(s.cues[1].stream.tone, TextTone::Dark, "cue override still wins");

        // Everything survives a save / load round trip.
        let toml = s.to_plan_toml(&dir.join("service.toml")).unwrap();
        let again = Service::from_plan_str(&toml, &dir).unwrap().service;
        assert_eq!(again.looks, s.looks);
        assert_eq!(again.stream_key, s.stream_key);
        assert_eq!(again.cues, s.cues);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_to_writes_new_items_beside_the_plan() {
        let dir = fixture("saveto");
        let mut s = Service::load_plan(dir.join("service.toml")).unwrap().service;
        let doc: LiturgyDocument =
            "---\ntitle: Collect of the Day\ntype: collect\n---\n[Celebrant] O God.\n".parse().unwrap();
        s.insert_cue(1, Cue::liturgy("", doc.clone(), None, CueOverrides::default()));
        s.insert_cue(2, Cue::liturgy("", doc, None, CueOverrides::default()));
        let mut hymn = s.cues[0].document().unwrap().clone();
        hymn.meta.tune = Some("Truro".into());
        s.set_document("hymn", hymn).unwrap();
        assert!(s.has_unsaved_changes());

        let target = dir.join("copy").join("service.toml");
        s.save_to(&target).unwrap();
        assert!(!s.has_unsaved_changes());
        assert_eq!(s.plan_path.as_deref(), Some(target.as_path()));
        let lit = dir.join("copy").join("liturgies");
        assert!(lit.join("collect-of-the-day.ewm").exists());
        assert!(lit.join("collect-of-the-day-2.ewm").exists(), "names stay unique");

        let back = Service::load_plan(&target).unwrap().service;
        assert_eq!(back.cues.len(), s.cues.len());
        assert_eq!(back.cues[1].title, "Collect of the Day");
        assert!(std::fs::read_to_string(dir.join("hymn.ewm")).unwrap().contains("tune: Truro"));
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
        assert_eq!(slug("  Psalm 78 "), "psalm-78");
    }

    #[test]
    fn a_single_pass_steps_through_the_deck_when_its_length_is_known() {
        let mut s = Service::default();
        s.insert_cue(0, Cue::announcements("pass", AnnouncementMode::SinglePass, CueOverrides::default()));
        s.insert_cue(1, Cue::announcements("loop", AnnouncementMode::Loop, CueOverrides::default()));
        assert_eq!(s.outline(), vec![1, 1], "unknown length: one position (the deck plays itself)");
        s.deck_slides = Some(8);
        s.rebuild_all();
        assert_eq!(s.outline(), vec![8, 1], "single pass: one position per slide; a loop stays one");
        // Kept in the plan.
        let toml = s.to_plan_toml(Path::new("/tmp/x/service.toml")).unwrap();
        assert!(toml.contains("deck_slides = 8"), "{toml}");
        let back = Service::from_plan_str(&toml, Path::new("/tmp/x")).unwrap().service;
        assert_eq!(back.outline(), vec![8, 1]);
    }

    #[test]
    fn blank_looks_layer_and_resolve() {
        let base = default_look(ElementCategory::Prayer, Output::Sanctuary);
        let blank = base.layered(&Look { blank: Some(true), ..Default::default() });
        let r = ResolvedLook::complete(&blank);
        assert!(r.blank);
        assert_eq!(r.layout, ResolvedLook::complete(&base).layout, "blank keeps the rest of the look");
        assert!(!ResolvedLook::complete(&base).blank);
    }
}
