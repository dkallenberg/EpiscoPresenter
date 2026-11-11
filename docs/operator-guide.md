# Operator guide

The control window has three panes: the **service timeline** (left), the
**slides / editor** (center) and the **live previews and stage controls** (right).

## Top bar

| Button | Does |
| --- | --- |
| New | Starts an empty service. |
| Open… | Opens a service plan (`.toml`). |
| Example | Loads the built-in sample service. |
| Save Service | Saves every changed item that already has a file, then the plan. Shift-click for *Save As*. A ● means there are unsaved changes. |

The right end shows the node's role and network status (and latency on a
slave). A stage message, when one is active, is shown here on every node.

## Service timeline (left)

* The first row is **Pre-Service**: the announcements loop that runs until the
  first cue is taken. It is marked LIVE while it runs.
* Each cue shows its number, type icon, title and lower-third caption, plus:
  * **LIVE 2/5**: on air, and which slide;
  * the **slide count**, or **GS loop / GS 1×** for a Google Slides cue;
  * **●**: unsaved changes;
  * a **colour bar**: its liturgical colour (hover for the background image).
* **Click** selects a cue (for the editor and slide grid), **double-click** puts
  it on air, **drag** reorders. Hover a row for ↑ ↓ ✕.
* **+ Add** inserts after the selected cue: existing `.ewm` files (several at
  once) or a new item from a template.

## Slides tab

Every slide of the service as it will look on the sanctuary screen, grouped by
cue and numbered. Click a slide to put it on air. The live slide has a red frame.

## Editor tab

Edits the selected item. Changes reach every output about a third of a second
after you stop typing.

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

## Source tab

The raw `.ewm` text of the selected item. **Apply** parses it and applies it
(problems are listed with line numbers). **Revert** reloads it.

## Right pane

* **Sanctuary** and **Live stream overlay** previews show exactly what each
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
