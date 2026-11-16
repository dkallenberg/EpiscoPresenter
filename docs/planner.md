# EpiscoPlanner

EpiscoPlanner is the planning tool: the rector, administrator, musician and
technician plan services on their own computers (Mac or Windows) while the
presentation computer runs them. It is built from the same code as the
presentation app, so it has the same Services screen, timeline, editor,
library, propers, templates and slide layouts, but no output screens, live
controls or network sync. Its main work is the **bulletin** and the
**officiant's script**; the slides are a tab away.

The cue sheet on the left is the order of service: the items, their type,
and the unsaved-changes dot. The slide planning (background thumbnails,
slide counts, the pre-service deck loop) is left to the presentation app.

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
  bible/        the parish's Bible text files (NRSVue.json, NRSV.json)
  bulletins/    exported bulletins and scripts
  layouts.json  slide layouts (Layout tool)
  parish.json   parish details for bulletins
```

**Google Drive without Drive for desktop.** The apps can also reach the
shared folder in Google Drive themselves: each person signs in with their
Google account and picks the folder, and the app keeps a full copy of it on
the computer. See [google-drive.md](google-drive.md) for the one-time Google
setup and the steps on each computer.

An empty folder is set up the first time it is chosen. Change the folder or
your name at any time with the **📁** button in the top bar; **This computer
only** keeps everything in the app's own folder instead. If the shared folder
cannot be found (for example Google Drive is not running), the button turns
gold and the planner works from this computer's folder until it is back.

On the presentation computer, set the shared folder to **Available offline**
in Google Drive, so services are on the computer before Sunday.

**Each computer's own copy.** Both apps keep a copy of the parish library
on the computer (library items and hymns, media, backgrounds, the Bible
file, slide layouts and parish details) and work from it, so the Library
opens quickly and keeps working when Google Drive is not. The shared folder
holds a **change log** for each computer (`change-log/<computer>.jsonl`):
a line for every file it added, changed or removed. At start-up, every
15 seconds, and a couple of seconds after anything in the library is saved,
each app reads what the other computers have logged since it
last looked and copies just those files (or removes them), then sends its
own changes to the shared folder and logs them. A file whose log line
arrives before the file itself (Google Drive can be slow) waits until it is
there. The first time, or with a new shared folder, the copy and the shared
folder are compared file by file instead: a file on one side only goes to
the other, and where both have it the newer wins (nothing is removed).
Google Drive's conflict copies (`name (1).ewm` beside `name.ewm`, made when
two computers change a file at once) are left out.

The Library shows how the copy stands under its list: **✓ In step with the
other computers** and when it last checked, **⇣ changes still downloading**
while Google Drive brings another computer's files, or **⚠** and why when
the shared folder cannot be reached.

**Copies of the same item.** An item in the library more than once (the
same hymn number, or the same kind and title) is marked **×2** in the list.
Select it to see the others and click **Keep this one** to remove the rest:
services keep their own words, so nothing else changes.
Until EpiscoPlanner's first copy is made it works from the shared folder.
Services and templates are not copied: they stay in the shared folder
(EpiscoPresenter takes its own copy of a service when it is Ready). The
copy is in the app's local data folder (on macOS
`~/Library/Application Support/org.episcopresenter.planner/parish-library/`;
EpiscoPresenter's is its own data folder).

**Speed.** Google Drive for desktop normally *streams* files: it downloads a
file's contents the first time it is read, which makes a folder of many
small files slow to open. On every computer that uses the shared folder,
right-click the folder in Finder (or Explorer) and choose **Google Drive →
Available offline** (or set Drive to *Mirror files* in its preferences), so
the files are already on the computer. The apps also remember the list of
services, templates and library items on each computer (in the app's cache
folder, never in the shared folder) and read a file again only when its
date or size has changed, so after the first start the Services screen and
the Library open without reading the files' contents.

The choice is kept per computer in `library.json` in the app's settings
folder (on macOS `~/Library/Application Support/org.episcopresenter.planner/`).

## Bulletin and script

The planner opens on the **Bulletin** tab: a live preview of the printed
bulletin in Garamond, updated as the service changes. The bar above it has
the **Service time**, the **Paper** (**11 × 17, folded** into 8½ × 11 pages,
the default; 8½ × 14 (legal), folded into 7 × 8½ pages; or 8½ × 11, folded
into 5½ × 8½ pages; also in Parish details),
**B** / **I**, **Reading texts…**, **Parish details…** and **Export to Word**.

**To do**, beside the page, lists what the service still needs, and each
line opens the place to do it: the parish's name and details, the service
time, the announcements' Google Slides link, a reading's text, a hymn not
yet chosen, the preacher's name. "Nothing left to do ✓" when it is ready.

* **Click an item** in the preview (or the service list) and choose how it
  appears in the bar that appears just above it:
  * **Reference line**: title, opening words and the page or hymn number,
    e.g. *Collect for Purity — Almighty God, to you all hearts are open… BCP 355*.
  * **Full text**: every word, speaker labels, the people's parts in bold.
  * **Responses only**: the title and just what the people say, each with
    the short line that cues it (*The Word of the Lord. / Thanks be to
    God.*); a long prayer is left out and only its *Amen* is printed.
  * **Title only**, or **Left out**.

  The usual choice for each item is underlined: Prayer Book texts (a `BCP`
  citation) as reference lines, hymns and anthems as title and number, the
  sermon and announcements as titles, the readings, psalm, collect and
  prayers in full, pictures and planned blanks left out. Choosing the usual
  one again clears your choice. The choice is saved in the service plan
  (`bulletin = "full"` on the cue).
* **Readings on the slides** show just the title and the responses (*The
  Word of the Lord. / Thanks be to God.*); the congregation reads along in
  the bulletin, which prints the text in full. To project the text too,
  tick **Show the reading on the slides** in the reading's panel in the
  Editor.
* **Reading texts…**: the app does not include scripture (the NRSV and
  NRSVue are copyright the National Council of Churches), so readings get
  their text in one of two ways:
  * **A Bible file.** If the parish has the translation as a JSON file
    (books → chapters → verses), click **Import Bible file…** once. It is
    checked and copied into the library folder (`bible/NRSVue.json`), so
    every computer has it. From then on every reading fills in from its
    citation by itself: when the propers are added from the lectionary,
    and when a service is created. **Fill missing readings** does it for a
    service that already exists, and **Refill** redoes one reading.
    Citations are read as the lectionary writes them: ranges and lists
    (*Isaiah 61:1-4, 8-11*), across chapters (*Genesis 1:1—2:4a*), optional
    verses in brackets (included), one-chapter books (*Philemon 1-21*),
    the first of several choices (*Acts 2:1-21 (or Ezekiel 37:1-14)*), and
    part-verses (*27a*, *9b*), which are cut at the clause nearest the
    middle of the verse, so check those by eye. Every reading in the
    lectionary (808 citations) has been checked against an NRSVue file.
  * **Paste.** Without a Bible file, **Open ↗** shows the passage in your
    browser (Bible Gateway for the NRSVue, Oremus for the NRSV); copy it,
    paste it and click **Use pasted**. Verse numbers, headings, footnotes
    and the copyright line are removed.

  Either way the text appears in the bulletin (in its paragraphs), the
  script and on the slides (about 40 words each). Until then a reading in full text shows a yellow
  *[The text of … goes here]*, also highlighted in the Word file, and the
  button turns gold and counts the readings still to add. Clicking a
  placeholder opens the dialog at that reading.

  Keep the Bible file in the parish library folder only: do not share it
  beyond what the parish's use of the translation allows.
* **Service time** goes on the cover: *November 9, 2025 at 8 o'clock in the
  morning*. The cover also names the Sunday from the date.
* **Speaker labels** (*People*, *All*, *Celebrant*…) print once at the
  start of each block by the same speaker, in the bulletin and the script:
  the Creed's paragraphs share one *All*, and a rubric in the middle does
  not repeat the label. **Parish details…** can switch this to every
  paragraph.
* **Prayers split across slides** print as one paragraph: when the same
  speaker carries on after a slide break, the bulletin and script run the
  text on (the slides keep their own breaks).
* **The prelude**: the first row of the service list, **♪ Prelude**, and
  the first line of the bulletin (*Add the title and composer* until it is
  filled in; that reminder is not printed). Click either for its title and
  composer (with any arranger: *Johann Sebastian Bach, arr. by Homer
  Whitfield*); they are also in **⚙ Settings → Service settings**. It heads
  the bulletin and script (*Prelude — Adagio — Johann Sebastian Bach…*) and
  shows on the live stream's welcome before the service.
* **The postlude**: the last row of the service list, **♪ Postlude**, and
  the last line of the bulletin and script, set the same way (and in
  Service settings). It is not on the live stream.
* **Shortening or leaving them out**: the Prelude and Postlude boxes have
  **In the bulletin**: *Title and composer* (the usual), *Name only* (just
  "Prelude", e.g. when the piece is not chosen) or *Left out*. A piece left
  out keeps a faint line in the Bulletin tab (not printed) to bring it back,
  the service list says so, and the script still names it. Until a piece
  the bulletin shows is named, To do lists *Name the prelude* or *Name the
  postlude*.
* **Choir anthems**: **+ Add → Choir anthem…** asks for its name in the
  service (*Offertory Anthem* to start with), its title and composer. The
  bulletin lists it like a hymn (*Offertory Anthem — Ave verum corpus — W.
  A. Mozart*), and on the live stream it is a lower third with the name
  above and the title and composer in the bar. Change the title and
  composer in the Editor (**Anthem title**, **Composer**); until it has
  them, To do says *Name the anthem*.
* **The sermon**: in the Editor, the sermon item has the preacher's title
  (*The Rev.*, *The Very Rev.*, *The Rt. Rev.* … or your own), name and
  position. **Preacher** offers the Rector, Assistant and Bishop from Parish
  details (or someone else); the bulletin prints *The Sermon — The Very Rev.
  Jane Doe, Rector*, and the slide shows the same.
* **Service times**: Parish details holds the Sunday service times (e.g.
  8:00, 10:30). Schedule services offers them for one service, and makes a
  service at each time that is ticked for the coming Sundays, and every service card shows its
  time.
* **Day and night**: ☀ / ☾ in the top bar switches the planner's colours
  (at first it follows the computer's setting). EpiscoPresenter is always
  dark.
* **Parish details…**: name, place, cover title, logo (PNG or JPEG),
  address, phone, website, email, the music licence notice, the service
  times, the clergy (Rector, Assistant, Bishop: title, name and position)
  and a welcome
  for visitors. They print on the cover and back page and are kept in
  `parish.json` in the library folder, so everyone shares them (a logo from
  elsewhere is copied in as `parish-logo.png`).
* **Export bulletin to Word** saves a `.docx` (suggested in the library's
  `bulletins/` folder, named like `2025-11-09 0800 Bulletin.docx`) and opens
  it. It is set up for **book fold**: Word shows the folded pages and, on
  a two-sided printer, prints them in booklet order on 11 × 17, legal or
  letter paper to fold in half. The service starts on the first page,
  under the title, in Garamond 12 pt with 0.5 in margins. (LibreOffice and Google Docs ignore book fold and show the
  pages as landscape sheets.) Edit the Word file freely for last touches.

**Edit the text in place**, like a word processor, in both the Bulletin
and the Script: click into any line and type. **Enter** starts a new line,
**Backspace** at the start of a line joins it to the one before,
**Ctrl/⌘ B** and **Ctrl/⌘ I** (or the **B** and **I** buttons) make bold
and italic, and **Esc** undoes the changes to the line you are in. Click an
item's title to rename it. Changes are saved as you go, into the item
itself, so the slides change with them. Text the app makes up (opening
words, page numbers, the cover's date) changes with the service, the
parish details or the item's style instead.

The **Script** tab is the officiant's copy: every word of every item, the
rubrics in red, large type on letter paper, the Great Thanksgiving starting
a new page. **Export script to Word** works the same way.

The preview and the Word files are built from one description of the
document (`episco-core`'s `bulletin` module, written out by its `docx`
module), so they always match.

## Ready for Sunday: sending services to the presentation computer

EpiscoPresenter never works from the shared folder. In the planner's
Services screen, **Mark Ready** on a service when it is ready to go to the
presentation computer (the button turns green: *✓ Ready*). On the
presentation computer:

1. Click **📁** in EpiscoPresenter's top bar once and choose the same parish
   library folder (set it to *Available offline* in Google Drive).
2. Within a minute the service is copied to that computer, with its texts,
   the pictures and videos it uses, and the slide layouts. It is listed under
   **From the planner** on the Services screen (*✓ On this computer*) and in
   the usual list, and runs from that copy.
3. Changes made in the planner afterwards are copied over automatically,
   unless the service is open on the presentation computer or was changed
   there. Then it shows **Updated in the planner** (or **Changed in both**),
   and **⟳ Updated in the planner** appears in the top bar while it is open:
   click it (or **Refresh**) when it is safe to take the planner's version.

A Drive outage, a sync conflict or a late edit in the planner cannot change
what is on the screens during a service. Taking a service back (click
*✓ Ready* again) stops further updates; the copy already on the presentation
computer stays.

## Running and installing

* Development: `cargo run -p episco-presenter --bin episco-planner`.
* Installer: `cargo tauri build --config src-tauri/planner/bundle.json`
  (builds "EpiscoPlanner" with its own app identity, so it installs beside
  the presentation app).

## Open on another computer

When a planner opens a service (or a template) from the shared folder, the
app marks it as open, with the name set with the library folder, in the
shared folder's `in-use/` folder (one small file per service, kept apart
from the service so the presentation computer does not see a change). The
mark is confirmed every 20 seconds and removed on closing; one not
confirmed for 3 minutes (a computer that crashed or went to sleep) no
longer counts.

* The **Services** screen shows **Open on Rector's computer** on a service
  someone else has open.
* Opening it gives **view only**: everything can be read, nothing can be
  changed (and nothing is saved). The top bar says **Open on Rector's
  computer · Take over**.
* **Take over** reloads the service as last saved and makes it yours. On
  the other computer it turns view only within about 20 seconds (plus
  however long Google Drive takes), with a note of who took it over; what
  they changed before that is saved.

## Coming next

1. Working together safely: live refresh when someone else saves, and a
   side-by-side choice instead of conflict copies.
2. Bulletin styles per item type (Prelude, Anthem with composer, Homily
   with preacher, one-line instructions) and the two psalm styles.
3. A Sunday with two services (8:00 Rite I, 10:30 Rite II) sharing propers.
4. The web bulletin.
5. ~~The presentation app reading services from the shared folder~~: done
   (Ready for Sunday, above).
