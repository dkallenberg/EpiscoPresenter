# Prayer Book pages → library items

How `library/occasions/*.ewm` were made (Holy Baptism, Confirmation, the
Renewal of Vows, Ash Wednesday, Holy Week, the Easter Vigil, ordinations,
Marriage and Burial),
from the 1979 Book of Common Prayer (public domain) at www.bcponline.org.

1. Download the pages, keeping the site's folders, into `pages/`
   (e.g. `pages/Baptism/holybaptism.html`, `pages/SpecialDays/friday.html`,
   `pages/EpiscopalServices/ordination_of_a_priest.html`,
   `pages/PastoralOffices/confirmation.html`).
2. `python3 mkitems.py OUT_DIR` writes one `.ewm` per item listed in
   `ITEMS` (page, first and last marker, title, type, first speaker).

* `bcp2ewm.py` reads a page into events: headings, rubrics, speaker rows,
  paragraphs and page numbers (the page footers give each item its
  `citation: BCP nnn`).
* `ewmout.py` turns events into `.ewm`: the speaker named in each rubric
  ("the Bishop says"), italic lines as the people's responses, wrapped prose
  joined, slides of about 90 words with responses kept beside their versicle.
* `FIXES` / `TEXT_FIXES` in `mkitems.py` are hand corrections where the
  rules pick the wrong speaker.

Then, by hand: `bulletin: full` in each item's front matter (the
congregation needs these texts printed), `type: prayer` for the two Passion
announcements, and `episcopal-blessing.ewm` (BCP 523) and
`in-the-midst-of-life.ewm` (BCP 492). The Marriage and Burial drafts
(`PastoralOffices/marriage.html`, `BurialI.html`, `BurialII.html`) were
largely rewritten by hand: speakers (Man, Woman, Giver), the responses the
page leaves out ("I will."), the Lord's Prayer, anthems set as lines, and
`burial-1-creed.ewm` (the Rite I Apostles' Creed, BCP 53). Check every item against the printed
book before relying on it; the tests only check that they parse.
