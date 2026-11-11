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
| 2. Operator control interface | **Done**: service timeline, slide grid, interactive block editor, live previews, quick actions ([operator guide](docs/operator-guide.md)) |
| 3. Output engine | **Mostly done**: per-cue backgrounds and layouts on both outputs, built-in stained-glass designs, lower thirds on a key colour, text fitted to its area ([backgrounds](docs/backgrounds.md)) |
| 4. Google Slides | **Done**: the deck is downloaded as a PDF and shown as slide pictures (pdf.js): Pre-Service loop, single-pass stepping in any order, loop mode ([operator guide](docs/operator-guide.md#announcements-google-slides)) |
| 5. Scripts and bulletins | Not started |
| 6. Tablet remote | The protocol is ready (`remote` client role); web app not started |
| Services & autosave | Services screen at start-up, scheduling about a month of Sundays ahead with church-year names and the day's propers, Rite I or Rite II, autosave ([services](docs/services.md)) |
| Library & propers | Built-in Rite I and Rite II texts, your saved items, a media library (images, videos); the lectionary (Years A–C, Tracks 1 & 2, holy days) with BCP collects (both rites), Psalter and canticles; NRSV/NRSVue reading texts pasted from a licensed source ([library](docs/library.md)) |
| Slide layouts | Layout tool: place every piece of each slide type and set its font, size and colour ([layouts](docs/layouts.md)) |

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

Once it opens, the Services screen offers **Schedule upcoming Sundays** to plan
about a month ahead, or **Example** for a sample service. Open one and press →
(or Next) to step through it. See the [operator guide](docs/operator-guide.md).

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
