//! Core data model for EpiscoPresenter.
//!
//! * [`role`], [`season`], [`category`] — liturgical vocabulary and the
//!   presentation rules derived from it.
//! * [`document`] — a parsed liturgy item ([`LiturgyDocument`]) and slides.
//! * [`ewm`] — the Episcopal Worship Markdown reader / writer.
//! * [`service`] — a service plan (ordered cues) and its on-disk TOML format.
//! * [`state`] — the live presentation state and the commands that drive it,
//!   shared by the operator UI, the sync network and the tablet remote.

pub mod category;
pub mod document;
pub mod ewm;
pub mod role;
pub mod season;
pub mod service;
pub mod state;

pub use category::{ElementCategory, SanctuaryLayout, StreamZone};
pub use document::{Block, FrontMatter, Line, LiturgyDocument, Slide, Span};
pub use role::Role;
pub use season::{LiturgicalColor, Season};
pub use service::{AnnouncementMode, Cue, CueContent, CueOverrides, Service};
pub use state::{Command, CuePosition, PresentationState};
