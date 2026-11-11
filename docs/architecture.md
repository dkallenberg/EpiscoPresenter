# Architecture

## Workspace

| Crate | Purpose | Depends on |
| --- | --- | --- |
| `crates/episco-core` | Data model, `.ewm` parser and writer, service plans, presentation state machine. No I/O beyond reading files and no async code. | `serde`, `toml` |
| `crates/episco-sync` | Node configuration and the master/slave WebSocket sync (server and client). | `episco-core`, `tokio`, `axum`, `tokio-tungstenite` |
| `src-tauri` | Desktop app: starts the engine for this node's role, opens output windows on request, exposes Tauri commands. | both of the above, `tauri` 2 |
| `ui/` | Plain HTML/CSS/ES-module front-end with no build step: control dashboard, `output/sanctuary.html`, `output/stream.html`. | |

`cargo build` / `cargo test` at the workspace root covers the two library
crates. The app is built with `cargo build -p episco-presenter` or the Tauri CLI.

## Data flow

```
 .ewm files ─┐
             ├─► Service::load_plan ─► Service ─┐
 service.toml┘                                   │
                                                 ▼
 operator UI / tablet / slave ──Command──► SyncHub (master)
                                           │  PresentationState::apply
                                           ▼
                           broadcast (JSON, encoded once)
                    ┌──────────────┼──────────────────┐
                    ▼              ▼                  ▼
          master windows     slave node (client)   tablet remote
          (Tauri events)     → its windows
```

* **`Service`** is fully resolved: every cue carries its parsed document,
  slides, effective colour/background, lower-third caption and layout zones.
  The master sends it to clients, so slave nodes need no access to the files.
* **`PresentationState`** (active cue/slide, blackout, cleared layers, stage
  message, `revision`) changes only through `PresentationState::apply(command,
  outline)`. This is a pure function, so it is unit-tested and every node agrees.
* **`revision`** rises by one on every change, including a service load. Clients
  drop any message that is not newer than what they hold.

## Nodes

| Role | Control display | Output display | Network |
| --- | --- | --- | --- |
| `master` | operator dashboard | sanctuary (opaque, full-screen) | sync server on `:7878` |
| `slave` | dashboard (status and secondary control) | stream overlay (transparent) | client of the master |
| `standalone` | operator dashboard | sanctuary **and** stream on separate monitors | sync server (for remotes) |

Each machine reads `node.json` from its app config directory. On macOS that is
`~/Library/Application Support/org.episcopresenter.app/node.json`. The file is
written with defaults on first run.

```json
{
  "role": "master",
  "node_name": "Sanctuary Mac mini",
  "sync_listen": "0.0.0.0:7878",
  "master_url": "ws://192.168.1.20:7878/ws",
  "access_token": null,
  "displays": { "sanctuary_monitor": null, "stream_monitor": null, "windowed": false, "open_at_start": false }
}
```

* The output windows do **not** open at start-up. The operator opens them
  from the control window's top bar (**▣ Outputs**): **Full screen** on a
  monitor or as a **Window**, and **Close**. In an output window, Esc leaves
  full screen and a double-click toggles it.
* `sanctuary_monitor` / `stream_monitor` (indexes into the OS monitor list)
  set the monitor **Full screen** offers first. When unset it is the "second
  monitor": the first screen other than the one showing the control window;
  on a standalone machine the stream takes the next screen after that.
* `open_at_start: true` opens the outputs when the app starts, full screen
  (or as half-size windows with `windowed: true`), as earlier versions did.
* `access_token`, if set, must also be set on the slave and is required from
  tablets. Without it, any device on the LAN can drive the presentation.

The environment variables `EPISCO_CONFIG` (config path), `EPISCO_ROLE` and
`EPISCO_MASTER_URL` override the file. This lets you run a master and a slave
on one development machine:

```bash
EPISCO_ROLE=master cargo run -p episco-presenter
```

```bash
EPISCO_ROLE=slave EPISCO_MASTER_URL=ws://127.0.0.1:7878/ws cargo run -p episco-presenter
```

## Sync protocol

JSON text frames on `ws://<master>:7878/ws`. Defined in
`crates/episco-sync/src/protocol.rs`.

1. The client sends `{"type":"hello","protocol_version":1,"client_name":"…","role":"mirror|remote|monitor","token":…}` within 5 s.
2. The server answers `welcome` with the full state and service, or `error` and
   closes the connection (wrong version or token).
3. The server pushes `state` on every change and `service` when a new service
   loads.
4. The client may send `command` (any `episco_core::Command`, e.g.
   `{"type":"command","command":{"type":"next"}}`). `monitor` clients cannot send commands.
5. The client sends a `ping` every 2 s and the server replies `pong`. The client
   reports the round-trip time and reconnects (backoff 250 ms → 5 s) if it hears
   nothing for 6 s.

Latency: `TCP_NODELAY` is set on both ends and each update is serialised once
for all clients. In release builds over loopback, an update reaches a client in
about 5 µs (`cargo test --release -p episco-sync -- --nocapture rapid`). On a
real LAN the network round trip dominates, typically well under a millisecond on
wired Ethernet.

A command pressed on a slave or tablet while it is disconnected is discarded,
not replayed later, so a stale "next" can never fire after reconnecting.

`GET /health` returns `{server_name, protocol_version, revision, subscribers}`.

## Tauri surface

Events (emitted to every window): `episco://state`, `episco://service`,
`episco://link`.

Commands:

| Area | Commands |
| --- | --- |
| Presentation | `get_node_config`, `get_snapshot`, `send_command` |
| Vocabulary / text | `vocabulary`, `parse_ewm`, `format_ewm` |
| Service | `new_service`, `open_service`, `load_example_service`, `save_service` |
| Items | `add_items`, `new_item`, `update_cue_document`, `set_cue_overrides`, `set_announcement_mode`, `move_cue`, `remove_cue`, `save_cue` |
| Looks | `set_cue_look`, `set_default_look`, `set_service_details`, `list_backgrounds`, `import_backgrounds`, `pick_background` |
| Library | `library_list`, `library_preview`, `library_add`, `library_save_cue`, `library_delete`, `library_folder`, `parse_propers`, `add_propers` |
| Services | `liturgical_info`, `services_list`, `service_create`, `services_schedule`, `service_delete`, `service_keep` (autosave runs in `autosave.rs`) |
| Dialogs | `confirm`, `pick_image` |

All edits go through `SyncHub::edit_service`, which broadcasts the new
service to every window and node and keeps the on-air slide attached to its
cue (by id) through reorders and edits. On a slave node the edit commands
return an error and the UI is read-only.
