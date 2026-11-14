#!/usr/bin/env python3
"""The Proper Prefaces (BCP pp. 344-349, Rite I; 377-382, Rite II) from the
Online BCP pages, as library/bcp/prefaces.txt.

    curl -s https://www.bcponline.org/HE/pphe1.html -o pphe1.html
    curl -s https://www.bcponline.org/HE/pphe2.html -o pphe2.html
    python3 prefaces.py pphe1.html pphe2.html > ../../library/bcp/prefaces.txt

and which preface each day uses, from the note under each collect
("Preface of the Epiphany, or of the Lord's Day"):

    for p in seasonsc holydaysc commonc variousc; do
      curl -s https://www.bcponline.org/Collects/$p.html -o $p.html; done
    python3 prefaces.py --days seasonsc.html holydaysc.html commonc.html variousc.html \
      > ../../library/bcp/preface-days.txt

Each <p> is a preface's title (in <em>) and text, "or this" (the next one is
another choice), a heading or rubric (skipped), a page footer (skipped), or
the rest of a text the page footer split.
"""

import html
import re
import sys

# Slips in the Online BCP, checked against the printed book.
TEXT_FIXES = {
    "Because thou are greatly glorified": "Because thou art greatly glorified",
}

# The book's titles → the names the app uses.
NAMES = {
    "1. Of God the Father": "The Lord's Day: Of God the Father",
    "2. Of God the Son": "The Lord's Day: Of God the Son",
    "3. Of God the Holy Spirit": "The Lord's Day: Of God the Holy Spirit",
}


def clean(s):
    s = re.sub(r"<br\s*/?>", " ", s)
    s = re.sub(r"<[^>]+>", "", s)
    s = html.unescape(s).replace(" ", " ")
    s = re.sub(r"\s+", " ", s).strip()
    for bad, good in TEXT_FIXES.items():
        s = s.replace(bad, good)
    return s


def read(path):
    """[(title, [text, …alternatives])] in the book's order."""
    page = open(path, encoding="utf-8", errors="replace").read()
    out = []
    alternative = False
    for attrs, body in re.findall(r"<p([^>]*)>(.*?)</p>", page, flags=re.S):
        cls = re.search(r'class="([^"]+)"', attrs)
        cls = cls.group(1) if cls else ""
        if "foot" in cls:
            continue
        if "rubric" in cls:
            alternative = clean(body).lower().startswith("or ")
            continue
        if "<strong>" in body:
            continue
        m = re.match(r"\s*<em>(.*?)</em>(.*)", body, flags=re.S)
        if m:
            title = clean(m.group(1))
            out.append((NAMES.get(title, title), [clean(m.group(2))]))
        elif out and alternative:
            out[-1][1].append(clean(body))
        elif out:
            # The rest of a text split by a page break.
            out[-1][1][-1] = (out[-1][1][-1] + " " + clean(body)).strip()
        alternative = False
    return out


def main(rite1, rite2):
    one, two = read(rite1), read(rite2)
    print("# Proper Prefaces of the 1979 Book of Common Prayer (public domain), pp. 344-349 (I) and 377-382 (II).")
    print("# Source: the Online BCP (www.bcponline.org/HE/pphe1.html, pphe2.html), by tools/bcp/prefaces.py.")
    print("# Format: '== Name' then 'II: preface' (contemporary) and 'I: preface' (traditional);")
    print("# several lines for one rite are alternatives ('or this').")
    titles = [t for t, _ in two] + [t for t, _ in one if t not in dict(two)]
    for title in titles:
        print(f"== {title}")
        for rite, items in (("II", dict(two)), ("I", dict(one))):
            for text in items.get(title, []):
                print(f"{rite}: {text}")


# "Preface of the Epiphany" → the names in prefaces.txt.
PREFACE_NAMES = {
    "the Lord's Day": "The Lord's Day",
    "Advent": "Advent",
    "the Incarnation": "Incarnation",
    "the Epiphany": "Epiphany",
    "Lent": "Lent",
    "Holy Week": "Holy Week",
    "Easter": "Easter",
    "the Ascension": "Ascension",
    "Pentecost": "Pentecost",
    "Trinity Sunday": "Trinity Sunday",
    "All Saints": "All Saints",
    "a Saint": "A Saint",
    "Apostles": "Apostles and Ordinations",
    "the Apostles": "Apostles and Ordinations",
    "Apostles and Ordinations": "Apostles and Ordinations",
    "the Dedication of a Church": "Dedication of a Church",
    "Dedication of a Church": "Dedication of a Church",
    "Baptism": "Baptism",
    "Marriage": "Marriage",
    "the Commemoration of the Dead": "Commemoration of the Dead",
    "Commemoration of the Dead": "Commemoration of the Dead",
    # "Preface of the Season": the season's own.
    "the Season": "Season",
    "Season": "Season",
}


def preface_names(note):
    """"Preface of the Epiphany, or of the Lord's Day" → ["Epiphany", "The Lord's Day"]."""
    # (Propers 8 and 12 have "Preface to the Lord's Day".)
    note = re.sub(r"^Preface\s+(?:of|to)\s+", "", note.strip().rstrip("."))
    out = []
    for part in re.split(r",?\s*or\s+(?:of\s+)?", note):
        part = part.strip()
        name = PREFACE_NAMES.get(part) or PREFACE_NAMES.get(part.removeprefix("the ").strip())
        if not name:
            print(f"? unknown preface: {part!r}", file=sys.stderr)
            continue
        out.append(name)
    return out


def days(paths):
    """'== Day' (as the collect's title) and the prefaces it uses."""
    print("# Which Proper Preface each day uses (the note under its collect, BCP pp. 159-261).")
    print("# Source: the Online BCP collects pages, by tools/bcp/prefaces.py --days.")
    print("# Format: 'Day title => Preface' (names as in prefaces.txt); 'A | B' is a choice, A first.")
    for path in paths:
        page = open(path, encoding="utf-8", errors="replace").read()
        title = None
        for body in re.findall(r"<p[^>]*>(.*?)</p>", page, flags=re.S):
            strong = re.search(r"<strong>(.*?)</strong>", body, flags=re.S)
            if strong:
                title = clean(strong.group(1))
                continue
            text = clean(body)
            if title and re.match(r"^No Proper Preface", text):
                print(f"{title} => None")
                title = None
                continue
            if title and re.match(r"^Preface\s+(of|to)\s", text):
                names = preface_names(text)
                if names:
                    print(f"{title} => {' | '.join(names)}")
                title = None


if __name__ == "__main__":
    if sys.argv[1] == "--days":
        days(sys.argv[2:])
    else:
        main(*sys.argv[1:3])
