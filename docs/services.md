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

### Kind of service, bishop and baptism

**Service** chooses the order. *Holy Eucharist* is the Sunday or holy-day
order. Ash Wednesday, Palm Sunday, Maundy Thursday, Good Friday and Holy
Saturday are suggested from the date. The Great Vigil of Easter and the
Ordinations of a Deacon, Priest or Bishop are chosen by hand. These use the
Prayer Book's own order instead of a template:

| Service | Order (BCP) |
| --- | --- |
| Ash Wednesday (264) | Salutation, Collect and readings, sermon, Invitation to a Holy Lent, Imposition of Ashes, Psalm 51, Litany of Penitence, the Peace, the Eucharist |
| Palm Sunday (270) | The Liturgy of the Palms, procession, Collect and readings with the Passion Gospel's announcement, sermon, Creed, Confession, the Eucharist |
| Maundy Thursday (274) | The usual order with the Washing of Feet after the sermon |
| Good Friday (276) | Opening and Collect, readings and the Passion (John), sermon, the Solemn Collects, Devotions before the Cross, Confession, Lord's Prayer, Concluding Prayer (no blessing or dismissal) |
| Holy Saturday (283) | Collect, readings, sermon, "In the midst of life" |
| Great Vigil of Easter (285) | Service of Light, Exsultet, Liturgy of the Word, the Renewal of Baptismal Vows (or Holy Baptism), "Alleluia. Christ is risen" and Collect, Epistle, Psalm 114, the Gospel of the year, the Eucharist |
| Ordinations (512-551) | Opening, Presentation, Litany for Ordinations, Collect, readings (the Prayer Book's first choices), sermon, Creed (deacons and priests), Examination, Consecration, the Peace, the Eucharist, After Communion with the new minister's blessing and dismissal |

* **A bishop presides**: on the slides, bulletin and script the
  Celebrant's and Priest's parts are labelled *Bishop* (the texts are not
  changed), and the Bishop's Blessing (BCP 523) is added before the
  dismissal. Ordinations always have it. It can be turned on or off later
  in **⚙ Service**.
* **Baptism → Holy Baptism**: the baptismal opening acclamation replaces
  the usual one, Collect for Purity and Salutation; the Presentation and
  Examination, the Baptismal Covenant, the Prayers for the Candidates, the
  Thanksgiving over the Water (and, with a bishop, the Consecration of the
  Chrism) and the Baptism take the place of the Creed; there is no
  Confession, and the Peace is in the Baptism.
* **Baptism → Renewal of Baptismal Vows** takes the place of the Creed.
* **Confirmation, Reception, Reaffirmation** (with the bishop): with a
  baptism, its prayers and laying on of hands follow the Baptism; without,
  the Confirmation rite (BCP 413) takes the place of the Creed, Confession
  and Peace.

These texts print in full in the bulletin by default. In **⚙ Service**,
*Add to this service* adds Holy Baptism, the Renewal or Confirmation to an
existing service.

### Starting from

New and scheduled services start from either:

* **Standard order**, built from the Library in the chosen rite and adjusted
  for the season: Welcome & Announcements, Entrance Hymn, Opening Acclamation
  (Easter or Lent form in those seasons), Collect for Purity, Gloria (Kyrie in
  Advent and Lent), Salutation, **the propers**, Sermon, Nicene Creed,
  Confession, Absolution, Peace, Offertory Hymn, Great Thanksgiving, Sanctus,
  Lord's Prayer, Breaking of the Bread, (Prayer of Humble Access in Rite I),
  Invitation, Communion Hymn, Postcommunion Prayer, Welcome & Announcements,
  Closing Hymn, Dismissal and the post-service announcements loop. Hymn slots hold a note until you fill
  them.
* **A template** you saved (see below), or **By season**, which uses the
  template kept for each date's season and the standard order where there is
  none. Scheduling across a change of season uses each season's template.
* **Copy of** an existing service: everything except its propers, with its
  own copies of the texts and its custom backgrounds kept. The new day's
  propers are added in the same place.

**The propers are added automatically** from the lectionary for the date:
the Collect of the Day, the first reading and psalm (of the chosen track),
the second reading and the Gospel, with the psalm's verses from the Prayer
Book. To choose a holy day or different readings instead, use
**Library → Propers…** (see [library.md](library.md#propers)).

## Service order templates

A template is a named order of service, e.g. *Lent, Rite I* or *Easter*.
Arrange a service the way you want it for a season (items, hymn slots,
backgrounds, rite), then save it:

* **Services → Save open service as template…** (or **⚙ Service → Save order
  as template…**). Name it and tick the seasons to **use it automatically
  for** (Advent, Christmas, Epiphany, Lent, Holy Week, Easter, After
  Pentecost), or none to choose it by name only. The template keeps the
  order and texts but not the day's propers or date. Saving under an
  existing name replaces that template.
* The **Service order templates** card on the Services screen lists them:
  **Edit order** opens a template like a service (the top bar shows
  *Template:*; changes save automatically), **Name & seasons** renames it or
  changes its seasons, **Delete** removes it (services made from it are not
  affected).

Templates live in the app data folder under `templates/`.

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
