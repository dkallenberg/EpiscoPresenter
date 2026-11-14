# Slide layouts

**⚙ Settings → Slide layouts…** opens the Layout tool. It sets, for each kind of
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
* **Make this the default** keeps the slide type as it is now as the
  parish's own default (`layouts-default.json`, shared with every computer
  through the parish library). **Reset element** / **Reset <type> to the
  default** go back to it; **Built-in layout** goes back to the layout the
  app comes with (and makes that the default again). Without a default of
  the parish's own, Reset goes back to the built-in layout.

Dashed boxes are pieces the previewed slide does not use (for example the
title on a slide without a heading). Text shrinks below its size when it
would not fit its box, and the text box grows upward into a title's space on
slides that have no title.

The sanctuary hymn lyrics and the welcome bar keep their fixed layouts.

## How much text a slide holds

Under the element settings, **Text on … slides** sets, for each slide type
with a text box:

| Setting | What it does |
| --- | --- |
| Lines per slide | How many lines of text a slide holds. Empty on the live stream: what the text box holds at its type size (Left side 14, Bottom Bar 6, Lower Third 2, Hymn 13); moving or resizing the text box, or changing its size, changes it. Empty on the sanctuary: 9, the size of the Confession's slides. |
| Line breaks | *Each line on its own line* (the Prayer Book's sense lines) or *Run a paragraph's lines together* (the default for the stream's hymn verses). |
| Hanging indent for long lines | A line too long for the box carries on indented under itself, as the Prayer Book prints the Creed, the Lord's Prayer and the psalms. Text written as one long paragraph wraps as usual. |
| Indent every other psalm verse | Even-numbered verses are indented a step (on for the stream, off for the sanctuary; always on in the bulletin and script). Both halves of a verse start at the same indent; a half that runs onto another line carries on a step in. |

The slides of each item are made to fit:

* **The sanctuary** keeps the slides as written (the `---` breaks in the
  item) while they fit nine lines (or **Lines per slide**) at the type size
  of the Confession's slides. Text shrinks a little to fit, but a slide that
  would need it more than about 10% smaller is split; shorter slides keep
  their larger type.
* **The live stream** runs the written slides together while they fit, two
  verses of a hymn or more of a prayer to a slide, and splits a slide that is
  clearly too long (one only a little long shrinks to fit, as before). A new
  section (a heading), a picture or a title slide always starts a new slide.
* A split falls at the end of a line where it can, otherwise at the
  punctuation nearest the end of the room (a full stop, semicolon or colon
  before a comma, a comma before a space). A short response stays on the
  slide of the words it answers, and a rubric with what follows it.

**Next** steps through both screens together: a step starts wherever either
screen starts a slide, and the other screen holds its slide. In the
**Slides** tab, a step where the screen being shown holds is dimmed and
marked *holds*.

To break a slide on one screen only, change a slide break in the Editor from
**Both screens** to **Sanctuary only** or **Live stream only**.

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

