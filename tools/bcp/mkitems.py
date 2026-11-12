"""Cut the BCP pages into .ewm library items (drafts for review)."""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from bcp2ewm import parse  # noqa: E402
from ewmout import render  # noqa: E402

B = os.environ.get("BCP_PAGES", os.path.join(os.path.dirname(__file__), "pages"))
OUT = sys.argv[1]

# (file, source page, start marker, end marker (exclusive, None = end),
#  title, type, first speaker, extra front matter, drop markers)
ITEMS = [
    # ---- Holy Baptism (BCP 299-311)
    ("baptism-acclamation", "Baptism/holybaptism.html", "The people standing, the Celebrant says", "The Collect of the Day",
     "Opening Acclamation (Holy Baptism)", "acclamation", "Celebrant", {}, ()),
    ("baptism-presentation", "Baptism/holybaptism.html", "Presentation and Examination", "The Baptismal Covenant",
     "Presentation and Examination of the Candidates", "prayer", "Celebrant", {}, ()),
    ("baptismal-covenant", "Baptism/holybaptism.html", "The Baptismal Covenant", "Prayers for the Candidates",
     "The Baptismal Covenant", "prayer", "Celebrant", {}, ()),
    ("baptism-prayers", "Baptism/holybaptism.html", "Prayers for the Candidates", "Thanksgiving over the Water",
     "Prayers for the Candidates", "prayer", "Celebrant", {}, ()),
    ("baptism-water", "Baptism/holybaptism.html", "Thanksgiving over the Water", "Consecration of the Chrism",
     "Thanksgiving over the Water", "prayer", "Celebrant", {}, ()),
    ("baptism-chrism", "Baptism/holybaptism.html", "Consecration of the Chrism", "=The Baptism",
     "Consecration of the Chrism", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("baptism-the-baptism", "Baptism/holybaptism.html", "=The Baptism", "At Confirmation, Reception, or Reaffirmation",
     "The Baptism", "prayer", "Celebrant", {}, ()),
    ("baptism-confirmation", "Baptism/holybaptism.html", "At Confirmation, Reception, or Reaffirmation", "At the Eucharist",
     "At Confirmation, Reception, or Reaffirmation", "prayer", "Bishop", {"bishop": "true"}, ()),
    # ---- Confirmation (BCP 413-419), for a visitation without baptisms
    ("confirmation-presentation", "PastoralOffices/confirmation.html", "Presentation and Examination", "The Baptismal Covenant",
     "Presentation and Examination of the Candidates (Confirmation)", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("confirmation-prayers", "PastoralOffices/confirmation.html", "Prayers for the Candidates", "For Confirmation",
     "Prayers for the Candidates (Confirmation)", "prayer", "Celebrant", {}, ()),
    ("confirmation", "PastoralOffices/confirmation.html", "For Confirmation", None,
     "Confirmation, Reception, Reaffirmation", "prayer", "Bishop", {"bishop": "true"}, ()),
    # ---- Renewal of Baptismal Vows (BCP 292-294)
    ("renewal-of-vows", "SpecialDays/EasterVigil.html", "The Celebrant may first address the people", "At the Eucharist",
     "The Renewal of Baptismal Vows", "prayer", "Celebrant", {}, ()),
    # ---- Ash Wednesday (BCP 264-269)
    ("ash-wednesday-invitation", "SpecialDays/ashwed.html", "After the Sermon, all stand", "If ashes are to be imposed",
     "Invitation to a Holy Lent", "prayer", "Celebrant", {}, ()),
    ("ash-wednesday-ashes", "SpecialDays/ashwed.html", "If ashes are to be imposed", "The following Psalm",
     "Imposition of Ashes", "prayer", "Celebrant", {}, ()),
    ("litany-of-penitence", "SpecialDays/ashwed.html", "Litany of Penitence", "A deacon or lay reader leading",
     "Litany of Penitence", "prayer", "Celebrant", {}, ()),
    # ---- Palm Sunday (BCP 270-273)
    ("liturgy-of-the-palms", "SpecialDays/palmsunday.html", "The Liturgy of the Palms", "In the absence of a bishop",
     "The Liturgy of the Palms", "prayer", "Celebrant", {}, ()),
    ("passion-announcement", "SpecialDays/palmsunday.html", "The Passion Gospel is announced", "Preface of Holy Week",
     "The Passion Gospel", "gospel", "Deacon", {}, ()),
    # ---- Maundy Thursday (BCP 274-275)
    ("foot-washing", "SpecialDays/thursday.html", "When observed, the ceremony", "The service continues with the Prayers",
     "The Washing of Feet", "anthem", "Celebrant", {}, ()),
    # ---- Good Friday (BCP 276-282)
    ("good-friday-opening", "SpecialDays/friday.html", "On this day the ministers enter in silence", "Old Testament",
     "Good Friday: Opening and Collect", "acclamation", "Celebrant", {}, ()),
    ("good-friday-passion", "SpecialDays/friday.html", "The Passion Gospel is announced", "The Solemn Collects",
     "The Passion Gospel (John)", "gospel", "Deacon", {"citation_text": "John 18:1-19:37"}, ()),
    ("solemn-collects", "SpecialDays/friday.html", "The Solemn Collects", "If desired, a wooden cross",
     "The Solemn Collects", "prayer", "Deacon", {}, ()),
    ("good-friday-anthems", "SpecialDays/friday.html", "If desired, a wooden cross", "The service concludes with the following prayer",
     "Devotions before the Cross", "anthem", "Celebrant", {}, ()),
    ("good-friday-final-prayer", "SpecialDays/friday.html", "The service concludes with the following prayer", None,
     "Good Friday: Concluding Prayer", "prayer", "Celebrant", {}, ()),
    # ---- Easter Vigil (BCP 285-295)
    ("vigil-service-of-light", "SpecialDays/EasterVigil.html", "The Lighting of the Paschal Candle", "Then the Deacon, or other person appointed, standing near",
     "The Service of Light", "prayer", "Celebrant", {}, ()),
    ("exsultet", "SpecialDays/EasterVigil.html", "Then the Deacon, or other person appointed, standing near", "The Liturgy of the Word",
     "The Exsultet", "prayer", "Deacon", {}, ()),
    ("vigil-liturgy-of-the-word", "SpecialDays/EasterVigil.html", "The Liturgy of the Word", "A homily may be preached",
     "The Vigil: Liturgy of the Word", "prayer", "Celebrant", {}, ()),
    ("vigil-first-eucharist", "SpecialDays/EasterVigil.html", "At the Eucharist", "Preface of Easter",
     "The Vigil: At the Eucharist", "acclamation", "Celebrant", {}, ()),
    # ---- Ordinations (BCP 512-551)
    ("ordination-priest-opening", "EpiscopalServices/ordination_of_a_priest.html", "A hymn, psalm, or anthem may be sung.", "The Presentation",
     "Ordination of a Priest: Opening", "acclamation", "Bishop", {"bishop": "true"}, ()),
    ("ordination-priest-presentation", "EpiscopalServices/ordination_of_a_priest.html", "The Presentation", "All kneel, and the Person appointed",
     "Ordination of a Priest: The Presentation", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("ordination-priest-examination", "EpiscopalServices/ordination_of_a_priest.html", "The Examination", "The Consecration of the Priest",
     "Ordination of a Priest: The Examination", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("ordination-priest-consecration", "EpiscopalServices/ordination_of_a_priest.html", "The Consecration of the Priest", "At the Celebration of the Eucharist",
     "Ordination of a Priest: The Consecration", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("ordination-priest-after-communion", "EpiscopalServices/ordination_of_a_priest.html", "After Communion", None,
     "Ordination of a Priest: After Communion", "prayer", "All", {"bishop": "true"}, ()),
    ("ordination-deacon-opening", "EpiscopalServices/ordination_of_a_deacon.html", "A hymn, psalm, or anthem may be sung.", "The Presentation",
     "Ordination of a Deacon: Opening", "acclamation", "Bishop", {"bishop": "true"}, ()),
    ("ordination-deacon-presentation", "EpiscopalServices/ordination_of_a_deacon.html", "The Presentation", "All kneel, and the Person appointed",
     "Ordination of a Deacon: The Presentation", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("ordination-deacon-examination", "EpiscopalServices/ordination_of_a_deacon.html", "The Examination", "The Consecration of the Deacon",
     "Ordination of a Deacon: The Examination", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("ordination-deacon-consecration", "EpiscopalServices/ordination_of_a_deacon.html", "The Consecration of the Deacon", "At the Celebration of the Eucharist",
     "Ordination of a Deacon: The Consecration", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("ordination-deacon-after-communion", "EpiscopalServices/ordination_of_a_deacon.html", "After Communion", None,
     "Ordination of a Deacon: After Communion", "prayer", "All", {"bishop": "true"}, ()),
    ("ordination-bishop-opening", "EpiscopalServices/ordination_of_a_bishop.html", "A hymn, psalm, or anthem may be sung.", "The Presentation",
     "Ordination of a Bishop: Opening", "acclamation", "Bishop", {"bishop": "true"}, ()),
    ("ordination-bishop-presentation", "EpiscopalServices/ordination_of_a_bishop.html", "The Presentation", "All kneel, and the Person appointed",
     "Ordination of a Bishop: The Presentation", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("ordination-bishop-examination", "EpiscopalServices/ordination_of_a_bishop.html", "The Examination", "The Consecration of the Bishop",
     "Ordination of a Bishop: The Examination", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("ordination-bishop-consecration", "EpiscopalServices/ordination_of_a_bishop.html", "The Consecration of the Bishop", "At the Celebration of the Eucharist",
     "Ordination of a Bishop: The Consecration", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("ordination-bishop-after-communion", "EpiscopalServices/ordination_of_a_bishop.html", "After Communion", None,
     "Ordination of a Bishop: After Communion", "prayer", "All", {"bishop": "true"}, ()),
    ("ordination-collect", "EpiscopalServices/ordination_of_a_priest.html", "All kneel, and the Person appointed", "=The Ministry of the Word",
     "Collect after the Litany (Ordinations)", "prayer", "Bishop", {"bishop": "true"}, ()),
    ("litany-for-ordinations", "EpiscopalServices/litany_for_ordinations.html", "The Litany for Ordinations", None,
     "The Litany for Ordinations", "prayer", "Leader", {}, ()),
]


# Hand corrections: (item, the start of a paragraph's text, new speaker).
# The paragraph's bold follows the new speaker.
FIXES = [
    ("baptism-the-baptism", "*N.*, I baptize you", "Celebrant"),
    ("exsultet", "And also with you.", "People"),
    ("exsultet", "It is right to give him thanks and praise.", "People"),
    ("good-friday-anthems", "A Confession of Sin", "Rubric"),
    ("liturgy-of-the-palms", "Almighty God, whose most dear Son", "Celebrant"),
    ("ordination-bishop-after-communion", "Our help is in the Name of the Lord;", "New Bishop"),
    ("ordination-bishop-consecration", "Therefore, Father, make N. a bishop", "Bishops"),
    ("ordination-bishop-presentation", "*N.*, Bishop in the Church of God, the clergy", "Presenters"),
    ("ordination-deacon-presentation", "N.*, Bishop in the Church of God", "Presenters"),
    ("ordination-priest-presentation", "N.*, Bishop in the Church of God", "Presenters"),
    ("solemn-collects", "Let us pray for the holy Catholic Church", "Deacon"),
    ("solemn-collects", "For its unity", "Deacon"),
    ("solemn-collects", "Let us pray for all nations", "Deacon"),
    ("solemn-collects", "For *N.*, the President", "Deacon"),
    ("solemn-collects", "That by God's help", "Deacon"),
    ("solemn-collects", "Let us pray for all who suffer", "Deacon"),
    ("solemn-collects", "Let us pray for all who have not received", "Deacon"),
    ("solemn-collects", "Let us commit ourselves to God", "Deacon"),
]
# Plain text corrections: (item, old, new).
TEXT_FIXES = [
    ("ordination-deacon-presentation", "[Presenters] N.*,", "[Presenters] *N.*,"),
    ("ordination-priest-presentation", "[Presenters] N.*,", "[Presenters] *N.*,"),
]


def apply_fixes(name, text):
    import re
    lines = text.split("\n")
    for item, start, role in FIXES:
        if item != name:
            continue
        for i, l in enumerate(lines):
            m = re.match(r"^\[([^\]]+)\] (\*\*)?(.*)$", l)
            if m and m.group(3).startswith(start):
                j = i + 1
                while j < len(lines) and lines[j].startswith("    "):
                    j += 1
                body = [re.sub(r"^\[[^\]]+\] ", "", lines[i])] + [x[4:] for x in lines[i + 1:j]]
                body = [b[2:-2] if b.startswith("**") and b.endswith("**") else b for b in body]
                if role == "Rubric":
                    new = ["[Rubric] *" + " ".join(b.replace("*", "") for b in body) + "*"]
                else:
                    if role in ("People", "All"):
                        body = ["**" + b.replace("*", "") + "**" for b in body]
                    new = [f"[{role}] " + body[0]] + ["    " + b for b in body[1:]]
                lines[i:j] = new
                break
        else:
            raise SystemExit(f"fix not found: {item}: {start}")
    text = "\n".join(lines)
    for item, old, new in TEXT_FIXES:
        if item == name:
            if old not in text:
                raise SystemExit(f"text fix not found: {item}: {old}")
            text = text.replace(old, new)
    return text


def text_of(e):
    return " ".join(str(x) for x in e[1:] if isinstance(x, str)).replace("\n", " ")


def find(events, marker, start=0):
    for i in range(start, len(events)):
        if marker.startswith("=") and events[i][0] == "h" and events[i][2].replace("\n", " ") == marker[1:]:
            return i
        if not marker.startswith("=") and marker in text_of(events[i]):
            return i
    raise SystemExit(f"marker not found: {marker!r}")


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, src, a, b, title, typ, speaker, extra, drop in ITEMS:
        ev = parse(os.path.join(B, src))
        i = find(ev, a)
        j = find(ev, b, i + 1) if b else len(ev)
        sl = ev[i:j]
        # the first page: content before a page footer N is on page N
        page = next((e[1] for e in ev[i:] if e[0] == "page"), None)
        # the item's own heading is its title: drop a leading heading
        if sl and sl[0][0] == "h":
            sl = sl[1:]
        body, _ = render(sl, speaker=speaker, drop=drop)
        body = apply_fixes(name, body)
        fm = [f"title: {title}", f"type: {typ}", f"citation: BCP {page}" if page else None]
        fm += [f"{k}: {v}" for k, v in extra.items() if k != "citation_text"]
        fm = "\n".join(x for x in fm if x)
        with open(os.path.join(OUT, name + ".ewm"), "w") as f:
            f.write(f"---\n{fm}\n---\n\n{body}")
        print(f"{name}: BCP {page}, {len(sl)} events")


main()
