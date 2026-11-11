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

## Import propers (readings, collect and psalm)

The lectionary site (lectionary.episcopalchurch.org) blocks automated access,
so the app does not download from it. Instead:

1. Open the day's page in your web browser.
2. Select all (⌘A) and copy (⌘C).
3. In the app, select the cue the items should follow (usually the
   Salutation), open **+ Add → Import propers…**, paste (⌘V), and click
   **Find readings & collect**.

The app reads the page's structure:

* **The collect.** The page gives **Rite I** and **Rite II**. The **Collect**
  selector at the top chooses which one is ticked (Rite II by default, and the
  app remembers your choice).
* **The readings.** In the season after Pentecost, the **Readings** selector
  chooses **Track 1** or **Track 2** (remembered too). The epistle and Gospel
  belong to both tracks. Scripture references are taken only from the
  headings ("Old Testament: Judges 4:1-7"), never from the reading texts.
* **The psalm**, with all its verses. Optionally type an **antiphon**, and
  choose whether it repeats only at the start and end or also every 2, 4 or 6
  verses.

Change any tick or item type, then click **Add to service**:

| Item | Becomes |
| --- | --- |
| Collect | Collect of the Day: the day's name as heading, the collect, People: **Amen.** |
| First / Second Reading | title slide ("The First Reading" + citation), then Reader: *The word of the Lord.* People: **Thanks be to God.** |
| Psalm | verses two per slide, with the verse numbers at the left. Verses alternate choir (plain) and congregation (**bold**), starting with the choir, with no "choir"/"congregation" labels. The antiphon, if any, is bold on its own slides. |
| Gospel | title slide, both Gospel acclamations, naming the evangelist |

Reading texts are not imported: the screens show only citations and
responses, and the NRSV text is copyrighted. The collects and psalms are from
the 1979 Book of Common Prayer.

### Psalm layout (after importing)

Select the psalm and open the **Editor**. The **Psalm layout** panel sets the
**antiphon**, how often it **repeats**, and **verses per slide** (1–4).
**Apply** rebuilds the slides from the numbered verses. A chant image added
with **♪ Music** moves to the antiphon slides. You can also edit the verses
directly; a verse is a block whose role is its number (`[1]`, `[2]` … in the
`.ewm` file).

To check how the app reads a pasted page, save it as a text file and run:

```bash
cargo run -p episco-core --example propers_dump -- page.txt
```
