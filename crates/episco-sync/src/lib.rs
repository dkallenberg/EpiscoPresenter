//! Network synchronisation between EpiscoPresenter nodes.
//!
//! The **master** (sanctuary) node owns a [`SyncHub`]: the authoritative
//! [`PresentationState`](episco_core::PresentationState) plus the loaded
//! [`Service`](episco_core::Service). It serves a WebSocket endpoint; every
//! state change is serialised once and pushed to all connected clients.
//!
//! The **slave** (live-stream) node runs a [`SyncClient`] that mirrors the
//! master, reconnecting automatically, and forwards its own operator commands
//! back to the master. The tablet remote speaks the same [`protocol`].

pub mod client;
pub mod config;
pub mod hub;
pub mod protocol;

pub use client::{ClientConfig, ClientEvent, SyncClient};
pub use config::{DisplayConfig, NodeConfig, NodeRole, OutputKind};
pub use hub::{HubConfig, HubUpdate, ServerHandle, SyncHub};
pub use protocol::{ClientMessage, ClientRole, ServerMessage, DEFAULT_SYNC_PORT, PROTOCOL_VERSION};
