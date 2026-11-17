Built-in library: texts from the 1979 Book of Common Prayer (public domain),
compiled into the app.

* `*.ewm` — the Holy Eucharist, Rite Two (`rite: II`; items without a `rite`
  key are the same in both rites, e.g. the dismissals).
* `rite1/*.ewm` — the Holy Eucharist, Rite One (`rite: I`), BCP pp. 323-340.
* `prayers-of-the-people-1` … `-6`, `prayers-of-the-people-collects` — the
  Prayers of the People, Forms I–VI and the Collects at the Prayers (BCP
  383-395); `rite1/prayers-of-the-people` — the Rite I prayer (BCP 328).
* `occasions/*.ewm` — Holy Baptism (BCP 299-311), Confirmation (413-419),
  the Renewal of Baptismal Vows (292), the Bishop's Blessing (523), Ash
  Wednesday (264-269), Palm Sunday, Maundy Thursday, Good Friday, Holy
  Saturday and the Great Vigil of Easter (270-295), the anthem "In the midst
  of life" (492), and the Ordinations of a Bishop, Priest and Deacon with the
  Litany for Ordinations (512-551), the Celebration and Blessing of a
  Marriage (423-432) and the Burial of the Dead, Rite One (`burial-1-*`,
  469-489) and Rite Two (`burial-2-*`, 491-505), converted from www.bcponline.org (see
  tools/bcp/). `bishop: true` marks the bishop's parts; `bulletin: full`
  prints them in full in bulletins.
* `bcp/psalter.txt` — the Psalter, one verse per line, from the CC0 XML
  edition at github.com/freeuniversalist/1979psalter.
* `bcp/canticles.txt` — Canticles 1-21 (Morning Prayer I and II).
* `bcp/prefaces.txt` — the Proper Prefaces (BCP pp. 344-349, 377-382), and
  `bcp/preface-days.txt` — which preface each day's collect is appointed
  with; from www.bcponline.org by `tools/bcp/prefaces.py`.
* `bcp/collects.txt` — the Collects of the Church Year, Holy Days and the
  Common of Saints, contemporary (II) and traditional (I).
  Canticles and collects are from the Online BCP source,
  github.com/bcponline/BCP-Source.
* `lectionary/rcl.md` — the parish's Revised Common Lectionary reference
  (Years A, B and C, Tracks 1 and 2, holy days, commons, lesser feasts), with
  the corrections listed at its top.

Items you save from the app go into your own library folder instead; see
docs/library.md. To add a built-in item, put an `.ewm` file here and list it
in `src-tauri/src/library.rs` (BUILTIN).
