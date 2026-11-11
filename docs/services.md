# Services, scheduling and autosave

## The Services screen

When the app opens, it shows **Services**. Open it again any time with
**☰ Services** in the top bar.

* **Upcoming** lists scheduled services by date. Each row shows the day's name
  in the church year, its lectionary year and Proper, its season colour and how
  many items it has. **Readings needed** means no reading or Gospel has been
  added yet. Click a row to open that service, or hover it and click ✕ to
  delete it.
* **Past services** are folded away below.
* **New service** creates one service for a date. The title fills in from the
  church year (for example, *The Twenty-Fifth Sunday after Pentecost*). You can
  change it.
* **Schedule upcoming Sundays** creates a service for each of the next 2–8
  Sundays (4 by default, about a month) that doesn't have one yet.
* **Open file…** opens a service plan from anywhere, and **Example** loads the
  built-in example.

### Rite, readings and Bible

Both cards have three choices, remembered for next time:

| Choice | Options | What it does |
| --- | --- | --- |
| Rite | Rite II (default), Rite I | The service texts (Collect for Purity, Gloria, Creed, Confession, Peace, Great Thanksgiving, Lord's Prayer, Postcommunion, …) and the Collect of the Day in contemporary or traditional language. Rite I services also include the Prayer of Humble Access. |
| Readings | Track 1 (default), Track 2 | Which Old Testament reading and psalm to use in the Season after Pentecost. |
| Bible | NRSVue (default), NRSV | The translation offered when you paste reading texts (see [library.md](library.md#reading-texts)). |

The same three are in **⚙ Service** for the open service. Changing them
there affects texts added from then on.

### Starting from

New and scheduled services start from either:

* **Standard order**, built from the Library in the chosen rite and adjusted
  for the season: Welcome & Announcements, Entrance Hymn, Opening Acclamation
  (Easter or Lent form in those seasons), Collect for Purity, Gloria (Kyrie in
  Advent and Lent), Salutation, **the propers**, Sermon, Nicene Creed,
  Confession, Absolution, Peace, Offertory Hymn, Great Thanksgiving, Sanctus,
  Lord's Prayer, Breaking of the Bread, (Prayer of Humble Access in Rite I),
  Invitation, Communion Hymn, Postcommunion Prayer, Closing Hymn, Dismissal and
  the post-service announcements loop. Hymn slots hold a note until you fill
  them.
* **Copy of** an existing service: everything except its propers, with its
  own copies of the texts and its custom backgrounds kept. The new day's
  propers are added in the same place.

**The propers are added automatically** from the lectionary for the date:
the Collect of the Day, the first reading and psalm (of the chosen track),
the second reading and the Gospel, with the psalm's verses from the Prayer
Book. To choose a holy day or different readings instead, use
**Library → Propers…** (see [library.md](library.md#propers)).

## Autosave

Services in the app's folder save themselves about 1.5 seconds after you stop
editing: text, order, backgrounds, settings, everything. The top bar shows the
status:

| Status | Meaning |
| --- | --- |
| ✓ Saved | Everything is on disk. |
| Saving… | A change will be saved in a moment. |
| Not saved · Keep… | The service has no file (the example). Click to keep it with your services; from then on it saves itself. |
| ⚠ Save failed | Click for the reason (for example, a full disk). |

**Save As…** writes a copy of the service elsewhere, for example to share it.
After that, the copy is the one that is open and being saved.

## Where services are kept

Each service is a folder in the app's data folder, named by date and title.
On macOS that's
`~/Library/Application Support/org.episcopresenter.app/services/`:

```
services/
  2026-11-15-the-twenty-fifth-sunday-after-pentecost/
    service.toml        the plan (order, backgrounds, settings)
    liturgies/          the texts, one .ewm file per item
```

These are ordinary files: back them up, or copy a folder to another computer
and open its `service.toml` with **Open file…**.

## The church calendar

Dates are named by the 1979 Book of Common Prayer calendar with the Revised
Common Lectionary:
* Advent, Christmas, the Epiphany (with the Baptism of Our Lord and the Last
  Sunday after the Epiphany), Lent, Palm Sunday and Holy Week, Easter,
  Ascension, Pentecost, Trinity Sunday, the Sundays after Pentecost with Proper
  numbers, Christ the King, and All Saints' Day.
* The lectionary year (A, B or C) changes on the First Sunday of Advent.
* Each day gets its season colour.
