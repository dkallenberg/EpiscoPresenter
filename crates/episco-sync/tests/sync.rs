//! End-to-end tests: a real master server and clients over localhost.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use episco_core::{ewm, Command, CuePosition, Service};
use episco_sync::{ClientConfig, ClientEvent, ClientRole, HubConfig, SyncClient, SyncHub};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::tungstenite::Message;

const SRC: &str =
    "---\ntitle: Sursum Corda\n---\n[Priest] The Lord be with you.\n---\n[People] **And also with you.**\n";

fn service() -> Service {
    Service::from_document(ewm::parse(SRC).unwrap().document, None)
}

async fn start(token: Option<&str>) -> (Arc<SyncHub>, episco_sync::ServerHandle, String) {
    let hub = SyncHub::new(HubConfig { server_name: "test-master".into(), access_token: token.map(String::from) });
    hub.load_service(Some(service()));
    let handle = hub.serve(SocketAddr::from(([127, 0, 0, 1], 0))).await.unwrap();
    let url = format!("ws://{}/ws", handle.local_addr());
    (hub, handle, url)
}

async fn next_event(rx: &mut UnboundedReceiver<ClientEvent>) -> ClientEvent {
    loop {
        let e = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await.expect("timed out").expect("closed");
        if !matches!(e, ClientEvent::Latency(_)) {
            return e;
        }
    }
}

#[tokio::test]
async fn mirror_receives_snapshot_and_updates_and_can_command() {
    let (hub, server, url) = start(None).await;
    let (client, mut events) = SyncClient::spawn(ClientConfig::new(&url, "stream-node", ClientRole::Mirror));

    match next_event(&mut events).await {
        ClientEvent::Connected { server_name } => assert_eq!(server_name, "test-master"),
        e => panic!("unexpected {e:?}"),
    }
    match next_event(&mut events).await {
        ClientEvent::Service { service, state } => {
            assert_eq!(service.unwrap().cues[0].slide_count(), 2);
            assert_eq!(state.active, None);
        }
        e => panic!("unexpected {e:?}"),
    }

    // Master-side change reaches the client.
    hub.apply(&Command::Next);
    match next_event(&mut events).await {
        ClientEvent::State(s) => assert_eq!(s.active, Some(CuePosition { cue: 0, slide: 0 })),
        e => panic!("unexpected {e:?}"),
    }

    // Client-side command is applied by the master and echoed back.
    client.send(Command::Next).unwrap();
    match next_event(&mut events).await {
        ClientEvent::State(s) => assert_eq!(s.active, Some(CuePosition { cue: 0, slide: 1 })),
        e => panic!("unexpected {e:?}"),
    }
    assert_eq!(hub.state().active, Some(CuePosition { cue: 0, slide: 1 }));

    // Reloading the service is pushed too.
    hub.load_service(None);
    match next_event(&mut events).await {
        ClientEvent::Service { service, state } => {
            assert!(service.is_none());
            assert_eq!(state.active, None);
        }
        e => panic!("unexpected {e:?}"),
    }

    drop(client);
    server.shutdown().await;
}

#[tokio::test]
async fn wrong_token_is_fatal() {
    let (_hub, server, url) = start(Some("secret")).await;
    let mut cfg = ClientConfig::new(&url, "intruder", ClientRole::Remote);
    cfg.token = Some("guess".into());
    let (_client, mut events) = SyncClient::spawn(cfg);
    match next_event(&mut events).await {
        ClientEvent::Disconnected { reason, fatal } => {
            assert!(fatal);
            assert!(reason.contains("token"), "{reason}");
        }
        e => panic!("unexpected {e:?}"),
    }
    server.shutdown().await;
}

#[tokio::test]
async fn monitor_clients_cannot_command() {
    let (hub, server, url) = start(None).await;
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    let send = |v: serde_json::Value| Message::Text(v.to_string().into());
    ws.send(send(serde_json::json!({"type":"hello","protocol_version":1,"client_name":"wall","role":"monitor"})))
        .await
        .unwrap();
    let welcome = ws.next().await.unwrap().unwrap();
    assert!(welcome.to_text().unwrap().contains("\"welcome\""));
    ws.send(send(serde_json::json!({"type":"command","command":{"type":"next"}}))).await.unwrap();
    let reply = ws.next().await.unwrap().unwrap();
    assert!(reply.to_text().unwrap().contains("\"error\""));
    assert_eq!(hub.state().active, None);
    server.shutdown().await;
}

#[tokio::test]
async fn protocol_mismatch_is_refused() {
    let (_hub, server, url) = start(None).await;
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    ws.send(Message::Text(r#"{"type":"hello","protocol_version":999,"client_name":"old","role":"remote"}"#.into()))
        .await
        .unwrap();
    let reply = ws.next().await.unwrap().unwrap();
    assert!(reply.to_text().unwrap().contains("protocol version mismatch"));
    server.shutdown().await;
}

/// Every update arrives, in order, with strictly increasing revisions; also
/// reports local round-trip latency for a command.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rapid_updates_arrive_in_order() {
    let (hub, server, url) = start(None).await;
    let (_client, mut events) = SyncClient::spawn(ClientConfig::new(&url, "stream-node", ClientRole::Mirror));
    let _ = next_event(&mut events).await; // connected
    let _ = next_event(&mut events).await; // snapshot

    const N: u64 = 200;
    let t0 = Instant::now();
    for _ in 0..N {
        hub.apply(&Command::ToggleBlackout);
    }
    let mut last = 0;
    for _ in 0..N {
        match next_event(&mut events).await {
            ClientEvent::State(s) => {
                assert!(s.revision > last);
                last = s.revision;
            }
            e => panic!("unexpected {e:?}"),
        }
    }
    let per_update = t0.elapsed() / N as u32;
    eprintln!("{N} updates delivered, ~{per_update:?} per update (localhost)");
    assert!(!hub.state().blackout); // even number of toggles
    server.shutdown().await;
}
