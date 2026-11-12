# EpiscoPlanner

EpiscoPlanner is the planning tool: the rector, administrator, musician and
technician plan services on their own computers (Mac or Windows) while the
presentation computer runs them. It is built from the same code as the
presentation app, so it has the same Services screen, timeline, editor,
library, propers, templates and slide layouts, but no output screens, live
controls or network sync. Its main work is the **bulletin** and the
**officiant's script**; the slides are a tab away.

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

## Bulletin and script

The planner opens on the **Bulletin** tab: a live preview of the printed
bulletin, half-letter pages in Garamond, updated as the service changes.

* **Click an item** in the preview (or the timeline) and choose how it
  appears with the bar above:
  * **Reference line**: title, opening words and the page or hymn number,
    e.g. *Collect for Purity — Almighty God, to you all hearts are open… BCP 355*.
  * **Full text**: every word, speaker labels, the people's parts in bold.
  * **Title only**, or **Left out**.

  The usual choice for each item is underlined: Prayer Book texts (a `BCP`
  citation) as reference lines, hymns and anthems as title and number, the
  sermon and announcements as titles, the readings, psalm, collect and
  prayers in full, pictures and planned blanks left out. Choosing the usual
  one again clears your choice. The choice is saved in the service plan
  (`bulletin = "full"` on the cue).
* **Service time** goes on the cover: *November 9, 2025 at 8 o'clock in the
  morning*. The cover also names the Sunday from the date.
* **Parish details…**: name, place, cover title, logo (PNG or JPEG),
  address, phone, website, email, the music licence notice and a welcome
  for visitors. They print on the cover and back page and are kept in
  `parish.json` in the library folder, so everyone shares them (a logo from
  elsewhere is copied in as `parish-logo.png`).
* **Export bulletin to Word** saves a `.docx` (suggested in the library's
  `bulletins/` folder, named like `2025-11-09 0800 Bulletin.docx`) and opens
  it. It is set up for **book fold**: Word shows half-letter pages and, on
  a two-sided printer, prints them in booklet order on letter paper to fold
  in half. (LibreOffice and Google Docs ignore book fold and show the
  pages as landscape sheets.) Edit the Word file freely for last touches.

The **Script** tab is the officiant's copy: every word of every item, the
rubrics in red, large type on letter paper, the Great Thanksgiving starting
a new page. **Export script to Word** works the same way.

The preview and the Word files are built from one description of the
document (`episco-core`'s `bulletin` module, written out by its `docx`
module), so they always match.

## Running and installing

* Development: `cargo run -p episco-presenter --bin episco-planner`.
* Installer: `cargo tauri build --config src-tauri/planner/bundle.json`
  (builds "EpiscoPlanner" with its own app identity, so it installs beside
  the presentation app).

## Coming next

1. Working together safely: live refresh when someone else saves, who is
   editing what, and a side-by-side choice instead of conflict copies.
2. Bulletin styles per item type (Prelude, Anthem with composer, Homily
   with preacher, one-line instructions) and the two psalm styles.
3. A Sunday with two services (8:00 Rite I, 10:30 Rite II) sharing propers.
4. The web bulletin.
5. The presentation app reading services from the shared folder.
