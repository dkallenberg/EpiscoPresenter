//! The presentation engine as seen by this node.
//!
//! On a master / standalone node the engine *is* the [`SyncHub`]. On a slave
//! node it is a mirror kept current by a [`SyncClient`]. Either way the rest
//! of the app talks to it the same way, and every change is re-emitted to all
//! local windows as Tauri events.

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use episco_core::{Command, PresentationState, Service};
use episco_sync::{
    ClientConfig, ClientEvent, ClientRole, HubConfig, NodeConfig, NodeRole, ServerHandle, ServerMessage, SyncClient,
    SyncHub,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Event names emitted to every window.
pub mod events {
    /// Payload: `PresentationState`.
    pub const STATE: &str = "episco://state";
    /// Payload: `Option<Service>`.
    pub const SERVICE: &str = "episco://service";
    /// Payload: `LinkStatus`.
    pub const LINK: &str = "episco://link";
}

/// Network status shown in the control UI.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LinkStatus {
    pub role: NodeRole,
    /// Master: the sync server is listening. Slave: connected to the master.
    pub online: bool,
    /// Master: listen address. Slave: master URL.
    pub endpoint: String,
    /// Slave: the master's name once connected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer: Option<String>,
    /// Slave: last heartbeat round trip in microseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Everything a window needs to render from scratch.
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub state: PresentationState,
    pub service: Option<Arc<Service>>,
    pub link: LinkStatus,
}

pub enum Engine {
    Host {
        hub: Arc<SyncHub>,
        link: Arc<RwLock<LinkStatus>>,
        // Kept alive for the life of the app.
        _server: Option<ServerHandle>,
    },
    Follower {
        client: SyncClient,
        mirror: Arc<RwLock<Mirror>>,
    },
}

#[derive(Default)]
pub struct Mirror {
    state: PresentationState,
    service: Option<Arc<Service>>,
    link: LinkStatus,
}

impl Engine {
    /// Starts the engine for `config`. Must be called inside a Tokio runtime.
    pub async fn start(app: AppHandle, config: &NodeConfig) -> Engine {
        if config.role.hosts_sync() {
            Self::start_host(app, config).await
        } else {
            Self::start_follower(app, config)
        }
    }

    async fn start_host(app: AppHandle, config: &NodeConfig) -> Engine {
        let hub = SyncHub::new(HubConfig {
            server_name: config.node_name.clone(),
            access_token: config.access_token.clone(),
        });
        let mut link = LinkStatus { role: config.role, endpoint: config.sync_listen.to_string(), ..Default::default() };
        let server = match hub.serve(config.sync_listen).await {
            Ok(handle) => {
                link.online = true;
                link.endpoint = display_addr(handle.local_addr());
                Some(handle)
            }
            Err(e) => {
                // Keep running: the local outputs still work without the network.
                let msg = format!("sync server could not listen on {}: {e}", config.sync_listen);
                tracing::error!("{msg}");
                link.error = Some(msg);
                None
            }
        };

        crate::autosave::spawn(app.clone(), Arc::clone(&hub));

        let mut updates = hub.subscribe();
        tokio::spawn(async move {
            use tokio::sync::broadcast::error::RecvError;
            loop {
                match updates.recv().await {
                    Ok(update) => match &update.message {
                        ServerMessage::State { state } => {
                            let _ = app.emit(events::STATE, state);
                        }
                        ServerMessage::Service { service, state } => {
                            let _ = app.emit(events::SERVICE, service);
                            let _ = app.emit(events::STATE, state);
                        }
                        _ => {}
                    },
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        });

        Engine::Host { hub, link: Arc::new(RwLock::new(link)), _server: server }
    }

    fn start_follower(app: AppHandle, config: &NodeConfig) -> Engine {
        let mut client_cfg = ClientConfig::new(&config.master_url, &config.node_name, ClientRole::Mirror);
        client_cfg.token = config.access_token.clone();
        let (client, mut events_rx) = SyncClient::spawn(client_cfg);
        let mirror = Arc::new(RwLock::new(Mirror {
            link: LinkStatus { role: config.role, endpoint: config.master_url.clone(), ..Default::default() },
            ..Default::default()
        }));

        let m = Arc::clone(&mirror);
        tokio::spawn(async move {
            while let Some(event) = events_rx.recv().await {
                let mut mirror = m.write().unwrap_or_else(|e| e.into_inner());
                match event {
                    ClientEvent::Connected { server_name } => {
                        mirror.link.online = true;
                        mirror.link.peer = Some(server_name);
                        mirror.link.error = None;
                        let _ = app.emit(events::LINK, &mirror.link);
                    }
                    ClientEvent::Disconnected { reason, fatal } => {
                        mirror.link.online = false;
                        mirror.link.latency_us = None;
                        mirror.link.error = Some(if fatal { format!("{reason} (not retrying)") } else { reason });
                        let _ = app.emit(events::LINK, &mirror.link);
                    }
                    ClientEvent::State(state) => {
                        let _ = app.emit(events::STATE, &state);
                        mirror.state = state;
                    }
                    ClientEvent::Service { service, state } => {
                        let _ = app.emit(events::SERVICE, &service);
                        let _ = app.emit(events::STATE, &state);
                        mirror.service = service;
                        mirror.state = state;
                    }
                    ClientEvent::Latency(rtt) => {
                        mirror.link.latency_us = Some(rtt.as_micros() as u64);
                        let _ = app.emit(events::LINK, &mirror.link);
                    }
                }
            }
        });

        Engine::Follower { client, mirror }
    }

    pub fn snapshot(&self) -> Snapshot {
        match self {
            Engine::Host { hub, link, .. } => {
                let (state, service) = hub.snapshot();
                Snapshot { state, service, link: link.read().unwrap_or_else(|e| e.into_inner()).clone() }
            }
            Engine::Follower { mirror, .. } => {
                let m = mirror.read().unwrap_or_else(|e| e.into_inner());
                Snapshot { state: m.state.clone(), service: m.service.clone(), link: m.link.clone() }
            }
        }
    }

    pub fn send(&self, command: Command) -> Result<(), String> {
        match self {
            Engine::Host { hub, .. } => {
                hub.apply(&command);
                Ok(())
            }
            Engine::Follower { client, .. } => client.send(command).map_err(|e| e.to_string()),
        }
    }

    pub fn load_service(&self, service: Option<Service>) -> Result<(), String> {
        match self {
            Engine::Host { hub, .. } => {
                // Every service is shown with the operator's slide layouts.
                let service = service.map(|mut s| {
                    s.templates = crate::templates::current();
                    s
                });
                hub.load_service(service);
                Ok(())
            }
            Engine::Follower { .. } => Err(FOLLOWER_EDIT.into()),
        }
    }

    /// Edits the live service (master / standalone only).
    pub fn edit_service<T>(&self, edit: impl FnOnce(&mut Service) -> Result<T, String>) -> Result<T, String> {
        match self {
            Engine::Host { hub, .. } => hub.edit_service(edit),
            Engine::Follower { .. } => Err(FOLLOWER_EDIT.into()),
        }
    }

    /// The live service, if one is loaded.
    pub fn service(&self) -> Option<Arc<Service>> {
        self.snapshot().service
    }
}

const FOLLOWER_EDIT: &str = "this is a slave node; edit the service on the master (sanctuary) node";

fn display_addr(addr: SocketAddr) -> String {
    if addr.ip().is_unspecified() {
        format!("all interfaces, port {}", addr.port())
    } else {
        addr.to_string()
    }
}
