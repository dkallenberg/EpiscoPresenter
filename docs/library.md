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

## Import propers (readings and collect)

The lectionary site (lectionary.episcopalchurch.org) blocks automated access,
so the app does not download from it. Instead:

1. Open the day's page in your web browser.
2. Select all (⌘A) and copy (⌘C).
3. In the app, open **+ Add → Import propers…** (or **Import propers…** in the
   Library tab), paste (⌘V), and click **Find readings & collect**.

The app finds the day's name, the collect (up to its "Amen.") and every
scripture citation. In the season after Pentecost the page lists two tracks.
The first track's reading and psalm are ticked, the other track and any "or"
alternatives are not, and the epistle and Gospel are shared. Change the ticks
or the type of an item, then click **Add to service**. The items go after the
selected cue:

| Item | Becomes |
| --- | --- |
| Collect | Collect of the Day: the day's name as heading, the collect, People: **Amen.** |
| First / Second Reading | title slide ("The First Reading" + citation), then Reader: *The word of the Lord.* People: **Thanks be to God.** |
| Psalm | title slide with the citation, and a placeholder slide for the verses |
| Gospel | title slide, both Gospel acclamations, naming the evangelist |

Reading texts are not imported, since the screens show only citations and
responses. Paste psalm verses into the psalm item's editor.

Copying works from any page that lists the readings, not only that site.
