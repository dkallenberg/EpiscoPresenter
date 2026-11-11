# EpiscoPresenter

A desktop worship presentation engine for Episcopal services, built with Rust and Tauri 2.
It drives the sanctuary display and the live-stream overlay, keeps two machines in sync
over the LAN, and builds everything from one plain-text liturgy format
([`.ewm`](docs/ewm-format.md)).

## Status

| Spec section | State |
| --- | --- |
| 1. Hardware architecture and network sync | **Done**: master, slave and standalone roles; WebSocket sync; per-monitor output windows |
| 7. `.ewm` data format and parser | **Done**: parser, writer (round-trips), service plans, data model |
| 2. Operator control interface | Scaffold: cue list, slide grid, live previews, quick actions, `.ewm` editor with role-coloured block view |
| 3. Output engine | Basic renderers following the typography and zone rules; auto-fit text and background images still to do |
| 4. Google Slides | Data model only (`announcements` cues, pre-service state) |
| 5. Scripts and bulletins | Not started |
| 6. Tablet remote | The protocol is ready (`remote` client role); web app not started |

## Layout

```
crates/episco-core   data model, .ewm parser/writer, service plans, state machine
crates/episco-sync   node config, master WebSocket server, slave client
src-tauri            desktop app (Tauri 2)
ui/                  HTML/CSS/JS front-end (no build step)
examples/            sample .ewm liturgies and a service plan
docs/                format spec and architecture notes
```

## Building

You need Rust 1.77 or later.

* **macOS:** install the Xcode command-line tools.
* **Windows:** install the WebView2 runtime.
* **Linux (development only):** install `libwebkit2gtk-4.1-dev` and `libgtk-3-dev`.

Run the library tests (no GUI toolchain needed):

```bash
cargo test
```

Run the desktop app:

```bash
cargo run -p episco-presenter
```

To try a master and a slave side by side, see [docs/architecture.md](docs/architecture.md#nodes).
To build a distributable app bundle, use the Tauri CLI:

```bash
cargo install tauri-cli --version "^2"
```

```bash
cargo tauri build
```

## Quick check of a liturgy file

```bash
cargo run -p episco-core --example ewm_dump -- examples/liturgies/sursum-corda.ewm
```
