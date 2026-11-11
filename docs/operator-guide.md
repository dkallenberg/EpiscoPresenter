# Operator guide

The control window has three panes: the **service timeline** (left), the
**slides / editor** (center) and the **live previews and stage controls** (right).

## Top bar

| Button | Does |
| --- | --- |
| ☰ Services | The Services screen: upcoming services, New service, Schedule upcoming Sundays, Open file…, Example. It also opens when the app starts. See [services.md](services.md). |
| Save status | ✓ Saved / Saving…, since services save themselves. For the example, *Not saved · Keep…* adds it to your services. |
| ⚙ Service | Title, date, season, colour, rite (I or II), lectionary track, Bible version for reading texts, the announcements deck (Google Slides link and its number of slides) and the stream key colour. |
| ▦ Layouts | Where text, titles and music go on each kind of slide, and their fonts. See [layouts.md](layouts.md). |
| Save As… | Writes a copy of the service elsewhere. |

The right end shows the node's role and network status (and latency on a
slave). A stage message, when one is active, is shown here on every node.

## Service timeline (left)

* The first row is **Pre-Service**: the announcements loop that runs until the
  first cue is taken. It is marked LIVE while it runs.
* Each cue shows its number, type icon, title and lower-third caption, plus:
  * **LIVE 2/5**: on air, and which slide;
  * the **slide count**, or **GS loop / GS 1×** for a Google Slides cue;
  * **●**: unsaved changes;
  * a **background thumbnail**: the sanctuary look, with the stream look
    inset at the corner (hover for the names).
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

**Backgrounds & layout** (the top section) has a card for each output. Each card
has a live preview and these settings:

* **Background**: opens a picker with the built-in designs (drawn in the
  liturgical colour), the key colour, black, and your library images. **Choose
  image file…** uses any image where it is; **Import to library…** copies
  images into the `backgrounds/` folder beside the service plan.
* **Layout**, **Text** and **Fit**. *Auto* shows what the item gets from its
  type.
* **Use for all … items**: makes this look the service default for every item
  of the same type (for example, every Collect), so you set each background
  once.
* **Reset**: returns the item to its type's default. A **custom** badge marks
  an item with its own look.

See [backgrounds.md](backgrounds.md) for the layouts and their text areas.

* **Header fields** (title, type, season, colour, background, citation, hymn #)
  are saved in the `.ewm` file. **Timeline label** is saved in the service plan
  instead (e.g. "Processional Hymn" for *Lift Up Your Heads*).
* **Rows** are blocks. The dropdown sets the speaker role, which sets the row's
  colour and the on-screen style (People is bold black; Rubric is red italic
  and never projected).
* Hover between rows for **( + ADD BLOCK )**. The new block continues the
  dialogue (after Priest comes People, after People the previous speaker). The
  small buttons beside it add a **§ Heading** (starts a new slide with a
  title), a **⎯ Slide break**, or a **♪ Music** snippet (an image).
* Each line in a row is one line on screen. Press Enter for a new line.
* Hover a row for ↑ ↓ ✕.
* **On-air protection:** while the item you are editing is on air, edits are
  held, and a red banner offers *Apply to screens* or *Discard*. Untick **Hold
  on-air edits** to apply them as you type.

| Key (in a text row) | Does |
| --- | --- |
| ⌘B / ⌘I | Bold / italic |
| ⌘↩ | New block after this one |
| ⌫ in an empty row | Delete the row |
| ⌘S / ⇧⌘S | Save item / Save item as… |
| ⌥⌘S | Save service |

## Announcements (Google Slides)

Paste the deck's link in **⚙ Service → Announcements deck**: the ordinary
sharing link (Share → Copy link) works, as do "Publish to web" and present
links. The deck must be shared as **Anyone with the link can view** (or
published to the web), since the output computers show it through Google's
embedded player and need to reach docs.google.com.

* **Pre-Service** loops the deck on the sanctuary screen, a slide every 9
  seconds. (The stream shows its Welcome design.)
* An **Announcements** item in **Loop** mode does the same until you move on.
* In **Single pass** mode, if **Slides in the deck** is set, **Next** /
  **Prev** step through the slides one by one and then on to the next item;
  without the count the deck plays through once on its own.
* The control window's previews show a "Google Slides · Slide 3 of 8" card;
  the deck itself appears on the output screens.

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

## Source tab

The raw `.ewm` text of the selected item. **Apply** parses it and applies it
(problems are listed with line numbers). **Revert** reloads it.

## Right pane

* **Sanctuary** and **Live Stream** previews show exactly what each
  output shows (the stream preview's checkerboard is transparent).
* **NOW / NEXT** name the current and following slide.
* **Prev / Next**, then **Clear Text**, **Blackout**, **Clear Media** and
  **Pre-Service**. A button stays red while its effect is active. Clear Text
  and Clear Media reset when you move to another slide; Blackout stays on until
  you press it again.
* **Stage message**: type and press Enter, or use a preset. It appears on the
  control screens only, never on the audience outputs.

| Key (outside text fields) | Does |
| --- | --- |
| → ↓ Space PageDown | Next |
| ← ↑ PageUp | Previous |
| B | Blackout on/off |
| Esc | Clear text |
| M | Clear media |

## Slave (live-stream) node

The same window, read-only: it follows the master and can send Next / Prev,
the quick actions and stage messages. Edit and save on the master.
