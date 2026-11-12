//! Core data model for EpiscoPresenter.
//!
//! * [`role`], [`season`], [`category`] — liturgical vocabulary.
//! * [`look`] — backgrounds and text layouts per output, with defaults per
//!   item type.
//! * [`document`] — a parsed liturgy item ([`LiturgyDocument`]) and slides.
//! * [`bcp`] — Psalter, canticles and collects of the 1979 Prayer Book.
//! * [`lectionary`] — the Sunday and holy-day propers for a date.
//! * [`ewm`] — the Episcopal Worship Markdown reader / writer.
//! * [`service`] — a service plan (ordered cues) and its on-disk TOML format.
//! * [`template`] — the operator's slide layouts (positions and fonts).
//! * [`state`] — the live presentation state and the commands that drive it,
//!   shared by the operator UI, the sync network and the tablet remote.

pub mod bcp;
pub mod bible;
pub mod bulletin;
pub mod calendar;
pub mod category;
pub mod document;
pub mod docx;
pub mod ewm;
pub mod hymn;
pub mod lectionary;
pub mod look;
pub mod paste;
pub mod portable;
pub mod propers;
pub mod role;
pub mod season;
pub mod service;
pub mod state;
pub mod template;

pub use category::ElementCategory;
pub use document::{Block, FrontMatter, Line, LiturgyDocument, Slide, Span};
pub use look::{CueLooks, Fit, Layout, Look, Output, ResolvedLook, TextTone};
pub use role::Role;
pub use season::{LiturgicalColor, Season};
pub use service::{AnnouncementMode, Cue, CueContent, CueOverrides, Service};
pub use state::{Command, CuePosition, PresentationState};
