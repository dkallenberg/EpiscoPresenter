"""Turn bcponline.org pages into a stream of events for .ewm items.

Events: ("h", level, text, anchor), ("rubric", text), ("say", role, lines, page),
("page", n). `role` is the speaker (None when unknown). Lines keep <em> as *…*.
"""
import html
import re
import sys
from html.parser import HTMLParser


class Page(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.events = []
        self.stack = []  # open tags with class
        self.buf = None  # (kind, data) collecting text
        self.row = None  # [label, text]
        self.cell = None
        self.anchor = None

    # -- helpers
    def start_buf(self, kind, **kw):
        self.buf = {"kind": kind, "text": "", **kw}

    def end_buf(self):
        b, self.buf = self.buf, None
        return b

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        cls = a.get("class", "") or ""
        if tag == "a" and a.get("name"):
            self.anchor = a["name"]
        if tag in ("p", "h1", "h2", "h3", "h4") and self.buf:
            self.handle_endtag("p" if self.buf["kind"] != "h" else "h%d" % self.buf["level"])
        if tag == "tr" and self.row:
            if self.cell is not None:
                self.handle_endtag("td")
            self.handle_endtag("tr")
        if tag in ("h1", "h2", "h3", "h4"):
            self.start_buf("h", level=int(tag[1]))
        elif tag == "p":
            if cls in ("rightfoot", "leftfoot"):
                self.start_buf("page")
            elif cls in ("topmenu",):
                self.start_buf("skip")
            elif "rubric" in cls or cls in ("small", "smaller", "adddirect"):
                self.start_buf("rubric")
            else:
                self.start_buf("p")
        elif tag == "tr":
            self.row = []
        elif tag == "td":
            if self.row is None:
                self.row = []
            self.cell = {"cls": cls, "text": ""}
        elif tag == "br":
            self.add("\n")
        elif tag == "u":
            # a blank for a name: remember where it starts
            self.u_start = len(self.cell["text"]) if self.cell is not None else len(self.buf["text"]) if self.buf else None
        elif tag in ("em", "i"):
            if not (cls in ("smaller", "small", "rubric")):
                self.add("*")
            else:
                self.add("_[")
        self.stack.append((tag, cls))

    def handle_endtag(self, tag):
        # pop to the matching tag
        while self.stack:
            t, cls = self.stack.pop()
            if t == tag:
                break
        else:
            cls = ""
        if tag in ("em", "i"):
            self.add("*" if cls not in ("smaller", "small", "rubric") else "]_")
        elif tag == "u" and getattr(self, "u_start", None) is not None:
            target = self.cell if self.cell is not None else self.buf
            if target is not None and not target["text"][self.u_start:].replace("\xa0", " ").strip():
                target["text"] = target["text"][: self.u_start] + " *N.* "
            self.u_start = None
        elif tag in ("h1", "h2", "h3", "h4") and self.buf:
            b = self.end_buf()
            t = clean(b["text"])
            if t:
                self.events.append(("h", b["level"], t, self.anchor))
        elif tag == "p" and self.buf:
            b = self.end_buf()
            t = clean(b["text"])
            if not t or b["kind"] == "skip":
                return
            if b["kind"] == "page":
                m = re.search(r"\d+", t)
                if m:
                    self.events.append(("page", int(m.group())))
            elif b["kind"] == "rubric":
                self.events.append(("rubric", t))
            else:
                self.events.append(("p", t))
        elif tag == "td" and self.cell is not None:
            self.row.append(self.cell)
            self.cell = None
        elif tag == "tr" and self.row is not None:
            cells = self.row
            self.row = None
            if len(cells) >= 2:
                label = clean(cells[0]["text"])
                text = clean(" \n".join(c["text"] for c in cells[1:]))
                if text:
                    self.events.append(("row", label, text, cells[0]["cls"]))
            elif len(cells) == 1:
                t = clean(cells[0]["text"])
                if t:
                    kind = "rubric" if "rubric" in cells[0]["cls"] else "p"
                    self.events.append((kind, t))

    def add(self, s):
        if self.cell is not None:
            self.cell["text"] += s
        elif self.buf is not None:
            self.buf["text"] += s

    def handle_data(self, data):
        self.add(data.replace("\n", " "))


def clean(t):
    t = t.replace("\xa0", " ")
    lines = [re.sub(r"[ \t]+", " ", l).strip() for l in t.split("\n")]
    t = "\n".join(l for l in lines if l)
    t = t.replace("**", "").replace("* *", " ")
    t = re.sub(r"\*\s+", "*", t) if False else t
    return t.strip()


def parse(path):
    p = Page()
    p.feed(open(path, encoding="utf-8", errors="replace").read())
    return p.events


if __name__ == "__main__":
    for e in parse(sys.argv[1]):
        print(repr(e)[:160])
