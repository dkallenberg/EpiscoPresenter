//! The master node's authoritative state and its WebSocket server.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::serve::ListenerExt;
use axum::{Json, Router};
use episco_core::{Command, PresentationState, Service};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;

use crate::protocol::{ClientMessage, ClientRole, ServerMessage, HEALTH_PATH, PROTOCOL_VERSION, WS_PATH};

/// How long a new connection has to send `hello`.
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
/// Updates buffered per subscriber before it is considered lagging (it then
/// receives a fresh snapshot instead of the missed updates).
const BROADCAST_CAPACITY: usize = 256;

#[derive(Debug, Clone, Default)]
pub struct HubConfig {
    /// Name reported to clients (e.g. "Sanctuary Mac mini").
    pub server_name: String,
    /// If set, clients must present this token in `hello`.
    pub access_token: Option<String>,
}

/// A change published by the hub, both typed (for in-process listeners such
/// as the Tauri UI) and pre-serialised (so it is encoded once, not per client).
#[derive(Debug)]
pub struct HubUpdate {
    pub message: ServerMessage,
    pub json: String,
}

impl HubUpdate {
    fn new(message: ServerMessage) -> Arc<HubUpdate> {
        let json = serde_json::to_string(&message).expect("server messages always serialise");
        Arc::new(HubUpdate { message, json })
    }

    pub fn revision(&self) -> u64 {
        self.message.revision().unwrap_or(0)
    }
}

struct Inner {
    state: PresentationState,
    service: Option<Arc<Service>>,
    outline: Vec<usize>,
}

/// Owns the live [`PresentationState`] and fans changes out to every client.
pub struct SyncHub {
    config: HubConfig,
    inner: RwLock<Inner>,
    tx: broadcast::Sender<Arc<HubUpdate>>,
    /// Who is connected now: id, name, role.
    clients: Mutex<Vec<(u64, String, ClientRole)>>,
    next_client: AtomicU64,
}

/// Removes a client from [`SyncHub::clients`] when its connection ends.
struct ClientGuard {
    hub: Arc<SyncHub>,
    id: u64,
}

impl Drop for ClientGuard {
    fn drop(&mut self) {
        self.hub.clients.lock().unwrap_or_else(|e| e.into_inner()).retain(|c| c.0 != self.id);
    }
}

impl SyncHub {
    /// The clients connected now (name and role), in the order they came.
    pub fn clients(&self) -> Vec<(String, ClientRole)> {
        self.clients.lock().unwrap_or_else(|e| e.into_inner()).iter().map(|c| (c.1.clone(), c.2)).collect()
    }

    pub fn new(config: HubConfig) -> Arc<SyncHub> {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Arc::new(SyncHub {
            config,
            inner: RwLock::new(Inner { state: PresentationState::default(), service: None, outline: Vec::new() }),
            tx,
            clients: Mutex::new(Vec::new()),
            next_client: AtomicU64::new(1),
        })
    }

    pub fn config(&self) -> &HubConfig {
        &self.config
    }

    pub fn state(&self) -> PresentationState {
        self.read().state.clone()
    }

    pub fn service(&self) -> Option<Arc<Service>> {
        self.read().service.clone()
    }

    pub fn snapshot(&self) -> (PresentationState, Option<Arc<Service>>) {
        let inner = self.read();
        (inner.state.clone(), inner.service.clone())
    }

    /// Receives every subsequent update, in revision order.
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<HubUpdate>> {
        self.tx.subscribe()
    }

    /// Applies a command. Returns the new state if anything changed.
    pub fn apply(&self, command: &Command) -> Option<PresentationState> {
        let mut inner = self.write();
        let Inner { state, outline, .. } = &mut *inner;
        if !state.apply(command, outline) {
            return None;
        }
        let state = state.clone();
        // Sent while holding the lock so subscribers see revisions in order.
        let _ = self.tx.send(HubUpdate::new(ServerMessage::State { state: state.clone() }));
        Some(state)
    }

    /// Replaces the loaded service (or unloads it with `None`). The active
    /// position is kept if it is still valid, otherwise reset to pre-service.
    pub fn load_service(&self, service: Option<Service>) -> PresentationState {
        let mut inner = self.write();
        let service = service.map(Arc::new);
        inner.outline = service.as_ref().map(|s| s.outline()).unwrap_or_default();
        let outline = inner.outline.clone();
        if !inner.state.reconcile(&outline) {
            inner.state.revision += 1;
        }
        inner.service = service.clone();
        let state = inner.state.clone();
        let _ = self.tx.send(HubUpdate::new(ServerMessage::Service { service, state: state.clone() }));
        state
    }

    /// Edits the loaded service in place (starting from an empty service if
    /// none is loaded) and broadcasts the result. The on-air slide follows
    /// its cue by id through reorders and edits; if that cue is removed the
    /// state returns to pre-service. If `edit` fails nothing changes.
    pub fn edit_service<T, E>(&self, edit: impl FnOnce(&mut Service) -> Result<T, E>) -> Result<T, E> {
        let mut inner = self.write();
        let before = inner.service.clone().unwrap_or_default();
        let mut after = (*before).clone();
        let out = edit(&mut after)?;

        inner.state.active = inner.state.active.and_then(|pos| before.remap_position(pos, &after));
        inner.state.revision += 1;
        inner.outline = after.outline();
        let service = Some(Arc::new(after));
        inner.service = service.clone();
        let state = inner.state.clone();
        let _ = self.tx.send(HubUpdate::new(ServerMessage::Service { service, state }));
        Ok(out)
    }

    /// The HTTP routes (`/ws`, `/health`). Exposed so other servers (e.g. the
    /// tablet remote) can mount them alongside their own routes.
    pub fn router(self: &Arc<Self>) -> Router {
        Router::new()
            .route(WS_PATH, get(ws_handler))
            .route(HEALTH_PATH, get(health_handler))
            .with_state(Arc::clone(self))
    }

    /// Binds `addr` and serves [`SyncHub::router`] until the handle is shut down.
    pub async fn serve(self: &Arc<Self>, addr: SocketAddr) -> std::io::Result<ServerHandle> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        // Disable Nagle: updates are tiny and must go out immediately.
        let listener = listener.tap_io(|tcp| {
            if let Err(e) = tcp.set_nodelay(true) {
                tracing::debug!("could not set TCP_NODELAY: {e}");
            }
        });
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let app = self.router();
        let task = tokio::spawn(async move {
            let result = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await;
            if let Err(e) = result {
                tracing::error!("sync server stopped: {e}");
            }
        });
        tracing::info!("sync server listening on {local_addr}");
        Ok(ServerHandle { local_addr, shutdown: Some(shutdown_tx), task })
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Inner> {
        self.inner.read().unwrap_or_else(|e| e.into_inner())
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Inner> {
        self.inner.write().unwrap_or_else(|e| e.into_inner())
    }

    fn snapshot_message(&self, welcome: bool) -> ServerMessage {
        let (state, service) = self.snapshot();
        if welcome {
            ServerMessage::Welcome {
                protocol_version: PROTOCOL_VERSION,
                server_name: self.config.server_name.clone(),
                state,
                service,
            }
        } else {
            ServerMessage::Service { service, state }
        }
    }
}

/// A running sync server.
pub struct ServerHandle {
    local_addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl ServerHandle {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Stops accepting connections and waits for the server task to finish.
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = (&mut self.task).await;
    }
}

async fn health_handler(State(hub): State<Arc<SyncHub>>) -> impl IntoResponse {
    let state = hub.state();
    Json(serde_json::json!({
        "server_name": hub.config.server_name,
        "protocol_version": PROTOCOL_VERSION,
        "revision": state.revision,
        "subscribers": hub.tx.receiver_count(),
    }))
}

async fn ws_handler(ws: WebSocketUpgrade, State(hub): State<Arc<SyncHub>>) -> Response {
    ws.on_upgrade(move |socket| async move {
        if let Err(reason) = connection(socket, hub).await {
            tracing::debug!("sync connection closed: {reason}");
        }
    })
}

async fn send(socket: &mut WebSocket, json: String) -> Result<(), String> {
    socket.send(Message::Text(json.into())).await.map_err(|e| e.to_string())
}

async fn send_msg(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), String> {
    send(socket, serde_json::to_string(msg).expect("server messages always serialise")).await
}

async fn refuse(mut socket: WebSocket, message: String) -> Result<(), String> {
    let _ = send_msg(&mut socket, &ServerMessage::Error { message: message.clone() }).await;
    let _ = socket.send(Message::Close(None)).await;
    Err(message)
}

async fn connection(mut socket: WebSocket, hub: Arc<SyncHub>) -> Result<(), String> {
    // ---- handshake -------------------------------------------------------
    let hello = match tokio::time::timeout(HELLO_TIMEOUT, next_client_message(&mut socket)).await {
        Ok(Some(msg)) => msg,
        Ok(None) => return Err("closed before hello".into()),
        Err(_) => return refuse(socket, "expected `hello` within 5 seconds".into()).await,
    };
    let ClientMessage::Hello { protocol_version, client_name, role, token } = hello else {
        return refuse(socket, "first message must be `hello`".into()).await;
    };
    if protocol_version != PROTOCOL_VERSION {
        return refuse(
            socket,
            format!("protocol version mismatch: server {PROTOCOL_VERSION}, client {protocol_version}"),
        )
        .await;
    }
    if let Some(expected) = &hub.config.access_token {
        if token.as_deref() != Some(expected.as_str()) {
            return refuse(socket, "invalid access token".into()).await;
        }
    }
    tracing::info!("sync client connected: {client_name} ({role:?})");
    let id = hub.next_client.fetch_add(1, Ordering::Relaxed);
    hub.clients.lock().unwrap_or_else(|e| e.into_inner()).push((id, client_name.clone(), role));
    let _guard = ClientGuard { hub: Arc::clone(&hub), id };

    // Subscribe before taking the snapshot so nothing falls in between;
    // anything not newer than the snapshot is skipped below.
    let mut updates = hub.subscribe();
    let welcome = hub.snapshot_message(true);
    let mut last_revision = welcome.revision().unwrap_or(0);
    send_msg(&mut socket, &welcome).await?;

    // ---- steady state ----------------------------------------------------
    loop {
        tokio::select! {
            update = updates.recv() => match update {
                Ok(update) => {
                    if update.revision() <= last_revision {
                        continue;
                    }
                    last_revision = update.revision();
                    send(&mut socket, update.json.clone()).await?;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("{client_name} lagged by {n} updates; resending snapshot");
                    let snapshot = hub.snapshot_message(false);
                    last_revision = snapshot.revision().unwrap_or(last_revision);
                    send_msg(&mut socket, &snapshot).await?;
                }
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
            },
            incoming = next_client_message(&mut socket) => match incoming {
                None => return Ok(()),
                Some(ClientMessage::Command { command }) => {
                    if role.may_command() {
                        hub.apply(&command);
                    } else {
                        send_msg(&mut socket, &ServerMessage::Error {
                            message: "monitor clients may not send commands".into(),
                        }).await?;
                    }
                }
                Some(ClientMessage::Ping { nonce }) => {
                    send_msg(&mut socket, &ServerMessage::Pong { nonce }).await?;
                }
                Some(ClientMessage::Hello { .. }) => {}
            },
        }
    }
}

/// Reads frames until a parseable client message arrives. `None` on close.
async fn next_client_message(socket: &mut WebSocket) -> Option<ClientMessage> {
    loop {
        match socket.recv().await? {
            Ok(Message::Text(text)) => match serde_json::from_str(text.as_str()) {
                Ok(msg) => return Some(msg),
                Err(e) => tracing::warn!("ignoring malformed sync message: {e}"),
            },
            Ok(Message::Close(_)) | Err(_) => return None,
            Ok(_) => {} // ping/pong/binary
        }
    }
}
