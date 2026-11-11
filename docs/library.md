# Library and propers

## The Library tab

The **Library** tab lists reusable items, which you can filter by type
(Acclamation, Collect, Prayer, Hymn, …) or search by title and text. Selecting
an item previews every slide on both outputs.

* **Built-in** items are texts from the Holy Eucharist, Rite Two, of the 1979
  Book of Common Prayer (public domain): the opening acclamations (ordinary,
  Easter, Lent), Collect for Purity, Gloria, Kyrie, Salutation, Nicene Creed,
  Confession, Absolution, Peace, Sursum Corda, Sanctus, Lord's Prayer
  (contemporary and traditional), the Breaking of the Bread, Invitation, both
  Postcommunion prayers and the four Dismissals. They ship with the app (the
  `library/` folder in the source).
* **Mine** items are ones you saved. In the **Editor**, **Save to Library**
  keeps a copy of the selected item, including your edits, for use in later
  services. They are `.ewm` files in the app's data folder (on macOS,
  `~/Library/Application Support/org.episcopresenter.app/library/`). You can
  also copy `.ewm` files into that folder yourself.

**Add to service** (or double-click) inserts a copy after the selected cue.
The copy belongs to this service: editing it never changes the library. It is
saved beside the service plan, in `liturgies/`, the next time you **Save
Service**.

Please proofread the built-in texts against your prayer book before relying
on them in worship.

## Media

**Library → Media** holds images and videos (`png`, `jpg`, `gif`, `svg`,
`webp`; `mp4`, `mov`, `m4v`, `webm`). **Import…** copies files into the app's
`media` folder (in the app data folder). Select one to:

* **Add to service**: a new item that shows it full screen on both outputs
  (double-click does the same). Videos play while the item is live, with
  sound on the sanctuary computer; previews show a still frame.
* Use it as the **background** of the selected item: Sanctuary, Live Stream
  or Both.
* For a psalm, make it the **Psalm's musical setting** (see below).
* Rename or delete it.

Images in the media library also appear in every background picker, and the
editor's **Choose…** buttons for music images open the media library first.

## Propers

**Library → Propers…** (also in the timeline's **+ Add** menu) gives the
Collect of the Day, readings and psalm for a date.

### From the lectionary (default)

The date starts at the open service's date. The app names the day in the
church year and looks it up in the parish's Revised Common Lectionary
reference (`library/lectionary/rcl.md`):

* **Sundays and seasons** of Years A, B and C, with **Track 1** and
  **Track 2** in the Season after Pentecost (Propers 3–29, chosen by the date
  as "the Sunday closest to …").
* **Holy days.** The Prayer Book's precedence rules (BCP pp. 15–17) pick the
  observance: Principal Feasts take precedence over Sundays; the Presentation,
  Transfiguration and Holy Cross replace a Sunday after the Epiphany or after
  Pentecost; other holy days falling on a Sunday are offered under
  **Observance** but do not replace it; All Saints' Day is also offered on
  the Sunday after November 1.
* **Observance** also lists the Common of Saints and the Lesser Feasts for
  the date, for weekday services.

Each item is listed with a tick box:

| Item | From |
| --- | --- |
| Collect of the Day | The 1979 Prayer Book, in **Rite I** and **Rite II** (the **Collect** picker chooses; it starts at the service's rite). Where the book gives a choice ("or this", e.g. Christmas, Easter Day), the others are listed unticked. |
| Readings | Citations from the lectionary. **Readings** chooses Track 1 or 2 (starting at the service's track). Alternatives ("or …") are listed unticked. |
| Psalm | The verses from the **1979 Psalter**, with optional verses `[…]` included; canticles (e.g. *Canticle 15: Magnificat*) from the Daily Office, in their traditional form in a Rite I service. |
| Liturgy of the Palms | On Palm Sunday, unticked. |

**Add to service** puts the ticked items after the selected cue. Gospel
responses follow the service's rite ("Glory to you, Lord Christ" or "Glory be
to thee, O Lord").

**Corrections to the reference.** Checked against the Prayer Book's own
lectionary tables, the parish document had three errors, corrected in the
app's copy (and listed at its top): in Year A the readings for Propers 4–28
were each listed one Proper early; Proper 3 repeated Proper 4 in all three
years (it now has the Eighth Sunday after the Epiphany readings); and Year C
Proper 4 Track 1 repeated Track 2 (now 1 Kings 18:20-21 [22-29] 30-39 with
Psalm 96).

### Paste a page

The **Paste a page** tab still reads a lectionary web page pasted in (for
anything the reference does not cover).

### Reading texts

The NRSV and NRSVue are copyright the National Council of Churches, so the
app does not include or download their text. To show a reading's text:

1. Select the reading (or the Gospel) and open the **Editor**.
2. In **Reading text**, choose **NRSVue** or **NRSV** and click
   **Open *citation* ↗**. Your browser opens the passage (Bible Gateway for the
   NRSVue, the Oremus Bible Browser for the NRSV).
3. Copy the passage, paste it into the box and click **Apply**.

The app removes the reference and version lines, section headings, verse
numbers, footnote and cross-reference markers, and the footnotes and copyright
notice at the end, then lays the text out about 30–70 words to a slide
(**Slides**) between the title slide (or the Gospel's announcement) and the
closing response. The version's initials, e.g. "(NRSVue)", follow the last
slide, as the NCC asks for use in services. **Remove text** goes back to the
citation and responses only.

### Psalm layout (after importing)

Select the psalm and open the **Editor**. The **Psalm layout** panel has:

| Setting | What it does |
| --- | --- |
| Antiphon music | The music line for the antiphon, from the media library (or any file). It shows on both screens: on the sanctuary almost full width with the words retyped under it. |
| Musical setting (stream) | An image of the psalm's setting (chant or tone), shown on the live stream between the gold title box and the text bar, on every slide of the psalm. |
| Antiphon words | The antiphon's text (sung by all), bold. |
| Antiphon | Where it repeats: *start & end*, *every 2/3/4/6 verses*, or *where placed* (keep the antiphons you put in by hand). |
| Slides | Verses per slide (1–4). |

**Apply** rebuilds the slides from the numbered verses, keeping who sings
each verse.

In the block list:

* **Antiphon** blocks are labelled and tinted gold. An antiphon is a
  `# Antiphon` section (its own slide) with the music image and an
  `[Antiphon]` paragraph for the words.
* **♫ Antiphon** (between any two blocks) inserts a copy of the psalm's
  first antiphon (music and words) there, so the antiphon can come after
  any verse, however uneven the groups are.
* Beside each verse, **Choir / People** sets who sings it. The people's
  verses are bold; the choir's are plain. Imported psalms start alternating,
  choir first.

A verse is a block whose role is its number (`[1]`, `[2]` … in the `.ewm`
file).

**Psalm with no verses?** If the Propers dialog says no verses were found (or a
psalm was imported with an older version and shows only its title), paste
the verses into the **Paste verses** box in the Psalm layout panel and click
**Apply**. Copy them from the lectionary page or anywhere else: each verse
starts with its number (`1`, `2`, …); blank lines, line breaks inside a
verse, and numbers on their own line are all fine. Pasting again replaces
the verses.

To check how the app reads a pasted page, save it as a text file and run:

```bash
cargo run -p episco-core --example propers_dump -- page.txt
```
