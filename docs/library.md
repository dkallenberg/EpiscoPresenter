# Library and propers

## The Library tab

The **Library** tab lists reusable items, which you can filter by type
(Acclamation, Collect, Prayer, Hymn, …) or search by title and text. Selecting
an item previews every slide on both outputs.

In EpiscoPlanner the selected item is shown as the bulletin prints it in
full; **Script** shows it as the officiant's script prints it, and
**Slides** shows its slides on both screens (two pairs to a row). The
parish's own items can be changed right there: click a line, type, and
press Enter (Esc undoes it); the item in the Library changes, not services
already made with it. The Prayer Book's own text is shown, not edited. The
**Library** button on the Services screen opens the Library without a
service open, to add, change or tidy items. (EpiscoPresenter shows the
slides.)

**Edit and add items without a service.** **Edit** (beside one of the
parish's items) opens it in the Editor on its own: words, verses, speaker
rows, a hymn's title, number, book and tune, a picture of the hymn, a
psalm's layout. The top bar says *Library: …* with **Save to Library** and
**Done**. **Done** goes back to the Library (and to the service that was
open, if there was one). A Prayer Book item offers **Edit a parish
version**, which makes the parish's own copy and opens that. **＋ New…**
makes a new hymn (book, number, title) or a new prayer or other text (type,
rite, title) and opens it the same way; **Paste a text…** and **Import
hymns…** are there too.

Services already made keep their own copy of an item. After **Save to
Library**, the upcoming services that use it are listed (a hymn by its
number and book, anything else by its kind and title): tick the ones that
should have the new version and click **Update**. Their name for it, how
it prints and the verses sung stay as they were; a service open on someone
else's computer can't be updated until they close it.

* **Built-in** (**BCP**) items are texts of the Holy Eucharist from the 1979 Book of
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

### Versions: BCP, the parish's copies, the default

Each built-in item is marked **BCP**: the Prayer Book's text, which is
protected. In a service it is shown, not edited: the Editor shows it
read-only with **Make a parish version to edit**, which saves a dated copy
in the Library and switches the item to it, and the bulletin preview does
not let you type into it. Any change has to be made in a parish version.
The parish can keep its own versions of it:

1. Select the item. Under **Versions**, **BCP** is the Prayer Book's text;
   each copy is labelled with the day it was made (*13 Nov 2026*), and the
   one services use is labelled **Default** and starred.
2. **Make a copy** copies the version shown. **Add & edit** puts a copy in
   the open service and opens it in the **Editor**; when you have changed
   it, **Save to Library** asks whether to **update** that copy or **save a
   new** one. Save to Library on any item that came from the Prayer Book
   (added from the Library, or by a new service) saves a new dated copy;
   it never changes the BCP text.
3. **Set as default** makes the version shown the one **new services** use
   (the standard order, Schedule, Holy Week and ordination services), and
   the one added when you double-click the item. Set **BCP** as the default
   to go back to the Prayer Book's text.

In the list, an item whose default is a parish copy shows **Parish
default** and how many copies it has; the copies themselves are listed under
**★ Mine**. Deleting the default copy goes back to the BCP. Services already
made keep the text they were made with, as do service order templates.

The copies are `.ewm` files in the parish library folder (with `based_on`
and `created` in their front matter), and the defaults are in
`defaults.json` there, so every computer using the parish library uses the
same defaults.

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
* **Many at once.** **Library → ＋ New… → Import hymns from files…** takes text files, one per
  hymn, named with the number and title (`436 Lift Up Your Heads.txt`,
  `S 280 Gloria.txt`), and saves each as a hymn in **Mine**. Search the
  Library by number ("Hymn 436") or title, then select the
  hymn slot in the service and click **Use for Entrance Hymn** (or whichever
  slot it is). Double-clicking the hymn does the same when the selected
  slot is still empty; otherwise a double-click adds the hymn after it.

**Verses sung.** A hymn with numbered verses has **Verses sung** in the
Editor, under its title and number: a tick for each verse, all ticked to start. Untick
a verse that is not sung and it is left out of the slides, the bulletin
and the script, with the refrain after it; its words stay in the hymn, so
ticking it brings it back, and **All** ticks them all again. The choice is
this service's only (a hymn from the Library starts with every verse). The
bulletin's line says which verses: *Entrance Hymn — Lift Up Your Heads ·
Hymn 436, vs. 1–3 & 5* (one verse: *v. 2*). It is kept in the service plan
(`verses = [1, 2, 3, 5]`).

**Hymn books.** A hymn's **Book** (in the Editor, beside **Hymn #**) says
which book its number is in: *The Hymnal 1982* (the usual), *Wonder, Love,
and Praise*, *Lift Every Voice and Sing II*, *Voices Found*, *My Heart Sings
Out*, *El Himnario*, **Another book…** (type its name) or **Not from a
book**. The bulletin and slides print the number with its book: *Hymn 436*,
*WLP 787*, *LEVAS 151*; a song from no book has no number. Two hymns with
the same number in different books are different hymns in the Library.
**Import hymns…** reads the book from the start of a file name:
`WLP 787 You Are the Light.txt`, `LEVAS 151 Lift Every Voice.txt`.

While a hymn's place is selected in the service, the Library lists hymns
only (click **All** for everything; selecting another item puts the list
back).

**A hymn and its place are kept apart.** The item in the service is named
for its place (*Entrance Hymn*, *Closing Hymn*); the hymn's own name goes in
**Hymn title** and **Hymn #** in the Editor. Naming the hymn, or filling the
slot from the Library, leaves the slot's name as it was: the Editor's
**Name in the service list** shows it. **Save to Library**
keeps a hymn under its number and title ("hymn-436-lift-up-your-heads"),
never under its place, so the same hymn can open the service one week and
close it the next. A hymn with neither a number nor its own title cannot be
saved to the library until it is given one.

**A picture of the hymn.** The Editor's **Hymn words** has **Picture of the
hymn**: a picture of its music and words (from `music/Hymn`). The bulletin
(in Full text) and the script print the picture in place of the words; the
screens show the words. The script's choice for a hymn is made on the
Script tab, like the bulletin's (see the planner guide).

### Music with its words

Some pictures of music have the words printed in them: the Opening
Acclamation sung, a Sanctus, a Fraction Anthem. Put the picture first on
its slide, the words after it as usual (*[Celebrant] Blessed be God…*,
*[People] And blessed be his kingdom…*), and tick **This music includes the
words after it** under the picture in the Editor. The usual rule is then:

| | Music | Words |
| --- | --- | --- |
| Bulletin | ✓ | — |
| Script | ✓ | — |
| Sanctuary | — | ✓ |
| Stream | ✓ | ✓ |

The words are the text after the picture on its slide (up to the next
heading, slide break or picture); rubrics are always kept. For an item that
should be different, open **Where it shows** in the Editor and tick the
places for the music and for the words; **Back to the usual** clears it.
The choice is saved with the item (`music_on:` and `words_on:` in its front
matter), so a parish version in the Library keeps it. In the `.ewm` file the
picture is marked `{words}`: `![](acclamation.png){words}`.

A psalm's antiphon and the music before the Gospel keep their own layout
(the music line with the words under it on the sanctuary).

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
* **Tag** it: type tags under its name (e.g. *Advent*, *music*, *Christmas
  pageant*; several at once with commas, Enter adds them) and remove one
  with ✕. Tags are kept in the media folder (`media/tags.json`), so every
  computer using the parish library sees them.

The **Tags** chips above the pictures show only those with a tag; the search
finds names and tags. The picture chooser (music lines, slide pictures) has
the same search and tags.

Images in the media library also appear in every background picker.

### Music folders

Pictures of music lines are kept apart from the media library, in the parish
library's `music` folder, one folder for each use:

* `music/Antiphon`: psalm antiphons;
* `music/Setting`: the settings (chants, tones) psalm verses are sung to;
* `music/Gospel Acclamation`: the Alleluia or acclamation before the Gospel;
* `music/Hymn`: pictures of hymns (music and words).

The Editor's **Choose…** for an antiphon, a setting, the music before the
Gospel or a hymn's picture opens that folder: pick a picture, or **Import…** to add new ones to
it. **From the media library…** takes one added there earlier. The folders
are kept in step between computers like the rest of the library.

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
closing response. (The translation is not named after the reading; to
credit it, put a line in **Parish details → Music licence notice**, which prints
in the bulletin.) **Remove text** goes back to the
citation and responses only.

### The Proper Preface

Eucharistic Prayers A and B (Rite I: Prayers I and II) say a Proper
Preface where the rubric "Here a Proper Preface is sung or said…" stands.
The app adds it there on the slides, in the bulletin and in the script (the
prayer's own text is not changed), in the prayer's rite:

* **As appointed for the day** (the default): the preface the Prayer Book
  notes under the day's collect (pp. 159-261), e.g. Advent, Incarnation,
  Epiphany, Lent, Holy Week, Easter, Ascension, Pentecost, Trinity Sunday,
  All Saints, or the Lord's Day on the Sundays after Pentecost. Where the
  book offers a choice the first is used; the Lord's Day prefaces are for
  Sundays only; Good Friday and Holy Saturday have none.
* Holy Baptism and Confirmation use the Baptism preface, ordinations the
  Apostles and Ordinations preface, and the Easter Vigil the Easter preface.
* **⚙ Settings → Service settings → Proper Preface** chooses another (for
  example the Lord's Day "Of God the Son"), or **None**.

Selecting the prayer in the Editor says which preface is used. The texts
(BCP pp. 344-349 and 377-382) and the table of days are in
`library/bcp/prefaces.txt` and `preface-days.txt`, made by
`tools/bcp/prefaces.py` from the Online BCP.

### The Sanctus, said or sung

The Sanctus item in the Editor has **Said** (the people's words in bold,
laid out like a prayer: new services start this way) or **Sung** (shown as
a hymn: words on black in the sanctuary, the hymn panel on the stream), and
its words: **Rite II (BCP 362)**, **Rite I (BCP 334)**, or **A hymnal
setting**: import the setting's words like a hymn (`S 125 Sanctus.txt`),
select it in the Library and click **Use for Sanctus** (it is then sung,
with its number).

Within a whole Eucharistic Prayer (A, B, C, D, I or II), select the prayer
in the Editor: its **Sanctus** row has **Said** (the people's part in bold,
as the rest of the prayer) or **Sung**: the Sanctus is then a slide of its
own on both screens, shown as a hymn (words on black, level lines, no
"People" label, in the sanctuary; the hymn panel on the stream), and the
rest of the prayer keeps its look. The bulletin and script print the prayer
as usual. (The choice is kept in the service plan: `sanctus_sung = true`.)
The parish's own Eucharistic Prayers have the same row: the Sanctus is the
first line that begins "Holy, holy, holy" (any punctuation, whoever it is
marked for, People, All or "Presider and People"; partway through a block
it becomes a block of its own when sung).

### Music before the Gospel

The Gospel's slides run in the order they are used: the music line (if
any), the announcement ("The Holy Gospel of our Lord Jesus Christ according
to …" / "Glory to you, Lord Christ."), the title slide, the text (if shown),
and "The Gospel of the Lord." / "Praise to you, Lord Christ."

Select the Gospel and open the **Editor**. In **Music before the Gospel**,
**Choose…** a picture of the Alleluia or Gospel acclamation from the `music/Gospel Acclamation` folder; it goes on the first slide, laid out like the psalm's antiphon
music (centred in the live stream's bar, as is the citation on the Gospel's
title slide). **Remove** takes it off. The music you choose becomes the
parish's usual one: new services start with it before the Gospel (choose
another to change it). In the bulletin and script it prints before the Gospel's own
title, under the heading *Gospel Acclamation*, with *All stand, as you are
able. The Responsorial Alleluia is introduced by the Cantor and then
repeated by the People.* (In Lent, music whose file name says *Lent* gets
*The Gospel Acclamation is introduced…* instead; a Rubric row of your own on
the music's slide replaces either.) The slides show just the music. Gospels in services made before this order are put
in order when the service opens; **Put the slides in order** does it again
after editing by hand.

### Psalm layout (after importing)

Select the psalm and open the **Editor**. The **Psalm layout** panel has:

| Setting | What it does |
| --- | --- |
| Antiphon music | The music line for the antiphon, from `music/Antiphon`. It is printed in the bulletin and script (without the words) and shown on both screens; the sanctuary shows the words under it. |
| Musical setting | An image of the psalm's setting (chant or tone), from `music/Setting`. It is printed in the bulletin and script under the antiphon, and shown on the live stream between the gold title box and the text bar on every slide of the psalm. |
| Antiphon words | The antiphon's text (sung by all), bold. |
| Antiphon | Where it repeats: *start & end*, *every 2/3/4/6 verses*, or *where placed* (keep the antiphons you put in by hand). |
| Slides | Verses per slide (1–4). |

**Apply** rebuilds the slides from the numbered verses, keeping who sings
each verse.

**On the sanctuary screen** an antiphon's slide has the psalm (*Psalm
78:1-7*) at the top left and **Antiphon** at the right, then the music line
with the words under it (only the words, in the middle, until it has music).
A psalm with an antiphon names itself there; one without names itself at
the top of its first verses.

**In the bulletin and script** a psalm with an antiphon is printed as it is
sung: its title; *The Cantor introduces the antiphon, which is then
repeated by the People.*; the antiphon's music and the setting's; *The
People are invited to join the Choir singing the verses printed in
boldface.*; then the verses, the verse before each antiphon ending with
**Antiphon** at the right margin. Without a picture of the antiphon, its
words are printed once instead.

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

1. **Library → ＋ New… → Paste a text…**
2. Give it a title ("Eucharistic Prayer 1 (EOW)"), a type (Eucharistic
   Prayer, or Prayers of the People), the rite it is for (or Either) and
   where it comes from ("EOW 1, p. 57"; printed on the bulletin).
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

### The parish's own Eucharistic Prayers and Prayers of the People

A Eucharistic Prayer in the library, and a prayer marked as **Prayers of
the People**, are offered with the Prayer Book's when services are made
(**Schedule services**: *Prayers of the People* and *Eucharistic Prayer*,
under "The parish's own") and in **⚙ Settings → Service settings** for the
open service, for services in its rite (or both, if it is for Either).

* To mark a prayer already in the library, select it and tick **Prayers of
  the People** under its name. Pasted as *Prayers of the People*, it is
  marked already.
* A Eucharistic Prayer takes the place of the dialogue (*The Great
  Thanksgiving*) and the separate Sanctus: the prayer has both. Choosing
  *Dialogue and Sanctus only* in Service settings puts them back.
* The Proper Preface goes in where the prayer says "Here a Proper Preface
  is sung or said…". If the pasted prayer has no such line, the app puts
  one after the presider's first words following the people's "It is right
  to give … thanks and praise", as in Prayers A and B (the Editor says so
  when it cannot find the place). The preface is the day's, or the one
  chosen in Service settings, where **Leave out** leaves it out.
