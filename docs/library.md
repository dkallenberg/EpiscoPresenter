# Library and propers

## The Library tab

The **Library** tab lists reusable items, which you can filter by type
(Acclamation, Collect, Prayer, Hymn, …) or search by title and text. Selecting
an item previews every slide on both outputs.

* **Built-in** items are texts of the Holy Eucharist from the 1979 Book of
  Common Prayer (public domain), in **Rite Two** and **Rite One** (filter with
  the **Rite I** / **Rite II** chips): the opening acclamations (ordinary,
  Easter, Lent), Collect for Purity, Summary of the Law (Rite I), Gloria,
  Kyrie, Salutation, Nicene Creed, Confession, Absolution, Peace, Sursum Corda,
  Sanctus, **the Eucharistic Prayers** (Rite II: Prayers A, B, C and D; Rite I:
  Prayers I and II, each from the Sursum Corda to the Great AMEN, with the
  people's parts in bold), Lord's Prayer, the Breaking of the Bread, O Lamb of
  God and the Prayer of Humble Access (Rite I), Invitation, Postcommunion
  prayers, the Blessing (Rite I) and the four Dismissals. They ship with the
  app (the `library/` folder in the source).
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

## Hymns

The app does not include the Hymnal 1982. The hymnal is a copyrighted
publication and many of its texts and translations are under copyright, so
hymn words come from your parish's own licensed source (the hymnal's digital
edition, a OneLicense or CCLI reprint licence, …):

* **One at a time.** Select a hymn item (new services have Entrance,
  Offertory, Communion and Closing Hymn slots) and open the **Editor**. Paste
  the words into **Hymn words** and click **Apply**: numbered stanzas become
  one slide each with the number as a label, a stanza marked `Refrain:` (or
  `Chorus:`) is the refrain (tick **Repeat the refrain after every stanza** to
  repeat it), and stanzas over eight lines are split over two slides.
  `Words:` / `Music:` lines and copyright notices go into **Words**, **Music**
  and **Licence notice**, which the stream shows under the hymn.
* **Many at once.** **Library → Import hymns…** takes text files, one per
  hymn, named with the number and title (`436 Lift Up Your Heads.txt`,
  `S 280 Gloria.txt`), and saves each as a hymn in **Mine**. Search the
  Library by number ("Hymn 436") or title, then **Add to service**.

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

### Music before the Gospel

The Gospel's slides run in the order they are used: the music line (if
any), the announcement ("The Holy Gospel of our Lord Jesus Christ according
to …" / "Glory to you, Lord Christ."), the title slide, the text (if shown),
and "The Gospel of the Lord." / "Praise to you, Lord Christ."

Select the Gospel and open the **Editor**. In **Music before the Gospel**,
**Choose…** a picture of the Alleluia or Gospel acclamation from the media
library; it goes on the first slide, laid out like the psalm's antiphon
music. **Remove** takes it off. **Put the slides in order** rearranges a
Gospel made before this order (or edited by hand).

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

## Pasting a text (Enriching Our Worship and other supplemental rites)

Some texts the parish uses are copyright and cannot be built into the app:
the Eucharistic Prayers of *Enriching Our Worship* (© The Church Pension
Fund), for example. Add them from the parish's own copy:

1. **Library → Paste a text…**
2. Give it a title ("Eucharistic Prayer 1 (EOW)"), a type (Eucharistic
   Prayer) and where it comes from ("EOW 1, p. 57"; printed on the
   bulletin).
3. Copy the prayer from the book or PDF and paste it. Start each speaker's
   part with the speaker's name, as the book prints it:

   ```text
   Presider    The Lord be with you.
   People      And also with you.
   Presider and People
   Holy, holy, holy Lord, God of power and might, …
   ```

   A line starting with a speaker's name (Celebrant, Presider, People, All,
   Deacon, Reader, Leader; then a tab, a colon or spaces) starts that part,
   and the lines after it continue it. A blank line starts a new paragraph
   by the same speaker. "Presider and People" or "All" is said together
   (bold). *Presider* is shown as *Celebrant*, like the parish's other
   items, so "A bishop presides" relabels it too. Lines in [brackets] become
   rubrics.
4. **Add to library**. The item is saved in the parish library folder, so
   everyone can use it. Look it over in the Editor (speakers, rubrics, slide
   breaks) and save any corrections back with **Save to Library**.
