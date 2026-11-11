//! Per-machine node configuration (`node.json` in the app config directory).

use std::net::{Ipv4Addr, SocketAddr};

use serde::{Deserialize, Serialize};

use crate::protocol::{DEFAULT_SYNC_PORT, WS_PATH};

/// What this machine does in the two-node sanctuary / live-stream setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    /// Sanctuary Mac: operator UI + 85" sanctuary output; hosts the sync server.
    Master,
    /// Live-stream Mac: status monitor + keyed stream overlay; follows the master.
    Slave,
    /// One machine driving both outputs; hosts the sync server for remotes.
    #[default]
    Standalone,
}

/// A full-screen output surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    /// 85" sanctuary display: opaque, full-screen.
    Sanctuary,
    /// Live-stream overlay: transparent / keyable.
    Stream,
}

impl OutputKind {
    /// Window label used by the desktop app.
    pub fn window_label(self) -> &'static str {
        match self {
            OutputKind::Sanctuary => "sanctuary",
            OutputKind::Stream => "stream",
        }
    }
}

impl NodeRole {
    /// Output windows this node opens.
    pub fn outputs(self) -> &'static [OutputKind] {
        match self {
            NodeRole::Master => &[OutputKind::Sanctuary],
            NodeRole::Slave => &[OutputKind::Stream],
            NodeRole::Standalone => &[OutputKind::Sanctuary, OutputKind::Stream],
        }
    }

    /// Whether this node owns the authoritative state and runs the sync server.
    pub fn hosts_sync(self) -> bool {
        !matches!(self, NodeRole::Slave)
    }
}

/// Which physical monitor (index into the OS monitor list) shows each output.
/// Monitor 0 is assumed to be the operator's control screen.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub sanctuary_monitor: Option<usize>,
    pub stream_monitor: Option<usize>,
    /// Open outputs as ordinary windows instead of full-screen (rehearsal / dev).
    pub windowed: bool,
    /// Open the output windows when the app starts (as before). Off by
    /// default: the operator opens them from the control window (Outputs).
    pub open_at_start: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NodeConfig {
    pub role: NodeRole,
    /// Shown to other nodes, e.g. "Sanctuary Mac mini".
    pub node_name: String,
    /// Where the sync server listens (master / standalone).
    pub sync_listen: SocketAddr,
    /// Master's sync URL (slave), e.g. `ws://192.168.1.20:7878/ws`.
    pub master_url: String,
    /// Optional shared secret all nodes and remotes must present.
    pub access_token: Option<String>,
    pub displays: DisplayConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            role: NodeRole::default(),
            node_name: "EpiscoPresenter".into(),
            sync_listen: SocketAddr::from((Ipv4Addr::UNSPECIFIED, DEFAULT_SYNC_PORT)),
            master_url: format!("ws://127.0.0.1:{DEFAULT_SYNC_PORT}{WS_PATH}"),
            access_token: None,
            displays: DisplayConfig::default(),
        }
    }
}

impl NodeConfig {
    /// Monitor index for an output, defaulting by role: with one output it
    /// goes on monitor 1; standalone puts sanctuary on 1 and stream on 2.
    pub fn monitor_for(&self, output: OutputKind) -> usize {
        let explicit = match output {
            OutputKind::Sanctuary => self.displays.sanctuary_monitor,
            OutputKind::Stream => self.displays.stream_monitor,
        };
        explicit.unwrap_or(match (self.role, output) {
            (NodeRole::Standalone, OutputKind::Stream) => 2,
            _ => 1,
        })
    }

    /// Checks settings that would otherwise fail later at runtime.
    pub fn validate(&self) -> Result<(), String> {
        if self.role == NodeRole::Slave && !self.master_url.starts_with("ws://") {
            return Err(format!("master_url must start with ws:// (got `{}`)", self.master_url));
        }
        if self.access_token.as_deref() == Some("") {
            return Err("access_token must not be empty; omit it to disable".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_partial_json() {
        let cfg: NodeConfig = serde_json::from_str(r#"{"role":"slave","master_url":"ws://10.0.0.5:7878/ws"}"#).unwrap();
        assert_eq!(cfg.role, NodeRole::Slave);
        assert_eq!(cfg.sync_listen.port(), DEFAULT_SYNC_PORT);
        assert_eq!(cfg.monitor_for(OutputKind::Stream), 1);
        assert!(cfg.validate().is_ok());
        assert!(!cfg.role.hosts_sync());
    }

    #[test]
    fn standalone_uses_two_output_monitors() {
        let cfg = NodeConfig::default();
        assert_eq!(cfg.role.outputs(), &[OutputKind::Sanctuary, OutputKind::Stream]);
        assert_eq!(cfg.monitor_for(OutputKind::Sanctuary), 1);
        assert_eq!(cfg.monitor_for(OutputKind::Stream), 2);
    }

    #[test]
    fn rejects_bad_master_url() {
        let cfg = NodeConfig { role: NodeRole::Slave, master_url: "http://x".into(), ..Default::default() };
        assert!(cfg.validate().is_err());
    }
}
