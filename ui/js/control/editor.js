// Center pane, "Editor" and "Source" tabs: the interactive liturgical block
// editor for the selected cue.
//
// The editor works on a local copy of the cue's document and sends it to the
// master (`update_cue_document`) shortly after each change, so every output
// updates live. Edits to the item that is *on air* are held until the
// operator applies them (unless "Hold" is switched off), so half-typed words
// never reach the sanctuary screen.

import { assetUrl } from "/js/bridge.js";
import { domToLines, linesToDom } from "/js/control/richtext.js";
import { saveCueToLibrary } from "/js/control/library.js";
import { lookPanel } from "/js/control/looks.js";
import { pickMedia } from "/js/control/media.js";
import { app, call, debounce, el, reportWarnings, titleCase, toast } from "/js/control/util.js";

const SEND_DELAY_MS = 350;

const ed = {
  id: null, // cue being edited
  doc: null, // local working copy of its document
  dirty: false, // local changes not yet sent
  held: false, // changes withheld because the cue is on air
  hold: true, // operator preference
};

let root, sourceArea, sourceStatus;

// ---------- helpers ----------

/** Stable JSON (sorted keys) so key order never causes a false mismatch. */
function canon(v) {
  if (Array.isArray(v)) return `[${v.map(canon).join(",")}]`;
  if (v && typeof v === "object")
    return `{${Object.keys(v)
      .filter((k) => v[k] !== undefined)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${canon(v[k])}`)
      .join(",")}}`;
  return JSON.stringify(v);
}

const clone = (v) => JSON.parse(JSON.stringify(v));
const cue = () => app.service?.cues.find((c) => c.id === ed.id) ?? null;
const cueIndex = () => app.service?.cues.findIndex((c) => c.id === ed.id) ?? -1;
const isOnAir = () => app.state.active?.cue === cueIndex();
const roleInfo = (role) => app.vocab.roles.find((r) => r.role === role);

function setField(obj, key, value) {
  if (value === "" || value == null) delete obj[key];
  else obj[key] = value;
}

// ---------- sending ----------

const flushSoon = debounce(() => flush(false), SEND_DELAY_MS);

function changed() {
  ed.dirty = true;
  flushSoon();
  renderStatus();
}

/** Sends local changes. `force` applies even to an on-air cue. */
async function flush(force) {
  flushSoon.cancel();
  if (!ed.dirty || !ed.doc) return;
  if (isOnAir() && ed.hold && !force) {
    ed.held = true;
    renderStatus();
    return;
  }
  ed.dirty = false;
  ed.held = false;
  renderStatus();
  await call("update_cue_document", { id: ed.id, document: ed.doc });
}

function discardHeld() {
  ed.dirty = ed.held = false;
  load(cue(), true);
}

// ---------- init & sync with the store ----------

export function initEditor() {
  root = document.getElementById("editor");
  sourceArea = document.getElementById("ewm-source");
  sourceStatus = document.getElementById("source-status");

  document.getElementById("btn-source-apply").addEventListener("click", applySource);
  document.getElementById("btn-source-reload").addEventListener("click", showSource);
  document.querySelector('.tabs button[data-tab="source"]').addEventListener("click", showSource);

  document.addEventListener("keydown", (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      save(e.shiftKey);
    }
  });

  app.on((what) => {
    if (what === "link") return;
    const c = app.selectedCue();
    if (c?.id !== ed.id) return load(c, true);
    if (what === "service") return load(c, false);
    if (what === "state") {
      // The on-air item may have changed: release or hold pending edits.
      if (ed.held && !isOnAir()) flush(false);
      renderStatus();
      refreshLooks(c);
    }
  });
}

/** Shows `c` in the editor. Unless `force`, keeps local edits in progress and
 *  skips the rebuild when the service just echoed our own change. */
function load(c, force) {
  if (!c) {
    ed.id = ed.doc = null;
    ed.dirty = ed.held = false;
    root.replaceChildren(el("p.placeholder", {}, app.service ? "Select an item in the timeline to edit it." : ""));
    return;
  }
  const incoming = c.content.kind === "liturgy" ? c.content.document : null;
  if (!force && c.id === ed.id) {
    refreshLooks(c);
    if (ed.dirty || ed.held) return renderStatus(); // ours is newer
    if (incoming && ed.doc && canon(incoming) === canon(ed.doc)) return renderStatus();
    if (!incoming) return renderStatus();
  }
  // Switching away from a cue with unsent edits: send them first.
  if (force && ed.dirty && ed.id && ed.id !== c.id) flush(true);
  ed.id = c.id;
  ed.doc = incoming ? clone(incoming) : null;
  ed.dirty = ed.held = false;
  render(c);
}

// ---------- rendering ----------

function render(c) {
  const ro = app.readonly;
  root.classList.toggle("readonly", ro);
  const looks = el("div#look-slot");
  if (c.content.kind === "announcements") {
    root.replaceChildren(header(c), announcementsPanel(c), looks);
  } else {
    root.replaceChildren(
      ...[
        header(c),
        el("div#live-banner.live-banner", { hidden: true }),
        looks,
        metaForm(c),
        c.category === "psalm" ? psalmPanel(c) : null,
        ["reading", "gospel"].includes(c.category) ? readingPanel(c) : null,
      c.category === "hymn" ? hymnPanel(c) : null,
        blockList(),
      ].filter(Boolean),
    );
  }
  refreshLooks(c);
  renderStatus();
}

/** Re-renders the backgrounds & layout panel (cheap; keeps the editor intact). */
function refreshLooks(c) {
  const slot = document.getElementById("look-slot");
  if (slot && c) slot.replaceChildren(lookPanel(c));
}

function header(c) {
  return el(
    "div.ed-head",
    {},
    el("div.ed-title", {}, el("strong#ed-title", {}, c.title), el("div#ed-file.muted.small")),
    el("span.spacer"),
    app.readonly
      ? el("span.muted", {}, "Read-only on a slave node")
      : el(
          "div.ed-actions",
          {},
          el(
            "label.toggle",
            { title: "Hold edits to the on-air item until you apply them" },
            el("input", {
              type: "checkbox",
              checked: ed.hold,
              onchange: (e) => {
                ed.hold = e.target.checked;
                if (!ed.hold) flush(true);
              },
            }),
            "Hold on-air edits",
          ),
          c.content.kind === "liturgy" ? el("button", { onclick: () => save(false), title: "⌘S" }, "Save") : null,
          c.content.kind === "liturgy" ? el("button", { onclick: () => save(true), title: "⇧⌘S" }, "Save As…") : null,
          c.content.kind === "liturgy"
            ? el("button", {
                title: "Keep a copy of this item in your library to reuse in other services",
                onclick: async () => { await flush(true); await saveCueToLibrary(cue()); },
              }, "Save to Library")
            : null,
        ),
  );
}

function renderStatus() {
  const c = cue();
  if (!c) return;
  const title = document.getElementById("ed-title");
  const file = document.getElementById("ed-file");
  if (title) title.textContent = c.title;
  if (file) {
    const src = c.content.source;
    file.textContent = c.content.kind === "liturgy" ? (src ?? "Not saved to a file yet") : "Google Slides announcements";
    if (c.modified || ed.dirty || ed.held) file.textContent += " · unsaved changes";
  }
  const banner = document.getElementById("live-banner");
  if (banner) {
    banner.hidden = !ed.held;
    if (ed.held)
      banner.replaceChildren(
        el("span", {}, "This item is on air. Your edits are held and not yet on the screens."),
        el("button.primary", { onclick: () => flush(true) }, "Apply to screens"),
        el("button", { onclick: discardHeld }, "Discard"),
      );
  }
}

// ----- front matter -----

function field(label, input, cls = "") {
  return el(`label.field${cls}`, {}, el("span", {}, label), input);
}

function textInput(key, placeholder) {
  return el("input", {
    value: ed.doc.meta[key] ?? "",
    placeholder: placeholder ?? "",
    disabled: app.readonly,
    oninput: (e) => {
      setField(ed.doc.meta, key, key === "title" ? e.target.value : e.target.value.trim());
      changed();
    },
  });
}

function select(options, value, onchange) {
  return el(
    "select",
    { disabled: app.readonly, onchange: (e) => onchange(e.target.value) },
    options.map(([v, label]) => el("option", { value: v, selected: v === (value ?? "") }, label)),
  );
}

function metaForm(c) {
  const m = ed.doc.meta;
  const v = app.vocab;
  const type = m.type ?? "prayer";
  const fields = [
    field("Title", textInput("title", "Title"), ".wide"),
    field(
      "Type",
      select(
        v.categories.map((x) => [x.key, titleCase(x.key)]),
        type,
        (val) => {
          setField(m, "type", val);
          changed();
          // Show the fields that apply to the new type.
          document.querySelector(".meta-form").replaceWith(metaForm(cue()));
        },
      ),
    ),
    field(
      "Season",
      select([["", "—"], ...v.seasons.map((s) => [s.key, s.label])], m.season, (val) => {
        setField(m, "season", val);
        changed();
      }),
    ),
    field(
      "Colour",
      select([["", "Auto (season / service)"], ...v.colors.map((x) => [x.key, x.key])], m.color, (val) => {
        setField(m, "color", val);
        changed();
      }),
    ),
  ];
  if (["reading", "psalm", "gospel", "collect", "other"].includes(type)) fields.push(field("Citation", textInput("citation", "e.g. Joshua 24:1-3a")));
  if (type === "hymn") fields.push(field("Hymn #", textInput("hymn_number", "e.g. 436")), field("Tune", textInput("tune")));

  // Plan-level overrides live on the cue, not in the .ewm file.
  const o = c.overrides ?? {};
  const setOverride = debounce((patch) => call("set_cue_overrides", { id: c.id, overrides: { ...o, ...patch } }), SEND_DELAY_MS);
  const planRow = el(
    "div.plan-row",
    {},
    field(
      "Timeline label",
      el("input", {
        value: o.title ?? "",
        placeholder: "Same as title",
        disabled: app.readonly,
        oninput: (e) => setOverride({ title: e.target.value.trim() || undefined }),
      }),
    ),
    o.category
      ? el(
          "span.note",
          {},
          `Service plan sets type: ${titleCase(o.category)} `,
          el("button.link", { disabled: app.readonly, onclick: () => setOverride.flush({ category: undefined }) }, "use file’s type"),
        )
      : null,
    el("span.note.muted", {}, `Lower third: “${c.caption}”`),
  );
  return el("div.meta-form", {}, el("div.meta-grid", {}, fields), planRow);
}

// ----- psalm layout -----

const extraOf = (doc, key) => doc.meta.extra?.find(([k]) => k === key)?.[1];
const isPsalm = () => (ed.doc?.meta.type ?? "") === "psalm";
const isAntiphonHeading = (b) => b.kind === "heading" && /^antiphon$/i.test(b.text ?? "");
const isVerse = (b) => b.kind === "paragraph" && /^\d/.test(b.role ?? "");
const spansOf = (b) => (b.lines ?? []).flat().filter((x) => !["", "*"].includes(x.text.trim()));
const isPeopleVerse = (b) => spansOf(b).length > 0 && spansOf(b).every((x) => x.bold);

function setExtra(key, value) {
  const m = ed.doc.meta;
  m.extra = (m.extra ?? []).filter(([k]) => k !== key);
  if (value) m.extra.push([key, value]);
}

/** The words of the first antiphon (lines joined by " / "). */
function firstAntiphonText() {
  const b = ed.doc.blocks.find((x) => x.kind === "paragraph" && /^antiphon$/i.test(x.role ?? ""));
  return b ? b.lines.map((l) => l.map((x) => x.text).join("")).join(" / ") : extraOf(ed.doc, "antiphon") ?? "";
}

/** A small image chooser row: thumbnail, Choose…, Remove. */
function imageField(label, src, onPick, hint) {
  const img = el("img.score-thumb", { alt: "" });
  img.hidden = !src;
  if (src) img.src = assetUrl(cue(), src);
  img.onerror = () => (img.hidden = true);
  return el(
    "div.image-field",
    {},
    el("span.image-field-label", {}, label),
    img,
    src ? null : el("span.muted.small", {}, hint),
    app.readonly
      ? null
      : el("button.small", {
          onclick: async () => {
            const path = await pickMedia({ kind: "image", relativeTo: cue()?.content.source ?? null });
            if (path) onPick(path);
          },
        }, src ? "Change…" : "Choose…"),
    src && !app.readonly ? el("button.small", { onclick: () => onPick(null) }, "Remove") : null,
  );
}

/** Antiphon, its music, the stream's musical setting, verses per slide;
 *  Apply rebuilds the slides. */
function psalmPanel(c) {
  const verses = ed.doc.blocks.filter(isVerse).length;
  const placedNow = extraOf(ed.doc, "antiphon_placed") === "true";
  const antiphon = el("input", { value: firstAntiphonText(), placeholder: "Antiphon (sung by all), or leave empty", disabled: app.readonly });
  const every = el(
    "select",
    { disabled: app.readonly },
    [["placed", "Where placed"], [0, "Start & end"], [2, "+ every 2 verses"], [3, "+ every 3 verses"], [4, "+ every 4 verses"], [6, "+ every 6 verses"]].map(([v, l]) =>
      el("option", { value: v, selected: placedNow ? v === "placed" : String(v) === (extraOf(ed.doc, "antiphon_every") ?? "0") }, l),
    ),
  );
  const per = el(
    "select",
    { disabled: app.readonly },
    [1, 2, 3, 4].map((n) => el("option", { value: n, selected: String(n) === (extraOf(ed.doc, "verses_per_slide") ?? "2") }, `${n} verse${n > 1 ? "s" : ""} per slide`)),
  );
  const paste = el("textarea.psalm-paste", {
    placeholder: verses
      ? "To replace the verses, paste the psalm here (lines starting 1, 2, 3 …)."
      : "Paste the psalm's verses here, e.g. copied from the lectionary page:\n1 To you I lift up my eyes, *\n    to you enthroned in the heavens.\n2 As the eyes of servants …",
    disabled: app.readonly,
  });
  const apply = async () => {
    await flush(true);
    const placed = every.value === "placed";
    const n = await call("psalm_arrange", {
      id: c.id,
      options: {
        antiphon: antiphon.value.trim() || null,
        antiphon_every: placed ? 0 : Number(every.value),
        placed,
        verses_per_slide: Number(per.value),
      },
      versesText: paste.value.trim() || null,
    });
    if (n) toast(`Arranged ${n} verses`);
  };

  const images = ed.doc.blocks.filter((b) => b.kind === "image");
  const antiphonSrc = images.find((b) => b.src)?.src ?? "";
  const setAntiphonMusic = (path) => {
    const blocks = ed.doc.blocks;
    if (images.length) {
      // Every copy of the antiphon shows the same music line.
      for (const b of images) if (!b.src || b.src === antiphonSrc) b.src = path ?? "";
      if (!path) ed.doc.blocks = blocks.filter((b) => !(b.kind === "image" && !b.src));
    } else if (path) {
      for (let i = blocks.length - 1; i >= 0; i--) {
        if (isAntiphonHeading(blocks[i])) blocks.splice(i + 1, 0, { kind: "image", alt: "Antiphon", src: path });
      }
      if (!blocks.some(isAntiphonHeading)) toast("Add an antiphon first (type it above and Apply, or use ♫ Antiphon).", "warn");
    }
    changed();
    document.querySelector(".psalm-section")?.replaceWith(psalmPanel(cue()));
    renderBlocks();
  };
  const setting = extraOf(ed.doc, "setting_image") ?? "";
  const setSetting = (path) => {
    setExtra("setting_image", path);
    changed();
    document.querySelector(".psalm-section")?.replaceWith(psalmPanel(cue()));
  };

  return el(
    "details.section.psalm-section",
    { open: true },
    el(
      "summary",
      {},
      "Psalm layout",
      el("span.muted.small", {}, verses ? ` · ${verses} verses` : " · no verses yet"),
    ),
    verses ? null : el("p.warn-note", {}, "This psalm has no verses yet. Paste them below and click Apply."),
    el(
      "div.psalm-images",
      {},
      imageField("Antiphon music", antiphonSrc, setAntiphonMusic, "The music line for the antiphon (both screens)."),
      imageField("Musical setting (stream)", setting, setSetting, "Shown on the stream between the gold title box and the text bar."),
    ),
    paste,
    el(
      "div.psalm-panel",
      {},
      field("Antiphon words", antiphon),
      field("Antiphon", every),
      field("Slides", per),
      el("button.primary", { disabled: app.readonly, onclick: apply }, "Apply"),
    ),
    el(
      "p.muted.small",
      {},
      "Apply rebuilds the slides from the verses, keeping who sings each verse. Or place antiphons by hand: ♫ Antiphon between verses copies the first antiphon there. Choose Choir or People beside each verse; the people’s verses are bold.",
    ),
  );
}

// ----- hymn words -----

/** A hymn's words, pasted from the parish's licensed source (the app ships
 *  no hymn texts), with its credits and licence notice. */
function hymnPanel(c) {
  const stanzas = ed.doc.blocks.filter((b) => b.kind === "paragraph" && b.role !== "Rubric").length;
  const credit = (key, label, placeholder) =>
    field(
      label,
      el("input", {
        value: extraOf(ed.doc, key) ?? "",
        placeholder,
        disabled: app.readonly,
        oninput: (e) => {
          setExtra(key, e.target.value.trim());
          changed();
        },
      }),
    );
  const paste = el("textarea", {
    placeholder:
      "Paste the hymn's words here, from your licensed source (hymnal PDF, OneLicense, CCLI …).\n1 First stanza…\n\nRefrain:\n…\n\n2 Second stanza…\n\nNumbered stanzas, a “Refrain:” and blank lines between stanzas are recognised; Words:/Music: lines and copyright notices fill the credits.",
    disabled: app.readonly,
  });
  const repeat = el("input", { type: "checkbox", disabled: app.readonly });
  const apply = async () => {
    if (!paste.value.trim()) return toast("Paste the words first.", "warn");
    await flush(true);
    const n = await call("hymn_set_text", {
      id: c.id,
      text: paste.value,
      options: { refrain_after_each: repeat.checked, max_lines: 8 },
    });
    if (n) toast(`Hymn laid out on ${n} slide${n === 1 ? "" : "s"}`);
  };
  return el(
    "details.section.reading-panel.hymn-panel",
    { open: stanzas <= 1 },
    el("summary", {}, "Hymn words", el("span.muted.small", {}, stanzas > 1 ? ` · ${stanzas} stanzas` : " · not entered yet")),
    paste,
    el(
      "div.reading-row",
      {},
      el("label.toggle", {}, repeat, "Repeat the refrain after every stanza"),
      el("button.primary", { disabled: app.readonly, onclick: apply }, "Apply"),
    ),
    el(
      "div.reading-row",
      {},
      credit("words", "Words", "Author, translator"),
      credit("music", "Music", "Tune, composer"),
      credit("license", "Licence notice", "e.g. Reprinted under OneLicense #A-…"),
    ),
    el(
      "p.muted.small",
      {},
      "Hymn texts are mostly under copyright, so the app does not include the Hymnal 1982. Use words your parish is licensed to project. To bring in many at once: Library → Import hymns… (text files named like “436 Lift Up Your Heads.txt”).",
    ),
  );
}

// ----- reading text -----

/** The text of a reading from the NRSV or NRSVue, pasted from a licensed
 *  source (the app cannot include it), laid out as slides. */
function readingPanel(c) {
  const citation = ed.doc.meta.citation ?? "";
  const textBlocks = ed.doc.blocks.filter((b) => b.kind === "paragraph" && !b.role).length;
  const current = extraOf(ed.doc, "bible");
  const version = el(
    "select",
    { disabled: app.readonly },
    [["NRSVue", "NRSVue (Updated Edition, 2021)"], ["NRSV", "NRSV (1989)"]].map(([v, l]) =>
      el("option", { value: v, selected: v === (current ?? app.service?.bible ?? "NRSVue") }, l),
    ),
  );
  const words = el(
    "select",
    { disabled: app.readonly },
    [[30, "Short slides (~30 words)"], [40, "~40 words a slide"], [55, "~55 words a slide"], [70, "Long slides (~70 words)"]].map(([v, l]) =>
      el("option", { value: v, selected: v === 40 }, l),
    ),
  );
  const paste = el("textarea", {
    placeholder: citation
      ? `Paste ${citation} here, copied from the page that opens with “Open ${citation}”. Verse numbers, headings, footnotes and the copyright notice are removed.`
      : "Set the citation above first.",
    disabled: app.readonly,
  });
  const apply = async (text) => {
    await flush(true);
    const n = await call("reading_set_text", { id: c.id, text, version: version.value, words: Number(words.value) });
    if (n != null) toast(text ? `Reading text set: ${n} slide${n === 1 ? "" : "s"}` : "Reading text removed");
  };
  return el(
    "details.section.reading-panel",
    { open: !textBlocks },
    el("summary", {}, "Reading text", el("span.muted.small", {}, textBlocks ? ` · ${textBlocks} slides (${current ?? "?"})` : " · citation and responses only")),
    el(
      "div.reading-row",
      {},
      field("Bible", version),
      el("button", {
        disabled: !citation,
        title: "Opens the passage in your browser: Bible Gateway (NRSVue) or the Oremus Bible Browser (NRSV)",
        onclick: () => call("open_passage", { citation, version: version.value }),
      }, citation ? `Open ${citation} ↗` : "Open passage ↗"),
      field("Slides", words),
    ),
    paste,
    el(
      "div.reading-row",
      {},
      el("button.primary", { disabled: app.readonly, onclick: () => paste.value.trim() ? apply(paste.value) : toast("Paste the reading first.", "warn") }, "Apply"),
      textBlocks ? el("button", { disabled: app.readonly, onclick: () => apply("") }, "Remove text") : null,
    ),
    el(
      "p.muted.small",
      {},
      "The NRSV and NRSVue are copyright the National Council of Churches, so the app cannot include their text. Copy the passage from a licensed source and paste it here; the version's initials are added after the reading, as the NCC asks for service use.",
    ),
  );
}

/** Inserts a copy of the psalm's first antiphon (music + words) at `index`. */
function insertAntiphon(index) {
  const blocks = ed.doc.blocks;
  const start = blocks.findIndex(isAntiphonHeading);
  let group;
  if (start >= 0) {
    let end = start + 1;
    while (end < blocks.length && blocks[end].kind !== "heading" && blocks[end].kind !== "slide_break") end++;
    group = clone(blocks.slice(start, end));
  } else {
    group = [
      { kind: "heading", level: 1, text: "Antiphon" },
      { kind: "image", alt: "Antiphon", src: "" },
      { kind: "paragraph", role: "Antiphon", lines: [] },
    ];
  }
  // The antiphon is its own slide: drop a slide break here, and start the
  // psalm's section again if verses follow.
  if (blocks[index]?.kind === "slide_break") blocks.splice(index, 1);
  if (blocks[index - 1]?.kind === "slide_break") {
    blocks.splice(index - 1, 1);
    index -= 1;
  }
  const next = blocks[index];
  if (next && next.kind !== "heading") group.push({ kind: "heading", level: 1, text: ed.doc.meta.citation || ed.doc.meta.title });
  blocks.splice(index, 0, ...group);
  changed();
  renderBlocks(undefined, index);
}

// ----- blocks -----

function blockList() {
  const list = el("div#blocks.blocks");
  renderBlocks(list);
  return list;
}

function renderBlocks(list = document.getElementById("blocks"), focusIndex = null) {
  const blocks = ed.doc.blocks;
  const children = [divider(0)];
  let paraCount = 0;
  blocks.forEach((b, i) => {
    children.push(blockRow(b, i, b.kind === "paragraph" ? paraCount++ : 0), divider(i + 1));
  });
  if (!blocks.length) children.push(el("p.placeholder", {}, "No blocks yet. Use ( + ADD BLOCK ) to start."));
  list.replaceChildren(...children);
  if (focusIndex != null) focusBlock(focusIndex);
}

function focusBlock(i, atEnd = false) {
  const row = document.querySelector(`#blocks .brow[data-index="${i}"]`);
  const target = row?.querySelector("[contenteditable], input");
  if (!target) return;
  target.focus();
  if (atEnd && target.isContentEditable) {
    const r = document.createRange();
    r.selectNodeContents(target);
    r.collapse(false);
    const sel = getSelection();
    sel.removeAllRanges();
    sel.addRange(r);
  }
}

/** Role for a newly added speaker block: alternate with the previous one. */
function suggestRole(index) {
  const before = ed.doc.blocks.slice(0, index).filter((b) => b.kind === "paragraph" && b.role);
  const last = before[before.length - 1];
  if (!last) return "Celebrant";
  if (last.role !== "People") return "People";
  const speaker = [...before].reverse().find((b) => b.role !== "People" && b.role !== "Rubric");
  return speaker?.role ?? "Celebrant";
}

function insertBlock(index, block) {
  ed.doc.blocks.splice(index, 0, block);
  changed();
  renderBlocks(undefined, index);
}

function divider(index) {
  if (app.readonly) return el("div.divider.ro");
  return el(
    "div.divider",
    {},
    el(
      "button.add-block",
      { onclick: () => insertBlock(index, { kind: "paragraph", role: suggestRole(index), lines: [] }) },
      "( + ADD BLOCK )",
    ),
    el(
      "span.add-more",
      {},
      el("button", { onclick: () => insertBlock(index, { kind: "heading", level: 2, text: "" }), title: "Section heading (starts a slide)" }, "§ Heading"),
      el("button", { onclick: () => insertBlock(index, { kind: "slide_break" }), title: "Start a new slide here" }, "⎯ Slide break"),
      el("button", { onclick: () => insertBlock(index, { kind: "image", alt: "", src: "" }), title: "Sheet-music snippet" }, "♪ Music"),
      isPsalm()
        ? el("button.antiphon-btn", { onclick: () => insertAntiphon(index), title: "Insert a copy of the first antiphon here" }, "♫ Antiphon")
        : null,
    ),
  );
}

function rowTools(i) {
  if (app.readonly) return null;
  const n = ed.doc.blocks.length;
  const move = (to) => {
    const [b] = ed.doc.blocks.splice(i, 1);
    ed.doc.blocks.splice(to, 0, b);
    changed();
    renderBlocks(undefined, to);
  };
  return el(
    "div.brow-tools",
    {},
    el("button.icon", { title: "Move up", disabled: i === 0, onclick: () => move(i - 1) }, "↑"),
    el("button.icon", { title: "Move down", disabled: i === n - 1, onclick: () => move(i + 1) }, "↓"),
    el(
      "button.icon.danger",
      {
        title: "Delete block",
        onclick: () => {
          ed.doc.blocks.splice(i, 1);
          changed();
          renderBlocks(undefined, Math.max(0, i - 1));
        },
      },
      "✕",
    ),
  );
}

function applyRoleStyle(row, role) {
  const info = role ? roleInfo(role) : null;
  row.dataset.role = role ?? "";
  const antiphon = /^antiphon$/i.test(role ?? "");
  row.style.setProperty("--bar", info?.style.editor_bar_color ?? (antiphon ? "#efe3c0" : "#f2f2f2"));
  row.style.setProperty("--accent", info?.style.editor_accent_color ?? (antiphon ? "#9a7a1c" : "#9a9a9a"));
  row.classList.toggle("people", role === "People");
  row.classList.toggle("rubric", role === "Rubric");
}

function blockRow(b, i, paraIndex) {
  const row = el(`div.brow.brow-${b.kind}`, { dataset: { index: i } });
  if (b.kind === "paragraph") {
    row.classList.add(paraIndex % 2 ? "odd" : "even");
    applyRoleStyle(row, b.role);
    const roles = app.vocab.roles.map((r) => [r.role, r.role]);
    if (isPsalm()) roles.unshift(["Antiphon", "Antiphon"]);
    row.classList.toggle("antiphon", /^antiphon$/i.test(b.role ?? ""));
    if (b.role && !roles.some(([r]) => r === b.role)) {
      // A number is a verse number (psalms, hymns), not a speaker.
      roles.push([b.role, /^\d/.test(b.role) ? `Verse ${b.role}` : `${b.role} (custom)`]);
    }
    const roleSel = select([["", "— Text —"], ...roles], b.role, (val) => {
      setField(b, "role", val);
      applyRoleStyle(row, b.role);
      row.classList.toggle("antiphon", /^antiphon$/i.test(b.role ?? ""));
      changed();
    });
    roleSel.className = "role-select";

    // A verse is sung by the choir (plain) or the people (bold).
    let who = null;
    if (isVerse(b)) {
      row.classList.toggle("verse-people", isPeopleVerse(b));
      who = select([["choir", "Choir"], ["people", "People"]], isPeopleVerse(b) ? "people" : "choir", (val) => {
        const bold = val === "people";
        b.lines = b.lines.map((l) => l.map((x) => (bold ? { ...x, bold: true } : (({ bold: _b, ...rest }) => rest)(x))));
        linesToDom(text, b.lines);
        row.classList.toggle("verse-people", bold);
        changed();
      });
      who.className = "who-select";
      who.title = "Who sings this verse: the people's verses are bold";
    }

    const text = el("div.btext", {
      contentEditable: app.readonly ? "false" : "true",
      spellcheck: true,
      oninput: () => {
        b.lines = domToLines(text);
        changed();
      },
      onpaste: (e) => {
        e.preventDefault();
        document.execCommand("insertText", false, e.clipboardData.getData("text/plain"));
      },
      onkeydown: (e) => blockKeys(e, b, i, text),
    });
    linesToDom(text, b.lines);
    row.append(el("div.brow-side", {}, roleSel, who), text, rowTools(i));
  } else if (b.kind === "heading") {
    row.classList.toggle("antiphon-head", isAntiphonHeading(b));
    row.append(
      el("div.brow-side", {}, el("span.kind-label", {}, isAntiphonHeading(b) ? "♫ Antiphon" : "Section")),
      el("input.heading-input", {
        value: b.text,
        placeholder: "Heading (starts a new slide)",
        disabled: app.readonly,
        oninput: (e) => {
          b.text = e.target.value;
          changed();
        },
      }),
      rowTools(i),
    );
  } else if (b.kind === "image") {
    const preview = el("img.score-thumb", { alt: "" });
    const refresh = () => {
      preview.hidden = !b.src;
      if (b.src) preview.src = assetUrl(cue(), b.src);
    };
    preview.onerror = () => (preview.hidden = true);
    refresh();
    const src = el("input", {
      value: b.src,
      placeholder: "scores/hymn-436-refrain.png",
      disabled: app.readonly,
      oninput: (e) => {
        b.src = e.target.value.trim();
        refresh();
        changed();
      },
    });
    row.append(
      el("div.brow-side", {}, el("span.kind-label", {}, "♪ Music")),
      el(
        "div.image-fields",
        {},
        el("div.image-inputs", {}, src, app.readonly ? null : el("button", {
          onclick: async () => {
            const path = await pickMedia({ kind: "image", relativeTo: cue()?.content.source ?? null });
            if (path) {
              b.src = src.value = path;
              refresh();
              changed();
            }
          },
        }, "Choose…")),
        el("input", {
          value: b.alt,
          placeholder: "Caption, e.g. Hymn 436, refrain",
          disabled: app.readonly,
          oninput: (e) => {
            b.alt = e.target.value;
            changed();
          },
        }),
        preview,
      ),
      rowTools(i),
    );
  } else {
    row.append(el("div.brow-side", {}, el("span.kind-label", {}, "Slide")), el("div.break-line", {}, "new slide"), rowTools(i));
  }
  return row;
}

function blockKeys(e, b, i, text) {
  const mod = e.metaKey || e.ctrlKey;
  if (mod && (e.key === "b" || e.key === "i")) {
    e.preventDefault();
    document.execCommand(e.key === "b" ? "bold" : "italic");
  } else if (mod && e.key === "Enter") {
    // New speaker block after this one.
    e.preventDefault();
    insertBlock(i + 1, { kind: "paragraph", role: suggestRole(i + 1), lines: [] });
  } else if (e.key === "Backspace" && !text.textContent.trim() && b.lines.length === 0) {
    // Backspace in an empty block removes it.
    e.preventDefault();
    ed.doc.blocks.splice(i, 1);
    changed();
    renderBlocks();
    focusBlock(Math.max(0, i - 1), true);
  }
}

// ----- announcements -----

function announcementsPanel(c) {
  const deck = app.service?.announcements_deck;
  return el(
    "div.ann-panel",
    {},
    field(
      "Mode",
      select(
        [
          ["single_pass", "Single pass · manual advance, quick exit to the next cue"],
          ["loop", "Loop · auto-advance every 9 seconds"],
        ],
        c.content.mode,
        (mode) => call("set_announcement_mode", { id: c.id, mode }),
      ),
    ),
    el(
      "p.muted",
      {},
      !deck
        ? "No announcements deck is set for this service. Add its Google Slides link in ⚙ Service."
        : c.content.mode === "loop"
          ? "The deck loops on its own, a slide every 9 seconds, until you move on."
          : (c.content.slides ?? 1) > 1
            ? `Next / Prev step through the deck’s ${c.content.slides} slides, then on to the next item.`
            : "The deck plays through once, a slide every 9 seconds. Set “Slides in the deck” in ⚙ Service to step through it with Next / Prev instead.",
    ),
    deck ? el("p.muted.small", {}, `Deck: ${deck}`) : null,
    el("button.small", { onclick: () => document.getElementById("btn-settings").click() }, "⚙ Deck settings…"),
  );
}

// ----- saving -----

async function save(saveAs) {
  const c = cue();
  if (!c || c.content.kind !== "liturgy" || app.readonly) return;
  await flush(true); // saving writes what is live, so apply held edits first
  const path = await call("save_cue", { id: c.id, saveAs });
  if (path) toast(`Saved ${path}`);
}

// ---------- Source tab ----------

async function showSource() {
  if (!ed.doc) {
    sourceArea.value = "";
    sourceArea.disabled = true;
    sourceStatus.textContent = "Select a text item in the timeline.";
    return;
  }
  sourceArea.disabled = app.readonly;
  sourceArea.value = (await call("format_ewm", { document: ed.doc })) ?? "";
  sourceStatus.textContent = "";
}

async function applySource() {
  if (!ed.doc || app.readonly) return;
  const parsed = await call("parse_ewm", { source: sourceArea.value });
  if (!parsed) return;
  sourceStatus.textContent = parsed.warnings.length
    ? parsed.warnings.map((w) => `line ${w.line}: ${w.message}`).join(" · ")
    : `${parsed.document.blocks.length} blocks · OK`;
  reportWarnings(parsed.warnings.map((w) => `line ${w.line}: ${w.message}`));
  ed.doc = parsed.document;
  ed.dirty = true;
  render(cue());
  await flush(true);
}
