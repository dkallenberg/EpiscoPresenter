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

    // Within a second of the master's change the stream computer's Next
    // is ignored; a slide click is applied by the master and echoed back.
    client.send(Command::Next).unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(hub.state().active, Some(CuePosition { cue: 0, slide: 0 }));
    client.send(Command::GoTo { cue: 0, slide: 1 }).unwrap();
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

    // The master knows who is following it, until they go.
    assert_eq!(hub.clients(), vec![("stream-node".to_string(), ClientRole::Mirror)]);
    drop(client);
    for _ in 0..50 {
        if hub.clients().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(hub.clients().is_empty(), "a closed connection leaves the list");
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

#[tokio::test]
async fn edits_follow_the_on_air_cue_and_reach_clients() {
    let (hub, server, url) = start(None).await;
    let (_client, mut events) = SyncClient::spawn(ClientConfig::new(&url, "stream-node", ClientRole::Mirror));
    let _ = next_event(&mut events).await; // connected
    let _ = next_event(&mut events).await; // snapshot

    hub.apply(&Command::GoTo { cue: 0, slide: 1 });
    let _ = next_event(&mut events).await;
    let on_air = hub.service().unwrap().cues[0].id.clone();

    // Insert a cue in front: the on-air slide moves to index 1 with it.
    hub.edit_service(|s| {
        let doc = ewm::parse("---\ntitle: Opening\n---\n[Celebrant] Blessed be God.\n").unwrap().document;
        s.insert_cue(0, episco_core::Cue::liturgy("", doc, None, Default::default()));
        Ok::<_, String>(())
    })
    .unwrap();
    match next_event(&mut events).await {
        ClientEvent::Service { service, state } => {
            let service = service.unwrap();
            assert_eq!(service.cues.len(), 2);
            assert_eq!(service.cues[1].id, on_air);
            assert_eq!(state.active, Some(CuePosition { cue: 1, slide: 1 }));
        }
        e => panic!("unexpected {e:?}"),
    }

    // A failed edit changes nothing and sends nothing.
    let rev = hub.state().revision;
    assert!(hub.edit_service(|s| s.move_cue("nope", 0)).is_err());
    assert_eq!(hub.state().revision, rev);

    // Removing the on-air cue returns to pre-service.
    hub.edit_service(|s| s.remove_cue(&on_air).map(drop)).unwrap();
    assert_eq!(hub.state().active, None);
    server.shutdown().await;
}

#[test]
fn another_computers_change_waits_a_second() {
    let hub = SyncHub::new(HubConfig::default());
    hub.load_service(Some(service()));
    assert!(hub.apply(&Command::Next).is_some(), "this computer moves on");
    assert!(hub.apply_from(7, &Command::Next).is_none(), "the other computer's Next is ignored");
    assert!(hub.apply_from(7, &Command::ToggleBlackout).is_none(), "and its Blackout");
    assert!(
        hub.apply_from(7, &Command::StageMessage { text: "Slow down".into() }).is_some(),
        "a stage message goes through"
    );
    assert!(hub.apply(&Command::Next).is_some(), "this computer may keep going");
    assert!(hub.apply_from(7, &Command::GoTo { cue: 0, slide: 0 }).is_some(), "a click still works");
    assert!(hub.apply(&Command::Prev).is_none(), "now this computer waits for the click");
    assert_eq!(hub.state().active, Some(CuePosition { cue: 0, slide: 0 }));
}
