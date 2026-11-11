//! Wire protocol between the master node and its clients.
//!
//! JSON text frames over a WebSocket at `ws://<master>:7878/ws`.
//!
//! ```text
//! client                                   master
//!   | -- hello {protocol_version, ...} -->   |
//!   | <-- welcome {state, service} -------   |   full snapshot
//!   | <-- state {state} ------------------   |   on every change
//!   | <-- service {service, state} -------   |   when a new service is loaded
//!   | -- command {command} -------------->   |   e.g. next / prev / blackout
//!   | -- ping {nonce} ------------------->   |
//!   | <-- pong {nonce} -------------------   |
//! ```
//!
//! Every update carries `state.revision`; it increases monotonically, so a
//! client can always discard anything not newer than what it has.

use std::sync::Arc;

use episco_core::{Command, PresentationState, Service};
use serde::{Deserialize, Serialize};

/// Bumped on any incompatible change to the messages below.
pub const PROTOCOL_VERSION: u32 = 1;
/// Default TCP port of the sync server on the master node.
pub const DEFAULT_SYNC_PORT: u16 = 7878;
/// HTTP path of the WebSocket endpoint.
pub const WS_PATH: &str = "/ws";
/// HTTP path of the JSON health endpoint.
pub const HEALTH_PATH: &str = "/health";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        protocol_version: u32,
        server_name: String,
        state: PresentationState,
        service: Option<Arc<Service>>,
    },
    State {
        state: PresentationState,
    },
    Service {
        service: Option<Arc<Service>>,
        state: PresentationState,
    },
    Pong {
        nonce: u64,
    },
    /// Sent before the server closes a connection it refuses.
    Error {
        message: String,
    },
}

impl ServerMessage {
    /// The revision this message brings the client to, if it carries state.
    pub fn revision(&self) -> Option<u64> {
        match self {
            ServerMessage::Welcome { state, .. }
            | ServerMessage::State { state }
            | ServerMessage::Service { state, .. } => Some(state.revision),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        protocol_version: u32,
        client_name: String,
        role: ClientRole,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },
    Command {
        command: Command,
    },
    Ping {
        nonce: u64,
    },
}

/// What kind of client is connecting. Determines whether it may send commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientRole {
    /// Another presentation node (e.g. the live-stream Mac) mirroring outputs.
    /// May also send commands from its secondary control screen.
    Mirror,
    /// A tablet / phone remote.
    Remote,
    /// Read-only status display; commands are refused.
    Monitor,
}

impl ClientRole {
    pub fn may_command(self) -> bool {
        !matches!(self, ClientRole::Monitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_json_shape() {
        let hello = ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
            client_name: "stream".into(),
            role: ClientRole::Mirror,
            token: None,
        };
        assert_eq!(
            serde_json::to_string(&hello).unwrap(),
            r#"{"type":"hello","protocol_version":1,"client_name":"stream","role":"mirror"}"#
        );
        let cmd: ClientMessage = serde_json::from_str(r#"{"type":"command","command":{"type":"next"}}"#).unwrap();
        assert_eq!(cmd, ClientMessage::Command { command: Command::Next });
    }
}
