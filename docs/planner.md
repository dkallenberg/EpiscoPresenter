# EpiscoPlanner

EpiscoPlanner is the planning tool: the rector, administrator, musician and
technician plan services on their own computers (Mac or Windows) while the
presentation computer runs them. It is built from the same code as the
presentation app, so it has the same Services screen, timeline, editor,
library, propers, templates and slide layouts, but no output screens, live
controls or network sync. Scripts and bulletins will be made from it.

## The parish library folder

On first run the planner asks for the **parish library folder** and **your
name** (shown to the others as who changed what). Use a folder everyone can
reach, for example a shared Google Drive folder synced to each computer with
Google Drive for desktop. Everything is kept there:

```text
Parish Library/
  services/     one folder per service (service.toml + its texts)
  templates/    service order templates
  library/      saved items and imported hymns
  media/        pictures and videos
  backgrounds/  slide backgrounds
  layouts.json  slide layouts (Layout tool)
```

An empty folder is set up the first time it is chosen. Change the folder or
your name at any time with the **📁** button in the top bar; **This computer
only** keeps everything in the app's own folder instead. If the shared folder
cannot be found (for example Google Drive is not running), the button turns
gold and the planner works from this computer's folder until it is back.

On the presentation computer, set the shared folder to **Available offline**
in Google Drive, so services are on the computer before Sunday.

The choice is kept per computer in `library.json` in the app's settings
folder (on macOS `~/Library/Application Support/org.episcopresenter.planner/`).

## Running and installing

* Development: `cargo run -p episco-presenter --bin episco-planner`.
* Installer: `cargo tauri build --config src-tauri/planner/bundle.json`
  (builds "EpiscoPlanner" with its own app identity, so it installs beside
  the presentation app).

## Coming next

1. Working together safely: live refresh when someone else saves, who is
   editing what, and a side-by-side choice instead of conflict copies.
2. Bulletins (Word, book-fold) and officiant scripts.
3. A Sunday with two services (8:00 Rite I, 10:30 Rite II) sharing propers.
4. The web bulletin.
5. The presentation app reading services from the shared folder.
