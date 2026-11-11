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
      header(c),
      el("div#live-banner.live-banner", { hidden: true }),
      looks,
      metaForm(c),
      c.category === "psalm" ? psalmPanel(c) : null,
      blockList(),
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

/** Antiphon and verses-per-slide for a psalm; rebuilds its slides. */
function psalmPanel(c) {
  const verses = ed.doc.blocks.filter((b) => b.kind === "paragraph" && /^\d/.test(b.role ?? "")).length;
  const antiphon = el("input", { value: extraOf(ed.doc, "antiphon") ?? "", placeholder: "Antiphon (sung by all), or leave empty", disabled: app.readonly });
  const every = el(
    "select",
    { disabled: app.readonly },
    [[0, "Start & end"], [2, "+ every 2 verses"], [4, "+ every 4 verses"], [6, "+ every 6 verses"]].map(([v, l]) =>
      el("option", { value: v, selected: String(v) === (extraOf(ed.doc, "antiphon_every") ?? "0") }, l),
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
    const n = await call("psalm_arrange", {
      id: c.id,
      options: { antiphon: antiphon.value.trim() || null, antiphon_every: Number(every.value), verses_per_slide: Number(per.value) },
      versesText: paste.value.trim() || null,
    });
    if (n) toast(`Arranged ${n} verses`);
  };
  return el(
    "details.section",
    { open: true },
    el(
      "summary",
      {},
      "Psalm layout",
      el("span.muted.small", {}, verses ? ` · ${verses} verses, choir plain / congregation bold` : " · no verses yet"),
    ),
    verses ? null : el("p.warn-note", {}, "This psalm has no verses yet. Paste them below and click Apply."),
    paste,
    el(
      "div.psalm-panel",
      {},
      field("Antiphon", antiphon),
      field("Repeat", every),
      field("Slides", per),
      el("button.primary", { disabled: app.readonly, onclick: apply }, "Apply"),
    ),
    el("p.muted.small", {}, "Apply rebuilds the slides from the verses. Chant images (♪ Music) move to the antiphon slides."),
  );
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
  row.style.setProperty("--bar", info?.style.editor_bar_color ?? "#f2f2f2");
  row.style.setProperty("--accent", info?.style.editor_accent_color ?? "#9a9a9a");
  row.classList.toggle("people", role === "People");
  row.classList.toggle("rubric", role === "Rubric");
}

function blockRow(b, i, paraIndex) {
  const row = el(`div.brow.brow-${b.kind}`, { dataset: { index: i } });
  if (b.kind === "paragraph") {
    row.classList.add(paraIndex % 2 ? "odd" : "even");
    applyRoleStyle(row, b.role);
    const roles = app.vocab.roles.map((r) => [r.role, r.role]);
    if (b.role && !roles.some(([r]) => r === b.role)) {
      // A number is a verse number (psalms, hymns), not a speaker.
      roles.push([b.role, /^\d/.test(b.role) ? `Verse ${b.role}` : `${b.role} (custom)`]);
    }
    const roleSel = select([["", "— Text —"], ...roles], b.role, (val) => {
      setField(b, "role", val);
      applyRoleStyle(row, b.role);
      changed();
    });
    roleSel.className = "role-select";

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
    row.append(el("div.brow-side", {}, roleSel), text, rowTools(i));
  } else if (b.kind === "heading") {
    row.append(
      el("div.brow-side", {}, el("span.kind-label", {}, "Section")),
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
            const path = await call("pick_image", { relativeTo: cue()?.content.source ?? null });
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
    el("p.muted", {}, deck ? `Deck: ${deck}` : "No announcements deck is set for this service (announcements_deck in the plan)."),
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
