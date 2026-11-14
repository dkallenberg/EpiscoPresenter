# Operator guide

The control window has three panes: the **service timeline** (left), the
**slides / editor** (center) and the **live previews and stage controls** (right).

## Top bar

| Shows | Does |
| --- | --- |
| ☰ Services | The Services screen: upcoming services, **Schedule services…** (one service, or the coming Sundays), Open file…, Example. It also opens when the app starts. See [services.md](services.md). |
| The service | Its title, date and time. |
| Save state | **Saved ✓** or **Saving…**, since services save themselves. For the example (or a copy), **Not saved yet · Keep this service** adds it to your services. |
| 🖥 and 📡 (screen icons, right end) | One icon for each screen this computer drives: the monitor for the sanctuary screen and the broadcast icon for the live stream (one computer shows both; with two computers, each shows its own). **Red** while that screen is closed, **green** while it is open; hover for where it is. Click for the **Output screens** card: open, close or move them. (EpiscoPresenter only.) |
| ⚙ (gear icon, right end) | Settings: everything not needed every week, in one menu (below). |

**⚙ Settings** (the gear at the right end of the top bar):

| Group | Entry | Does |
| --- | --- | --- |
| This service | Service settings… | Title, date, season, colour, rite (I or II), lectionary track, Bible version for reading texts, the announcements deck (Google Slides link and its number of slides) and the stream key colour. |
| | Save a copy… | Writes a copy of the service elsewhere. |
| Screens | Output screens… | Opens the output screens, which do not open by themselves: for the Sanctuary (and Live Stream), **Full screen** on a monitor (the second monitor unless you choose another), **Window** for an ordinary window, or **Close**. It shows how many are open. In an output, Esc leaves full screen; a double-click toggles it. On the Mac a full-screen output is on every desktop (Space) of its monitor and above other windows, so switching desktops (a swipe, Mission Control, Ctrl-arrow) leaves it showing. |
| | Slide layouts… | Where text, titles and music go on each kind of slide, their fonts, and how much text a slide holds. See [layouts.md](layouts.md). |
| | Connection to the other computer… | The sanctuary and live-stream computers (below). |
| Parish | Parish library folder… | The shared parish library folder. Marked ⚠ (and the ⚙ button too) when it cannot be found. |
| | Parish details… | (EpiscoPlanner) Name, clergy, service times and bulletin options. |
| This computer | Day / Night colours | (EpiscoPlanner) Light or dark screens. |

The right end shows the connection in plain words: **One computer**,
**Live stream connected ✓** / **Waiting for the live-stream computer** (on
the sanctuary computer), or **Following … ✓** / **Not connected to the
sanctuary computer** (on the live-stream computer). Hover for the address
and delay; click to set up the connection. A stage message, when one is
active, is shown here on every computer.

## Getting started

Until it is done (or hidden), the Services screen shows **Getting started**:
choose the parish library folder, then in EpiscoPresenter choose the screens
and, if there are two computers, connect them (or **One computer is
enough**); in EpiscoPlanner add the parish's details and plan the coming
Sundays. Each step has a **Do it…** button and is ticked when done.

When something goes wrong, the message says what to try next (for example,
"Check that the folder can be reached (is Google Drive running?)"), with the
technical detail at the end for whoever helps.

## Service timeline (left)

* The first row is **Before the service**: the announcements loop that runs
  until the first item is shown. It is marked **On screen** while it runs.
* Each row shows the item's number, an icon for its kind and its name, with
  **On screen 2/5** (which slide) while it is shown and **●** while it has
  unsaved changes. Hover a row for the rest: its kind and number of slides,
  its caption, and its backgrounds on both screens.
* **Click** selects a cue (for the editor and slide grid), **double-click** puts
  it on air, **drag** a row up or down to reorder (a gold line shows where it will go). Hover a row for ↑ ↓ ✕.
* **+ Add** inserts after the selected cue: an item from the **Library**,
  **Import propers** (readings and collect pasted from the lectionary page),
  existing `.ewm` files (several at once), or a new item from a template.
  See [library.md](library.md).

## Slides tab

Every slide of the service, grouped by cue and numbered. The **Sanctuary / Live
stream** switch at the top right picks which output the thumbnails show. Click
a slide to put it on air. The live slide has a red frame.

## Editor tab

Edits the selected item. Changes reach every output about a third of a second
after you stop typing.

The item's **words** come first, then its **title** (and citation or hymn
number), any panel for its kind (psalm, reading, Gospel music, hymn,
preacher), and **Appearance** last. **Name in the service list** is saved in the service plan instead of
the item (e.g. "Processional Hymn" for *Lift Up Your Heads*). **More…** under
them holds the item's type, season and colour, its live-stream caption, and
**Edit the file (.ewm)…** (the Source view, for experienced users).

**Appearance** (closed at first; the two coloured chips show the
backgrounds) has a card for each screen, with a live preview and one
**Look**:

| Sanctuary | Live stream |
| --- | --- |
| As usual for the item's type | As usual for the item's type |
| Stained glass, light / colour / charcoal | Text on the left, camera on the right |
| Black, as hymn lyrics | Wide text on the left (hymns) |
| Blank: a black screen | Bar across the bottom |
| | Lower third over the camera |
| | Full screen, as the sanctuary |
| | Camera only: nothing shown |

**More…** in each card has the details:

* **Background**: opens a picker with the built-in designs (drawn in the
  liturgical colour), the key colour, black, and your library images. **Choose
  image file…** uses any image where it is; **Import to library…** copies
  images into the `backgrounds/` folder beside the service plan.
* **Layout**, **Text** and **Fit**. *Auto* shows what the item gets from its
  type.
* **Use for all … items**: makes this look the service default for every item
  of the same type (for example, every Collect), so you set each background
  once.
* **Reset**: returns the item to its type's default. A **changed** badge marks
  an item with its own look.

See [backgrounds.md](backgrounds.md) for the layouts and their text areas.

* **Rows** are blocks. The dropdown sets the speaker role, which sets the row's
  colour and the on-screen style (People is bold black; Rubric is red italic
  and never projected).
* Hover between rows for **( + ADD BLOCK )**. The new block continues the
  dialogue (after Priest comes People, after People the previous speaker). The
  small buttons beside it add a **§ Heading** (starts a new slide with a
  title), a **⎯ Slide break**, or a **♪ Music** snippet (an image).
* A slide break's menu makes it a break on **Both screens**, the
  **Sanctuary only** or the **Live stream only**.
* Each line in a row is one line on screen. Press Enter for a new line,
  **Tab** / **⇧Tab** to indent or outdent it.
* Hover a row for ↑ ↓ ✕.
* **On-air protection:** while the item you are editing is on air, edits are
  held, and a red banner offers *Apply to screens* or *Discard*. Untick **Hold
  on-air edits** to apply them as you type.

**Undo** (↶, ⌘Z) and **Redo** (↷, ⇧⌘Z; Ctrl on Windows, and Ctrl+Y)
take back the last change to the open service and put it back again: text,
order, looks, settings, bulletin choices. Changes coming in quick succession
(typing) are one step; up to 100 steps are kept while the service is open.
In a text box (a title, a search) ⌘Z is the box's own undo, as is ⌘Z while
typing a bulletin line; Esc there undoes the line.

| Key (in a text row) | Does |
| --- | --- |
| ⌘B / ⌘I | Bold / italic |
| ⌘↩ | New block after this one |
| ⌫ in an empty row | Delete the row |
| ⌘S / ⇧⌘S | Save item / Save item as… |
| ⌥⌘S | Save service |

## Announcements (Google Slides)

Paste the deck's link in **⚙ Settings → Service settings → Announcements deck**: the ordinary
sharing link (Share → Copy link) works. Share the deck as **Anyone with the
link can view**.

When a service with a deck opens, each computer downloads the deck once (as
a PDF, from Google) and shows its slides as pictures:

* **Before the service** is the first row of the service list, and a
  section at the top of the **Slides** tab with every slide; the one on
  screen is outlined. It repeats on the sanctuary screen, a slide every 9
  seconds. Click its heading or a slide (or the **Before the service**
  button) to go back to it.
* An **Announcements** item set to **One time through** has one card per
  slide: **Next** / **Prev** step through them, and you can click any slide,
  in any order, to show it. Set to **Repeating**, it plays like Before the
  service.
* The number of slides is found automatically. After changing the deck in
  Google Slides, click **↻ Refresh slides** (Service settings, or the Announcements
  item's Editor) to download it again.
* The slides are shown as still pictures (animations and embedded videos in
  the deck do not play). If the deck cannot be downloaded (for example a
  "Publish to web" link, or no internet), the screens fall back to Google's
  own player, which plays the deck by itself.

## Slides, presentations and web video

**+ Add** also offers **New slides (slide maker)**, **Slides from a PDF**
(PowerPoint, Keynote), **Google Slides presentation…** and **Web video**
(YouTube, Facebook, Instagram). See [slides-and-video.md](slides-and-video.md).

## Sanctuary-only blanks, pictures and video

* **+ Add → Blank (sanctuary only)** adds a planned blank: the sanctuary
  screen goes black while the stream shows the camera. Rename it (e.g.
  "Blank during the sermon") in the Editor.
* Any item can be blank on one screen: in the Editor's **Backgrounds &
  layout**, tick **Blank on the sanctuary screen** (the stream is unaffected)
  or **Camera only on the stream** (the sanctuary is unaffected).
* In **Library → Media**, **Add (sanctuary only)** puts a picture or video
  full screen on the sanctuary while the stream shows the camera. **Add to
  service** shows it on both.

## Library tab

**Items**: built-in Rite I and Rite II texts (filter with the **Rite I** /
**Rite II** chips) and the items you have saved, by type, with previews of
both outputs. **Propers…** adds the day's Collect, readings and psalm from the
lectionary. **Media**: images and videos to show full screen, use as
backgrounds, or set as a psalm's music. **Add to service** (or double-click) puts a copy after the
selected cue. In the Editor, **Save to Library** keeps the selected item for
later services. See [library.md](library.md).

## The item's file (Source)

Editor → **More…** → **Edit the file (.ewm)…** shows the raw `.ewm` text of
the selected item; **← Editor** goes back. **Apply** parses it and applies it
(problems are listed with line numbers). **Revert** reloads it.

## Right pane

* **Sanctuary** and **Live Stream** previews show exactly what each
  output shows (the stream preview's checkerboard is transparent).
* **NOW / NEXT** name the current and following slide.
* **Prev / Next** step both screens together. Where the live stream runs
  the sanctuary's slides together (or splits a long one), one screen holds
  its slide while the other moves on; see
  [layouts.md](layouts.md#how-much-text-a-slide-holds).
* Then **Clear Text**, **Blackout**, **Clear Media** and
  **Before the service**. A button stays red while its effect is active. Clear Text
  and Clear Media reset when you move to another slide; Blackout stays on until
  you press it again.
* **Blackout** blacks out the **sanctuary screen only**; the live stream
  carries on. The **🔓** beside it locks Blackout as it is (on or off), on
  every computer: the button, **B** and Clear All cannot change it until you
  unlock it (**🔒**).
* **Stage message**: type and press Enter, or use a preset. It appears on the
  control screens only, never on the audience outputs.

| Key (outside text fields) | Does |
| --- | --- |
| → ↓ Space PageDown | Next |
| ← ↑ PageUp | Previous |
| B | Blackout on/off (sanctuary only; not while locked) |
| Esc | Clear text |
| M | Clear media |

## Two computers: sanctuary and live stream

One computer runs the service and the sanctuary screen; the other shows the
live-stream overlay and follows it, so every slide changes on both at once.

1. Put both computers on the same network (wired is best).
2. On the **sanctuary computer**, click the connection status at the right of
   the top bar ("One computer"), or ⚙ Settings → Connection to the other computer. Choose
   **Sanctuary computer**, then **Save** (the app restarts). The window lists
   this computer's address, e.g. `Sanctuary-Mac-mini.local` or
   `192.168.1.20`.
3. On the **live-stream computer**, open **Connection**, choose
   **Live-stream computer**, type that address and **Save**. The status then
   reads "Following <the sanctuary computer's name> ✓".
4. On each, open the screen it drives with the monitor icon (🖥): the sanctuary
   screen on the sanctuary computer, the stream on the live-stream computer.

The first time, macOS asks whether EpiscoPresenter may find devices on the
local network (on the live-stream computer) and accept incoming connections
(on the sanctuary computer): choose **Allow**. On a managed Mac, IT may need
to allow these. If the live-stream computer stays offline, check the
address, that both are on the same network, and that the sanctuary
computer's firewall allows EpiscoPresenter.

Pictures, PDFs and videos from the parish library are shown from each
computer's own copy of the library folder, so set the parish library folder
up on the live-stream computer as well (⚙ Settings → Parish library folder); services marked ready
are copied to it with their pictures. To go back to one computer, choose
**One computer for both**.

## Slave (live-stream) node

The same window, read-only: it follows the master and can send Next / Prev,
the quick actions and stage messages. Edit and save on the master.
