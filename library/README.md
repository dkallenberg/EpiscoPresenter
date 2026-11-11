Built-in library: texts from the 1979 Book of Common Prayer (public domain),
compiled into the app.

* `*.ewm` — the Holy Eucharist, Rite Two (`rite: II`; items without a `rite`
  key are the same in both rites, e.g. the dismissals).
* `rite1/*.ewm` — the Holy Eucharist, Rite One (`rite: I`), BCP pp. 323-340.
* `bcp/psalter.txt` — the Psalter, one verse per line, from the CC0 XML
  edition at github.com/freeuniversalist/1979psalter.
* `bcp/canticles.txt` — Canticles 1-21 (Morning Prayer I and II).
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
