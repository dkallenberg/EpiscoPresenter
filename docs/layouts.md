# Slide layouts

**▦ Layouts** in the top bar opens the Layout tool. It sets, for each kind of
slide, where every piece goes and how its text looks. Changes apply to every
service, save straight away (`layouts.json` in the app data folder), and are
sent to the live-stream computer with the service.

| Output | Slide type | Used for | Pieces |
| --- | --- | --- | --- |
| Live Stream | Left side | Prayers, collects, dialogues (left panel) | Title, Reference, Subtitle, Text |
| Live Stream | Bottom Bar | Psalms, Eucharistic Prayer | Title in the gold box, Musical setting image, Text (and an optional title in the bar) |
| Live Stream | Lower Third | Readings, short responses over the camera; the sermon (the preacher's name in the bar, "The Sermon" and the position above) | Title, Reference, Text |
| Live Stream | Hymn | Hymns (wide left panel) | Title, Hymn number, Hymn title, Verses, Credits |
| Sanctuary | Prayer | Prayers and dialogues | Title, Section heading, Text |
| Sanctuary | Title | Title slides ("The First Reading / Joshua 24:1-3a") | Heading, Main title |
| Sanctuary | Sermon | The sermon, laid out like a reading's title slide ("The Sermon / The Very Rev. Jane Doe / Rector") | Heading, Preacher, Position |
| Sanctuary | Psalm | Psalm verses and antiphons, and the music line before the Gospel | Psalm title, Verses, Antiphon music, Antiphon words |

Every piece also has **Speaker labels & verse numbers** and **People (bold)
lines** styles, which apply inside the text box.

## Using the tool

* Pick a slide type on the left. The preview shows a slide from the open
  service that uses it (choose another under **Preview**), or a sample.
* **Click** a box to select it, **drag** to move it, drag its **corner** to
  resize. Arrow keys nudge by 1 px (⇧ by 10 px).
* On the right set **X, Y, Width, Height** (on the 1920 × 1080 screen),
  **Font**, **Size**, **Colour**, **Bold**, **Italic**, **Align** and
  **Vertical** alignment, or untick **Show this element** to hide it.
  **Auto** colour follows the background's light or dark text.
* **Reset element** / **Reset <type>** go back to the built-in layout.

Dashed boxes are pieces the previewed slide does not use (for example the
title on a slide without a heading). Text shrinks below its size when it
would not fit its box, and the text box grows upward into a title's space on
slides that have no title.

The sanctuary hymn lyrics and the welcome bar keep their fixed layouts.

## The “People” label

Choose **People (bold) lines** as the element on any slide type and untick
**Show the “People” label** to leave the label off every slide type at
once (the bold text already marks the people's parts; the text keeps its
place beside the other speakers'). Tick it again to bring it back. In the
layout file this is `hidden = true` on each type's `people` element.

## Hymn credits

On the Hymn slide the Words / Music credits keep their size and grow
upward from the bottom of their box, up to four lines; the verse box above
gets shorter to make room. Only credits longer than four lines are made
smaller.

