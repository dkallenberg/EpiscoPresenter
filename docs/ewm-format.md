# Episcopal Worship Markdown (`.ewm`)

One `.ewm` file holds one liturgy item: a prayer, dialogue, hymn, reading or
psalm. The same file feeds every output (sanctuary display, stream overlay,
officiant script, bulletin).

```markdown
---
title: Great Thanksgiving - Rite II
season: Pentecost
color: Green
type: eucharistic_prayer
default_background: "green_stained_glass.png"
---

[Priest] The Lord be with you.
[People] **And also with you.**
[Priest] Lift up your hearts.
[People] **We lift them to the Lord.**
```

Files are UTF-8. A byte-order mark and Windows (CRLF) line endings are accepted.

## Front matter

An optional header between two `---` lines at the very top of the file. Each
line is `key: value`. Values may be bare or quoted (`"..."` with `\"`/`\\`
escapes, or `'...'` with `''` for a quote). Lines starting with `#` are comments.

| Key | Meaning | Values |
| --- | --- | --- |
| `title` | Item title (shown in the setlist, script and bulletin) | text |
| `type` | Element category; selects layouts on both outputs | see below |
| `season` | Liturgical season | `Advent`, `Christmas`, `Epiphany`, `Lent`, `Holy Week`, `Easter`, `Pentecost` (also `Season after Pentecost`) |
| `color` | Liturgical colour; picks the stained-glass background family | `Green`, `Violet` (`Purple`), `White`, `Red`, `Blue`, `Black` |
| `default_background` | Background image file, overriding the colour default | file name |
| `citation` | Scripture reference for lower-thirds and bulletins | e.g. `Joshua 24:1-3a` |
| `hymn_number` | Hymnal number | e.g. `436`, `S 125` |
| `tune` | Hymn tune name | text |
| `words`, `music`, `setting` | Hymn credits, shown at the foot of the stream slide | text |
| `setting_image` | Psalms: image of the musical setting, shown on the stream between the gold title box and the text bar | image path |
| `antiphon`, `antiphon_every`, `antiphon_placed`, `verses_per_slide` | Psalms: how **Apply** in the Psalm layout panel arranges the slides | text, number, `true`, number |
| `media` | A media item (image or video shown full screen) | file path |
| `rite` | Which rite a text belongs to (shown in the Library) | `I`, `II` |
| `bible` | Translation of a reading's pasted text | `NRSV`, `NRSVue` |

If `color` is omitted it follows the season (Advent → Blue, Christmas/Easter →
White, Epiphany/Pentecost → Green, Lent → Violet, Holy Week → Red). If
`default_background` is omitted it is `<color>_stained_glass.png`.

`citation` is also the reference shown in a stream slide's header: a
scripture reference for readings, or a prayer book page such as `BCP 355`.
For hymns the header shows `Hymn <hymn_number>`.

Unknown keys are kept as-is and written back when the file is saved. An invalid
`season`, `color` or `type` produces a warning and is likewise preserved.

### `type` values and output layouts

| `type` | Sanctuary | Live stream |
| --- | --- | --- |
| `prayer` (default), `collect`, `acclamation` | seasonal background | left 45% panel |
| `hymn` | white text on black | left 45% panel |
| `psalm`, `gospel`, `eucharistic_prayer` | seasonal background | bottom bar (28%) |
| `reading`, `homily`, `anthem`, `announcements`, `other` | seasonal background | lower-third badge |

Aliases: `dialogue`/`liturgy` → `prayer`, `lesson`/`epistle` → `reading`,
`great thanksgiving` → `eucharistic_prayer`, `sermon` → `homily`, `choir` → `anthem`.

On the stream, a slide that contains only sheet music becomes a text
lower-third (e.g. *Hymn 436: Lift Up Your Heads*).

## Body

### Speaker blocks

`[Role] text` starts a block spoken by `Role`. Following non-blank lines
continue the same block, and each line break is kept. A blank line ends the block.

```markdown
[Reader] Hear my teaching, O my people; *
incline your ears to the words of my mouth.
```

The tag may sit on its own line:

```markdown
[People]
**Holy, holy, holy Lord, God of power and might,**
**heaven and earth are full of your glory.**
```

Role names are case-insensitive:

| Role | Aliases | Rendering |
| --- | --- | --- |
| `Celebrant` | `Officiant` | 55pt regular, `#404040` |
| `Priest` | `Presider` | 55pt regular, `#404040` |
| `Bishop` | | 55pt regular, `#404040` |
| `Deacon` | | 55pt regular, `#404040` |
| `Reader` | `Lector` | 55pt regular, `#404040` |
| `People` | `All`, `Congregation` | 55pt **bold**, `#000000`; included in bulletins |
| `Rubric` | | instructions; not projected |

A **number** in place of a role, such as `[1]`, `[2]` or `[12a]`, is a
**verse number**. It shows in the label column, in bold and upright, on the
stream and on the sanctuary screen (except in hymn lyrics there). Use it for
psalm verses and hymn stanzas.

Any other name (e.g. `[Cantor]`) is kept as a custom role and rendered like an
officiant. Role labels are drawn at 48pt italic.

Text with no tag forms a block with no speaker.

### Inline formatting

* `**bold**` and `*italic*`, which can be nested (`***both***`).
* Formatting never crosses a line break. In a multi-line block, mark each line
  separately (`**line one**` / `**line two**`). This keeps the psalm-pointing
  asterisk at the end of a half-verse from pairing with one on the next line.
* An asterisk with no matching partner on the same line stays literal (`5 * 3`),
  so psalm pointing asterisks need no escaping unless a line has two of them.
* `\` escapes any punctuation: `\*`, `\[`, `\#`, `\\`.

### Structure

| Syntax | Meaning |
| --- | --- |
| `# Heading` … `###### Heading` | Section title. Starts a new slide and is shown as that slide's heading. |
| `---` (three or more dashes, alone on a line) | Explicit slide break. |
| `--- stream` / `--- sanctuary` | A slide break on that screen only. |
| Leading tabs (or every three spaces) on a line | Indent: one step each, as the Prayer Book sets the Creed and the Lord's Prayer. |
| `![alt text](path/to/image.png)` alone on a line | Sheet-music snippet or other image, shown full-width. |
| `<!-- comment -->` | Ignored. May span several lines. |

A line that would otherwise be read as syntax can be escaped with a leading
backslash: `\[Not a role]`, `\# not a heading`.

### Indents

A line's leading tabs indent it, one step a tab (three spaces count as a
tab). Write a paragraph whose first line is indented with its tag on a line
of its own:

```markdown
[All] **We believe in one God,**
	**the Father, the Almighty,**
	**maker of heaven and earth,**

---

[All]
	**Through him all things were made.**
		**he came down from heaven:**
```

Psalm verses need no tabs: the two halves of a verse (split at the `*`)
each start a line at the same indent, a half too long for its line carries
on indented under itself, and every other verse can be indented a step (see
[layouts.md](layouts.md#how-much-text-a-slide-holds)). In the Editor, **Tab** and **⇧Tab** indent and
outdent the line the cursor is on.

### Slides

The body becomes slides by splitting at every `---` break and every heading.
A heading labels its own slide and every later slide until the next heading.
Its first slide shows it as a header (for example "Psalm 78:1-7 … Antiphon").

A heading followed directly by a `---` break, or by another heading, makes a
**title slide**. On the sanctuary screen it shows the heading with the
citation large beneath it; on the stream it shows as a lower-third title.

```markdown
# The First Reading
---
[Lector] The word of the Lord.
[People] **Thanks be to God.**
```

Other empty slides are dropped.

These are the sanctuary's slides. The live stream runs them together where
they fit and splits ones too long for its boxes (see
[layouts.md](layouts.md#how-much-text-a-slide-holds)). `--- stream` and
`--- sanctuary` break one screen only.

How slides look (background, layout and text colours) is set per cue and per
output, not in the `.ewm` file. See [backgrounds.md](backgrounds.md).

## Diagnostics

The parser is lenient so that a typo never blocks a service. Problems are
reported as warnings with a line number, and the file still loads:

* malformed front-matter lines, unknown `season`/`color`/`type` values
* a role tag with no text
* an unmatched `**` on a line (usually bold split across a line break)
* an unclosed `<!--` comment

The only fatal error is a front-matter block that opens with `---` and never
closes.

## Service plans (`.toml`)

A service is a TOML file listing cues in order. Paths are relative to the plan.

```toml
title = "Holy Eucharist, Rite II — Proper 27"
date = "2026-11-08"
season = "Pentecost"                 # default colour for cues that set none
announcements_deck = "https://docs.google.com/presentation/d/e/…/pub"
rite = "I"                           # optional: Rite I (default Rite II)
track = 2                            # optional: lectionary Track 2 (default 1)
bible = "NRSV"                       # optional: for reading texts (default NRSVue)

[[cue]]
file = "liturgies/hymn-436.ewm"
title = "Processional Hymn"          # optional: setlist label
type = "hymn"                        # optional: overrides the file's type
[cue.sanctuary]                      # optional: this item's look (see backgrounds.md)
background = "backgrounds/advent_window.png"

[[cue]]
announcements = "single_pass"        # or "loop": the Google Slides deck
```

Each cue has either `file` or `announcements`, never both.

To check a file from the terminal:

```bash
cargo run -p episco-core --example ewm_dump -- examples/service.toml
```

## Psalm antiphons

An antiphon is its own section: a `# Antiphon` heading, the music line as an
image, and the words as an `[Antiphon]` paragraph (bold, centred under the
music, no label). Verses use their number as the role; bold verses are the
people's, plain verses the choir's.

```text
# Antiphon
![Antiphon](/media/ps96-antiphon.png)
[Antiphon] **Sing to the Lord a new song.**

# Psalm 96
[1] Sing to the Lord a new song; *
    sing to the Lord, all the whole earth.
[2] **Sing to the Lord and bless his Name; ***
    **proclaim the good news of his salvation from day to day.**
```

## Everyone together: `[All]`

`[All]` marks text said by priest and people together (the Creed, the
Lord's Prayer, the Confession, the Postcommunion Prayer). It is bold like
`[People]` and prints as *All* in bulletins and scripts. (`[All]` used to
be read as `[People]`.)

