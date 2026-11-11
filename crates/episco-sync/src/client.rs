//! Client side: used by the slave (live-stream) node to mirror the master.

use std::sync::Arc;
use std::time::{Duration, Instant};

use episco_core::{Command, PresentationState, Service};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::protocol::{ClientMessage, ClientRole, ServerMessage, PROTOCOL_VERSION};

#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// e.g. `ws://192.168.1.20:7878/ws`
    pub url: String,
    pub client_name: String,
    pub role: ClientRole,
    pub token: Option<String>,
    /// First reconnect delay; doubles on each failure up to `reconnect_max`.
    pub reconnect_min: Duration,
    pub reconnect_max: Duration,
    /// Interval between heartbeat pings (also used to report latency).
    pub heartbeat: Duration,
    /// Reconnect if nothing is heard from the master for this long.
    pub silence_timeout: Duration,
}

impl ClientConfig {
    pub fn new(url: impl Into<String>, client_name: impl Into<String>, role: ClientRole) -> Self {
        ClientConfig {
            url: url.into(),
            client_name: client_name.into(),
            role,
            token: None,
            reconnect_min: Duration::from_millis(250),
            reconnect_max: Duration::from_secs(5),
            heartbeat: Duration::from_secs(2),
            silence_timeout: Duration::from_secs(6),
        }
    }
}

/// Events delivered to the application, in order.
#[derive(Debug, Clone)]
pub enum ClientEvent {
    Connected {
        server_name: String,
    },
    /// Connection lost or refused; the client keeps retrying unless `fatal`.
    Disconnected {
        reason: String,
        fatal: bool,
    },
    /// A new presentation state (revision is always newer than the last one).
    State(PresentationState),
    /// The master loaded a different service.
    Service {
        service: Option<Arc<Service>>,
        state: PresentationState,
    },
    /// Round-trip time of the latest heartbeat.
    Latency(Duration),
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("sync client has stopped")]
    Stopped,
}

/// A self-reconnecting connection to the master's sync server.
///
/// Dropping the client stops it.
pub struct SyncClient {
    commands: mpsc::UnboundedSender<Command>,
    task: JoinHandle<()>,
}

impl SyncClient {
    /// Starts the client on the current Tokio runtime.
    pub fn spawn(config: ClientConfig) -> (SyncClient, mpsc::UnboundedReceiver<ClientEvent>) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (evt_tx, evt_rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(run(config, cmd_rx, evt_tx));
        (SyncClient { commands: cmd_tx, task }, evt_rx)
    }

    /// Sends a command to the master. Commands issued while disconnected are
    /// discarded rather than replayed later (a stale "next" is worse than none).
    pub fn send(&self, command: Command) -> Result<(), ClientError> {
        self.commands.send(command).map_err(|_| ClientError::Stopped)
    }
}

impl Drop for SyncClient {
    fn drop(&mut self) {
        self.task.abort();
    }
}

enum Outcome {
    /// Retry after a delay.
    Retry(String),
    /// Give up (bad config, version mismatch, wrong token).
    Fatal(String),
    /// The application dropped its event receiver / client.
    Stop,
}

async fn run(
    config: ClientConfig,
    mut commands: mpsc::UnboundedReceiver<Command>,
    events: mpsc::UnboundedSender<ClientEvent>,
) {
    let mut delay = config.reconnect_min;
    let mut last_revision: Option<u64> = None;
    loop {
        let started = Instant::now();
        match session(&config, &mut commands, &events, &mut last_revision).await {
            Outcome::Stop => return,
            Outcome::Fatal(reason) => {
                let _ = events.send(ClientEvent::Disconnected { reason, fatal: true });
                return;
            }
            Outcome::Retry(reason) => {
                if events.send(ClientEvent::Disconnected { reason, fatal: false }).is_err() {
                    return;
                }
            }
        }
        // A connection that lived a while resets the backoff.
        if started.elapsed() > config.reconnect_max {
            delay = config.reconnect_min;
        }
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(config.reconnect_max);
        // Drop anything pressed while we were offline.
        while commands.try_recv().is_ok() {}
    }
}

type Ws = WebSocketStream<TcpStream>;

async fn connect(config: &ClientConfig) -> Result<Ws, Outcome> {
    let request = config
        .url
        .as_str()
        .into_client_request()
        .map_err(|e| Outcome::Fatal(format!("invalid master URL `{}`: {e}", config.url)))?;
    let uri = request.uri();
    if uri.scheme_str() != Some("ws") {
        return Err(Outcome::Fatal(format!("master URL must start with ws:// (got `{}`)", config.url)));
    }
    let host = uri.host().ok_or_else(|| Outcome::Fatal("master URL has no host".into()))?.to_string();
    let port = uri.port_u16().unwrap_or(80);
    let tcp = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|e| Outcome::Retry(format!("cannot reach {host}:{port}: {e}")))?;
    let _ = tcp.set_nodelay(true);
    let (ws, _) = tokio_tungstenite::client_async(request, tcp)
        .await
        .map_err(|e| Outcome::Retry(format!("websocket handshake failed: {e}")))?;
    Ok(ws)
}

async fn send(ws: &mut Ws, msg: &ClientMessage) -> Result<(), Outcome> {
    let json = serde_json::to_string(msg).expect("client messages always serialise");
    ws.send(Message::Text(json.into())).await.map_err(|e| Outcome::Retry(format!("send failed: {e}")))
}

async fn session(
    config: &ClientConfig,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    events: &mpsc::UnboundedSender<ClientEvent>,
    last_revision: &mut Option<u64>,
) -> Outcome {
    let mut ws = match connect(config).await {
        Ok(ws) => ws,
        Err(outcome) => return outcome,
    };
    let hello = ClientMessage::Hello {
        protocol_version: PROTOCOL_VERSION,
        client_name: config.client_name.clone(),
        role: config.role,
        token: config.token.clone(),
    };
    if let Err(o) = send(&mut ws, &hello).await {
        return o;
    }

    let epoch = Instant::now();
    let mut heartbeat = tokio::time::interval(config.heartbeat);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_heard = Instant::now();
    let mut welcomed = false;

    let emit = |e: ClientEvent| events.send(e).is_ok();

    loop {
        tokio::select! {
            frame = ws.next() => {
                let text = match frame {
                    Some(Ok(Message::Text(t))) => t,
                    Some(Ok(Message::Close(_))) | None => return Outcome::Retry("master closed the connection".into()),
                    Some(Err(e)) => return Outcome::Retry(format!("connection error: {e}")),
                    Some(Ok(_)) => { last_heard = Instant::now(); continue; }
                };
                last_heard = Instant::now();
                let msg: ServerMessage = match serde_json::from_str(text.as_str()) {
                    Ok(m) => m,
                    Err(e) => { tracing::warn!("ignoring malformed message from master: {e}"); continue; }
                };
                // Discard anything not newer than what we already have, except
                // the welcome snapshot (the master may have restarted and
                // reset its revision counter).
                if let (Some(rev), Some(last)) = (msg.revision(), *last_revision) {
                    if rev <= last && !matches!(msg, ServerMessage::Welcome { .. }) {
                        continue;
                    }
                }
                if let Some(rev) = msg.revision() {
                    *last_revision = Some(rev);
                }
                let ok = match msg {
                    ServerMessage::Welcome { server_name, state, service, .. } => {
                        welcomed = true;
                        emit(ClientEvent::Connected { server_name })
                            && emit(ClientEvent::Service { service, state })
                    }
                    ServerMessage::State { state } => emit(ClientEvent::State(state)),
                    ServerMessage::Service { service, state } => emit(ClientEvent::Service { service, state }),
                    ServerMessage::Pong { nonce } => {
                        let rtt = epoch.elapsed().saturating_sub(Duration::from_micros(nonce));
                        emit(ClientEvent::Latency(rtt))
                    }
                    ServerMessage::Error { message } if !welcomed => return Outcome::Fatal(message),
                    ServerMessage::Error { message } => {
                        tracing::warn!("master reported: {message}");
                        true
                    }
                };
                if !ok {
                    return Outcome::Stop;
                }
            }
            cmd = commands.recv() => match cmd {
                Some(command) => {
                    if let Err(o) = send(&mut ws, &ClientMessage::Command { command }).await {
                        return o;
                    }
                }
                None => return Outcome::Stop,
            },
            _ = heartbeat.tick() => {
                if last_heard.elapsed() > config.silence_timeout {
                    return Outcome::Retry("master stopped responding".into());
                }
                let nonce = epoch.elapsed().as_micros() as u64;
                if let Err(o) = send(&mut ws, &ClientMessage::Ping { nonce }).await {
                    return o;
                }
            }
        }
    }
}
