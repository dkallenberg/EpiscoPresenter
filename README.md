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
| 5. Scripts and bulletins | **Planner app** (EpiscoPlanner) with a shared parish library folder: bulletin and officiant script previews, exported to Word (book fold) ([planner](docs/planner.md)) |
| 6. Tablet remote | The protocol is ready (`remote` client role); web app not started |
| Services & autosave | Services screen at start-up, scheduling about a month of Sundays ahead with church-year names and the day's propers, Rite I or Rite II, autosave ([services](docs/services.md)) |
| Library & propers | Built-in Rite I and Rite II texts, your saved items, a media library (images, videos); the lectionary (Years A–C, Tracks 1 & 2, holy days) with BCP collects (both rites), Psalter and canticles; NRSV/NRSVue reading texts pasted from a licensed source ([library](docs/library.md)) |
| Events, slides and video | Events that are not services; a slide maker (title and information slides, text or pictures); presentations from PowerPoint, Keynote (as PDF) or Google Slides; YouTube, Facebook and Instagram videos ([slides and video](docs/slides-and-video.md)) |
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

Run the presentation app:

```bash
cargo run -p episco-presenter
```

Run **EpiscoPlanner**, the planning tool (no output screens or live controls;
services and the library in a shared folder; see [docs/planner.md](docs/planner.md)):

```bash
cargo run -p episco-presenter --bin episco-planner
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

and for the planner's own installer (its own name and app identity):

```bash
cargo tauri build --config src-tauri/planner/bundle.json
```

### The parish library in Google Drive

The apps can use a shared Google Drive folder directly (each person signs
in with Google; no Google Drive for desktop needed). The parish sets up a
free Google Cloud project once: see [docs/google-drive.md](docs/google-drive.md).

### Installers from GitHub (Windows and Mac)

GitHub can build the installers for both apps without a Windows PC: on the
repository's **Actions** tab choose **Build installers → Run workflow**.
After about 20 minutes the run's page lists, under **Artifacts**,
`windows-installers` (an `.msi` and a setup `.exe` for each app) and
`mac-installers` (a `.dmg` for each app, for Apple silicon and Intel Macs).
Pushing a version tag (`v0.2.0`) builds them too.

**Releases and betas.** To publish a version, type it in the **Release** box
when running **Build installers** (or push the tag): `v0.2.0-beta.1` makes a
GitHub release marked *Pre-release* (a beta), `v0.2.0` a full release, each
with the installers attached and kept (Actions downloads expire after 90
days). Set the same version in `Cargo.toml` and both `tauri.conf.json` files
first.

The Windows installers are not signed: the first time, Windows SmartScreen
says **"Windows protected your PC"**; choose **More info → Run anyway**.
Windows 10 and 11 have the WebView2 runtime the app needs (the installer
fetches it if it is missing).

### Updates

Both apps look for a newer version of themselves about 10 seconds after
they start. If there is one they download it in the background and the top
bar says **"EpiscoPresenter 0.2.1 is ready · Restart to update"**; nothing
is installed until someone clicks it (**Later** keeps working and asks again
next time), so an update never interrupts a service.

Updates come from the GitHub releases (betas too) and are signed with the
parish's own key, which the apps check before installing. To set it up once:

1. On a computer with the Tauri CLI, make the key pair (a password is
   optional):

   ```bash
   cargo tauri signer generate -w ~/.tauri/episcopresenter.key
   ```

2. In the repository on GitHub: **Settings → Secrets and variables →
   Actions**.
   * **Secrets → New repository secret**: `TAURI_SIGNING_PRIVATE_KEY`, the
     contents of `~/.tauri/episcopresenter.key` (and
     `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if you gave it a password).
   * **Variables → New repository variable**: `TAURI_SIGNING_PUBLIC_KEY`,
     the contents of `~/.tauri/episcopresenter.key.pub`.
3. Keep a copy of the private key somewhere safe: without it, later versions
   cannot be signed for the apps already installed.

Then for each new version: raise the version in `Cargo.toml` and both
`tauri.conf.json` files (an update must have a higher number, e.g. 0.2.1),
and run **Build installers** with that version in **Release**
(`v0.2.1-beta.1` or `v0.2.1`). The release also updates the `updates`
release, where the apps look. Versions built before this (0.2.0 and
earlier) do not look for updates: install the first version with updates by
hand once.

### Installing on another Mac

The app is not signed with an Apple Developer ID, so on a Mac it was not
built on, macOS may say it **"is damaged and can't be opened"** (it was
copied from a download, AirDrop, email or a cloud drive and is
quarantined). It is not damaged. On that Mac, after dragging it into
Applications, run once in Terminal:

```bash
xattr -dr com.apple.quarantine /Applications/EpiscoPresenter.app
```

(and `/Applications/EpiscoPlanner.app` for the planner). Builds sign the app
ad hoc (`signingIdentity: "-"`), so with a new build macOS may instead say it
is from an unidentified developer: open it once with Control-click → Open,
or System Settings → Privacy & Security → **Open Anyway**.

A build runs on the kind of Mac it was made on. To make one for both Apple
silicon and Intel Macs:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

```bash
cargo tauri build --target universal-apple-darwin
```

To install without any warning, the app must be signed with an Apple
Developer ID and notarized (an Apple Developer Program membership); Tauri
does this when `APPLE_SIGNING_IDENTITY` and the notarization credentials
are set when building.

## Quick check of a liturgy file

```bash
cargo run -p episco-core --example ewm_dump -- examples/liturgies/sursum-corda.ewm
```
