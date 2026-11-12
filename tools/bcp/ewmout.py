"""Stage 2: events (from bcp2ewm.parse) → .ewm text with speakers, rubrics,
headings and slides."""
import re

ROLE_WORDS = [
    ("celebrant and people", "All"), ("bishop and people", "All"), ("people and celebrant", "All"),
    ("all", "All"), ("together", "All"),
    ("bishop or priest", "Celebrant"), ("bishop or a priest", "Celebrant"), ("bishop, if present, or the priest", "Celebrant"),
    ("presiding bishop", "Bishop"), ("new priest", "New Priest"), ("new bishop", "New Bishop"), ("new deacon", "New Deacon"),
    ("answer", "Answer"), ("bishop", "Bishop"), ("celebrant", "Celebrant"), ("priest", "Celebrant"),
    ("deacon", "Deacon"), ("reader", "Reader"), ("lector", "Reader"), ("leader", "Leader"),
    ("people", "People"), ("congregation", "People"), ("candidates", "Candidate"), ("candidate", "Candidate"),
    ("sponsors", "Sponsor"), ("sponsor", "Sponsor"), ("parents and godparents", "Parents and Godparents"),
    ("presenters", "Presenters"), ("bishop-elect", "Bishop-elect"), ("ordinand", "Ordinand"),
    ("minister", "Celebrant"), ("officiant", "Celebrant"), ("person appointed", "Leader"),
]
SAY = r"(says|say|saying|asks|continues|addresses|prays|sings|sing|sung|reads|respond|responds|answers|begins|concludes|proceeds|calls|dismisses|leads|invites|words)"

BOLD = {"People", "All"}
RESPONDERS = {"People", "All", "Answer", "Candidate", "Sponsor", "Parents and Godparents", "Presenters"}


def label_role(label, current):
    l = label.strip().rstrip(":").strip()
    low = l.lower()
    if not l:
        return current
    if low == "question":
        return "Q"
    if low == "or":
        return current
    for w, r in ROLE_WORDS:
        if low == w:
            return r
    if " and people" in low or "and the people" in low:
        return "All"
    return l


def speaker_from_rubric(t):
    """The speaker a rubric hands over to, or None."""
    low = t.lower().replace("\n", " ")
    short = low.strip(" .:")
    if len(short.split()) <= 4 and not re.search(SAY, short):
        for w, r in ROLE_WORDS:
            if short == w or short == "the " + w:
                return r, True  # the rubric is just a speaker label
    # the last sentence with a verb of speaking; in it, the speaker nearest
    # before the verb that is not an object ("to the people")
    sentences = [x for x in re.split(r"(?<=[.;])\s+", low) if re.search(r"\b" + SAY + r"\b", x)]
    if not sentences:
        return None, False
    sent = sentences[-1]
    if re.search(r"\b(celebrant|bishop|priest|minister)\b[^.]{0,20}\band (the )?people\b", sent) or re.search(r"\ball\b[^.]*\b(say|sing)\b|\b(say|sing)\b[^.]*\btogether\b|\bin unison\b", sent):
        return "All", False
    v = re.search(r"\b" + SAY + r"\b", sent).start()
    best = None
    words = sorted((w for w, _ in ROLE_WORDS if w not in ("all", "together", "answer")), key=len, reverse=True)
    for m in re.finditer(r"\b(" + "|".join(re.escape(w) for w in words) + r")\b", sent):
        if m.start() >= v:
            break
        before = sent[max(0, m.start() - 22):m.start()]
        if re.search(r"\b(to|of|upon|over|before|from|for|with|asks|addresses|address|invites|greets|by|calls)\b( the| an| a| his| her| their| each)?( \w+)?\s*$", before):
            continue  # an object, not the speaker
        best = dict(ROLE_WORDS)[m.group(1)]
    return best, False


RUBRIC_P = re.compile(r"^(the|then|after|when|here|if|all|each|at|before|in|during|this|a|an|silence|or)\b.*\b" + SAY + r"\b[^.]*[.:]?$", re.I | re.S)


def is_heading_p(t):
    t2 = t.strip("* ")
    if "_[" in t2 or t2.lower().startswith("let us"):
        return False
    return "\n" not in t2 and len(t2.split()) <= 6 and not t2.endswith((".", ",", ";", ":", "?", "!")) and t2[:1].isupper() and not re.search(SAY, t2.lower())


def ital_line(l):
    s = l.strip()
    return s.startswith("*") and s.endswith("*") and s.count("*") == 2 and len(s) > 2


def fmt_lines(lines, bold):
    out = []
    for l in lines:
        l = l.replace("_[", "*").replace("]_", "*").strip()
        if bold:
            # bold the line; italics inside become plain (bold carries it)
            l = "**" + l.replace("*", "") + "**"
        out.append(l)
    return out


class Out:
    def __init__(self):
        self.slides = [[]]
        self.words = 0

    def para(self, role, lines):
        if not lines:
            return
        n = sum(len(l.split()) for l in lines)
        cur = self.slides[-1]
        response = role in BOLD and n <= 14
        if cur and self.words + n > 90 and role != "Rubric" and not response and not (cur and cur[-1][0] == "Rubric"):
            self.slides.append([])
            self.words = 0
        self.slides[-1].append((role, lines))
        self.words += n

    def heading(self, text):
        if self.slides[-1]:
            self.slides.append([])
        self.slides[-1].append(("#", [text]))
        self.words = 0

    def text(self):
        parts = []
        for s in self.slides:
            if not s:
                continue
            b = []
            for role, lines in s:
                if role == "#":
                    b.append(f"# {lines[0]}\n")
                    continue
                if role == "Rubric":
                    t = " ".join(lines).replace("*", "").replace("_[", "").replace("]_", "")
                    t = re.sub(r"\s+", " ", t).strip()
                    b.append(f"[Rubric] *{t}*")
                    continue
                bold = role in BOLD
                ls = fmt_lines(lines, bold)
                head = f"[{role}] " if role else ""
                b.append(head + ls[0] + "".join("\n    " + l for l in ls[1:]))
            parts.append("\n".join(b))
        return "\n\n---\n\n".join(parts) + "\n"


def merge_pages(events):
    """Re-join a paragraph that a page break split in two."""
    out = []
    for e in events:
        if e[0] == "p" and out:
            k = len(out) - 1
            while k >= 0 and out[k][0] == "page":
                k -= 1
            prev = out[k] if k >= 0 else None
            if prev and prev[0] == "p" and k < len(out) - 1 and not re.search(r"([.?!:;\"”*)]|\]_)\s*$", prev[1]):
                out[k] = ("p", prev[1] + "\n" + e[1])
                continue
        out.append(e)
    return out


def unwrap(lines):
    """Join prose hard-wrapped at the page width; keep short (poetry) lines."""
    out = []
    for l in lines:
        prev = out[-1].rstrip() if out else ""
        listy = len(prev) < 46 and l.strip()[:1].isupper()
        if out and (len(prev) >= 52 or (not re.search(r"[.,;:?!*”\"]$", prev) and not listy)) and not l.lstrip().startswith(("*", "_[")):
            out[-1] = out[-1].rstrip() + " " + l.strip()
        else:
            out.append(l)
    return out


def render(events, speaker="Celebrant", skip_vs=True, drop=()):
    events = merge_pages(events)
    out = Out()
    cur = speaker
    asker = speaker
    pages = []
    for e in events:
        k = e[0]
        if k == "page":
            pages.append(e[1])
            continue
        if k == "h":
            out.heading(e[2].replace("\n", " "))
            cur = asker
            continue
        if k == "rubric":
            t = e[1]
            if any(d in t for d in drop):
                continue
            if re.match(r"^\*?(Old Testament|Year A|Psalm|John 18)", t.strip()):
                continue  # lesson citations: the lectionary supplies them
            sp, is_label = speaker_from_rubric(t)
            if sp:
                cur = sp
                if sp not in ("People", "All", "Candidate", "Sponsor", "Parents and Godparents", "Presenters"):
                    asker = sp
            if not is_label:
                out.para("Rubric", [t])
            continue
        if k == "row":
            label, text, cls = e[1], e[2], e[3]
            if cls == "vsnum" and skip_vs:
                continue
            role = label_role(label, cur)
            if role == "Q":
                role = asker
            elif label.strip().lower() == "answer":
                role = "Answer"
            cur = role
            responder = role in RESPONDERS
            if not responder:
                asker = role
            emit_text(out, role, text)
            if responder:
                cur = asker
            continue
        if k == "p":
            t = e[1]
            if any(d in t for d in drop):
                continue
            if t.strip("* ") in ("Preface of Lent", "Preface of Holy Week", "Preface of Baptism", "Preface of Easter"):
                continue
            if RUBRIC_P.match(t.replace("\n", " ")) and len(t) < 300 and "*" not in t:
                sp, _ = speaker_from_rubric(t)
                if sp:
                    cur = sp
                out.para("Rubric", [t])
                continue
            if is_heading_p(t):
                out.heading(t.strip("* "))
                cur = asker
                continue
            emit_text(out, cur, t)
    return out.text(), pages


def fix_italics(text):
    """Pairs *…* markers in order (across lines) and moves spaces and line
    breaks outside them."""
    if text.count("*") % 2 or "_[" in text:
        return text
    parts = text.split("*")
    out = parts[0]
    for k in range(1, len(parts), 2):
        it, after = parts[k], parts[k + 1]
        lead = it[: len(it) - len(it.lstrip())]
        trail = it[len(it.rstrip()):]
        it = it.strip()
        if not it:
            out += lead + trail + after
            continue
        if lead and out and not out[-1:].isspace():
            out += lead
        out += "*" + it + "*"
        if trail and not after[:1].isspace():
            out += trail
        out += after
    return re.sub(r"[ \t]{2,}", " ", out)


def emit_text(out, role, text):
    """A paragraph: italic-only lines are the people's response (or, when
    they read like an instruction, a rubric); a trailing *Amen.* is the people's."""
    text = re.sub(r"\*Amen\*\.", "*Amen.*", text)
    text = fix_italics(text)
    text = re.sub(r"\*Amen\.\s*\n([^*]*?)\*", lambda m: "*Amen.*\n_[" + m.group(1).replace("\n", " ") + "]_", text)
    text = re.sub(r"(?m)^>", "", text)
    # an instruction at the end of a line goes on its own line
    text = re.sub(r"(\S)\s*(_\[[^\]]*\]_)\s*$", r"\1\n\2", text, flags=re.M)
    lines = text.split("\n")
    # a heading line ("Consecration of the Chrism") before the text
    if len(lines) > 1 and is_heading_p(lines[0]) and lines[1].startswith(("_[", "*")):
        out.heading(lines[0].strip("* "))
        lines = lines[1:]
    # an inline instruction _[ ... ]_ (one or more lines)
    merged = []
    k = 0
    while k < len(lines):
        l = lines[k]
        if l.strip().startswith("_[") and "]_" not in l:
            grp = [l]
            while k + 1 < len(lines) and "]_" not in lines[k]:
                k += 1
                grp.append(lines[k])
            l = " ".join(g.strip() for g in grp)
        merged.append(l)
        k += 1
    lines = merged
    buf = []
    def flush(r):
        nonlocal buf
        if buf:
            out.para(r, unwrap(buf))
            buf = []
    i = 0
    while i < len(lines):
        l = lines[i]
        m = re.match(r"^\s*_\[(.*?)\]_\s*(.*)$", l)
        if m:
            flush(role)
            r = m.group(1).strip()
            sp, _ = speaker_from_rubric(r)
            if sp:
                role = sp
            if r:
                out.para("Rubric", [r])
            if m.group(2).strip():
                buf.append(m.group(2).strip())
            i += 1
            continue
        # an italic run spanning several lines: *start ... end*
        if l.strip().startswith("*") and not ital_line(l) and l.strip().count("*") == 1:
            j = i
            group = [l]
            while j + 1 < len(lines) and lines[j].strip().count("*") != 2 and not lines[j].strip().endswith("*") or j == i:
                j += 1
                if j >= len(lines):
                    break
                group.append(lines[j])
                if lines[j].strip().endswith("*"):
                    break
            joined = " ".join(group).strip()
            if joined.endswith("*"):
                flush(role)
                plain = [g.replace("*", "").strip() for g in group]
                if re.search(SAY, joined.lower()) and len(joined) < 200:
                    out.para("Rubric", [" ".join(plain)])
                else:
                    out.para("People", unwrap(plain))
                i = j + 1
                continue
        if ital_line(l):
            flush(role)
            t = l.strip().strip("*").strip()
            if re.search(r"\b" + SAY + r"\b", t.lower()) and not t.lower().startswith(("amen", "thanks")):
                out.para("Rubric", [t])
            else:
                out.para("People", [t])
            i += 1
            continue
        m = re.match(r"^(.*?)\s*\*Amen\.?\*\s*$", l)
        if m and m.group(1).strip():
            buf.append(m.group(1).strip())
            flush(role)
            out.para("People", ["Amen."])
            i += 1
            continue
        buf.append(l)
        i += 1
    flush(role)
