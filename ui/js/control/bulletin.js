// EpiscoPlanner's Bulletin and Script tabs: a live preview of the printed
// bulletin (half-letter pages) and the officiant's script, built by the app
// from the open service, with Export to Word. Clicking an item in the
// preview selects it; the bar above sets how it appears in the bulletin.
//
// The text can be edited in place, like a word processor: each printed line
// knows the line of the item it came from (data-src="block:line"), and an
// edit is written back to the item, so the slides change with it.

import { assetUrl, listen } from "/js/bridge.js";
import { app, call, debounce, el, toast } from "/js/control/util.js";

const STYLES = [
  ["reference", "Reference line", "Title, opening words and page or hymn number"],
  ["full", "Full text", "Every word, the people's parts in bold"],
  ["responses", "Responses only", "The title and just what the people say, each with its short cue line"],
  ["title", "Title only", "Just the title"],
  ["hidden", "Left out", "Not in the bulletin"],
];

const views = {};
let styles = new Map(); // cue id → {style, default}

function makeView(kind) {
  const host = document.getElementById(`${kind}-view`);
  if (!host) return null;
  const time = el("input", {
    type: "time",
    title: "When the service starts, for the cover",
    onchange: () => call("set_service_time", { time: time.value || null }),
  });
  // How the clicked item appears: a bar over the item itself.
  const item = el("div.doc-item-bar", { hidden: true });
  const page = el("div.doc-page");
  const scroll = el("div.doc-scroll", {}, page);
  const todo = el("aside.doc-todo");
  const exportLabel = "Export to Word";
  const paper =
    kind === "bulletin"
      ? el(
          "select.doc-paper",
          {
            title: "The sheet the bulletin is printed on, folded in half (also in Parish details)",
            onchange: async () => {
              const info = await call("parish_get");
              if (info) await call("parish_set", { info: { ...info, paper: paper.value } });
            },
          },
          [["tabloid", "11 × 17, folded"], ["letter", "8½ × 11, folded"]].map(([v, l]) => el("option", { value: v }, l)),
        )
      : null;
  const toolbar = el(
    "div.doc-toolbar",
    {},
    el("label.field.inline", {}, el("span", {}, "Service time"), time),
    paper ? el("label.field.inline", {}, el("span", {}, "Paper"), paper) : el("span.muted.small", {}, "Every word, with rubrics in red."),
    el(
      "div.seg.doc-format",
      { title: "Click into any text to edit it. Changes go into the item itself, so the slides change too." },
      el("button", { onmousedown: (e) => e.preventDefault(), onclick: () => document.execCommand("bold"), title: "Bold (Ctrl/⌘ B)" }, el("b", {}, "B")),
      el("button", { onmousedown: (e) => e.preventDefault(), onclick: () => document.execCommand("italic"), title: "Italic (Ctrl/⌘ I)" }, el("i", {}, "I")),
    ),
    el("span.spacer"),
    el("button.reading-texts", { onclick: () => openReadings(), title: "Paste the scripture text of each reading (NRSV or NRSVue)" }, "Reading texts…"),
    el("button", { onclick: openParish, title: "Name, logo, address, welcome and licence notice for the cover and back page" }, "Parish details…"),
    el("button.primary", { onclick: () => exportDoc(kind) }, `⤓ ${exportLabel}`),
  );
  page.addEventListener("click", (e) => {
    const missing = e.target.closest(".missing");
    const entry = e.target.closest("[data-cue]");
    if (entry) app.select(entry.dataset.cue);
    if (missing && entry) openReadings(entry.dataset.cue);
  });
  host.replaceChildren(toolbar, el("div.doc-body", {}, scroll, todo));
  const v = { kind, host, time, paper, item, page, scroll, todo, stale: true, editing: null, pending: false };
  wireEditing(v);
  views[kind] = v;
  return v;
}

const visible = (v) => document.getElementById(`tab-${v.kind}`)?.classList.contains("active");

async function refresh(v) {
  if (!v) return;
  if (!visible(v)) {
    v.stale = true;
    return;
  }
  v.stale = false;
  // Never redraw under someone's cursor: wait until they leave the line.
  if (v.editing && v.page.contains(v.editing)) {
    v.pending = true;
    return;
  }
  v.editing = null;
  v.pending = false;
  const s = app.service;
  if (document.activeElement !== v.time) v.time.value = s?.time ?? "";
  v.time.disabled = !s;
  if (!s) {
    v.page.replaceChildren(el("p.doc-empty", {}, "Open or start a service to see its bulletin."));
    renderItem();
    return;
  }
  const [html, list] = await Promise.all([
    call("bulletin_html", { kind: v.kind }),
    v.kind === "bulletin" ? call("bulletin_styles") : Promise.resolve(null),
  ]);
  if (list) styles = new Map(list.map((x) => [x.id, x]));
  else if (v.kind === "script") styles = new Map(((await call("bulletin_styles")) ?? []).map((x) => [x.id, x]));
  showMissing();
  renderTodo(v);
  if (html == null) return;
  const top = v.scroll.scrollTop;
  v.page.innerHTML = html;
  for (const img of v.page.querySelectorAll("img[data-path]")) img.src = assetUrl(null, img.dataset.path);
  if (!app.readonly) {
    for (const n of v.page.querySelectorAll(".ed, .entry[data-cue] .ed-title")) {
      n.contentEditable = "true";
      n.spellcheck = true;
      n.dataset.orig = n.innerHTML;
    }
  }
  if (v.kind === "bulletin") v.page.append(v.item);
  v.scroll.scrollTop = top;
  markSelected(v, false);
  renderItem();
}

/** What is still to do before Sunday, beside the page; each opens the place to do it. */
async function renderTodo(v) {
  const s = app.service;
  if (!s) return v.todo.replaceChildren();
  const parish = (await call("parish_get")) ?? {};
  if (v.paper && document.activeElement !== v.paper) v.paper.value = parish.paper ?? "tabloid";
  const items = [];
  if (!parish.name) items.push(["Add the parish's name, logo and details", () => openParish()]);
  if (!s.time) items.push(["Set the service time (for the cover)", () => v.time.focus()]);
  if (s.cues.some((c) => c.content.kind === "announcements") && !s.announcements_deck) {
    items.push(["Add the announcements' Google Slides link", () => document.getElementById("btn-settings")?.click()]);
  }
  for (const c of s.cues) {
    const t = styles.get(c.id)?.todo;
    if (!t) continue;
    items.push([
      `${c.title}: ${t.toLowerCase()}`,
      () => {
        app.select(c.id);
        if (styles.get(c.id)?.needs_text) openReadings(c.id);
        else if (c.category === "homily" || c.category === "hymn") document.querySelector('.tabs button[data-tab="editor"]')?.click();
      },
    ]);
  }
  v.todo.replaceChildren(
    el("h4", {}, items.length ? `To do (${items.length})` : "To do"),
    items.length
      ? el("ul", {}, items.map(([text, go]) => el("li", {}, el("button.link", { onclick: go }, text))))
      : el("p.muted.small", {}, "Nothing left to do ✓"),
  );
}

// ---------- editing in place ----------

/** The text of an edited line as spans, with bold and italic. */
function domSpans(node, fmt = { bold: false, italic: false }, out = []) {
  for (const n of node.childNodes) {
    if (n.nodeType === Node.TEXT_NODE) {
      if (n.nodeValue) out.push({ text: n.nodeValue, ...fmt });
    } else if (n.nodeType === Node.ELEMENT_NODE) {
      if (n.tagName === "BR") {
        out.push({ text: " ", ...fmt });
        continue;
      }
      const weight = n.style?.fontWeight ?? "";
      const bold = (fmt.bold || /^(B|STRONG)$/.test(n.tagName) || /bold|[6-9]00/.test(weight)) && weight !== "normal";
      const italic = (fmt.italic || /^(I|EM)$/.test(n.tagName) || n.style?.fontStyle === "italic") && n.style?.fontStyle !== "normal";
      domSpans(n, { bold, italic }, out);
    }
  }
  return out;
}

/** Where a line came from: its item and its block / line in the item. */
function origin(node) {
  const cue = node.closest("[data-cue]")?.dataset.cue;
  if (!cue) return null;
  if (node.classList.contains("ed-title")) return { cue, title: true };
  const [block, line] = (node.dataset.src ?? "").split(":");
  return { cue, block: Number(block), line: line === undefined ? null : Number(line) };
}

const caretOffset = (node) => {
  const sel = getSelection();
  if (!sel.rangeCount) return 0;
  const r = document.createRange();
  r.selectNodeContents(node);
  r.setEnd(sel.getRangeAt(0).startContainer, sel.getRangeAt(0).startOffset);
  // Leading spaces are not shown, so the caret after them is at the start.
  return r.toString().trimStart().length;
};

/** Puts the caret `offset` characters into the line `src` of item `cue`. */
function focusAt(v, cue, src, offset) {
  const node = v.page.querySelector(`[data-cue="${CSS.escape(cue)}"] [data-src="${src}"]`);
  if (!node) return;
  node.focus();
  const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
  let left = offset;
  let t;
  while ((t = walker.nextNode())) {
    if (left <= t.nodeValue.length) {
      getSelection().collapse(t, left);
      return;
    }
    left -= t.nodeValue.length;
  }
  getSelection().collapse(node, node.childNodes.length);
}

/** Saves a line if it changed. */
async function commit(node) {
  if (!node || node.innerHTML === node.dataset.orig) return true;
  const o = origin(node);
  if (!o) return true;
  node.dataset.orig = node.innerHTML;
  if (o.title) return (await call("bulletin_set_title", { id: o.cue, title: node.textContent })) !== undefined;
  const edit = { action: "set", spans: domSpans(node) };
  return (await call("bulletin_edit", { id: o.cue, block: o.block, line: o.line, edit })) !== undefined;
}

/** A change that adds or removes lines: save, redraw, put the caret back. */
async function restructure(v, node, edit, focus) {
  const o = origin(node);
  node.dataset.orig = node.innerHTML; // saved by this edit
  v.editing = null;
  const ok = (await call("bulletin_edit", { id: o.cue, block: o.block, line: o.line, edit })) !== undefined;
  await refresh(v);
  if (ok && focus) focusAt(v, o.cue, focus.src, focus.offset);
}

function wireEditing(v) {
  const page = v.page;
  const editable = (t) => t?.closest?.(".ed[contenteditable=true], .ed-title[contenteditable=true]");
  const autosave = debounce((n) => commit(n), 1200);
  page.addEventListener("focusin", (e) => {
    const n = editable(e.target);
    if (n) v.editing = n;
  });
  page.addEventListener("focusout", async (e) => {
    const n = editable(e.target);
    if (!n) return;
    autosave.cancel();
    // Moving straight to another line: keep the page as it is until the
    // editing stops, so the line being clicked is not redrawn away.
    const next = editable(e.relatedTarget);
    if (v.editing === n) v.editing = next && page.contains(next) ? next : null;
    await commit(n);
    if (v.pending && !v.editing) refresh(v);
  });
  page.addEventListener("input", (e) => {
    const n = editable(e.target);
    if (n) autosave(n);
  });
  page.addEventListener("paste", (e) => {
    if (!editable(e.target)) return;
    e.preventDefault();
    const text = (e.clipboardData?.getData("text/plain") ?? "").replace(/\s*\n\s*/g, " ");
    document.execCommand("insertText", false, text);
  });
  page.addEventListener("keydown", async (e) => {
    const n = editable(e.target);
    if (!n) return;
    const o = origin(n);
    if (e.key === "Escape") {
      n.innerHTML = n.dataset.orig;
      n.blur();
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      if (o.title || o.line == null) return n.blur();
      autosave.cancel();
      // Split the line at the caret: the rest becomes a new line after it.
      const sel = getSelection();
      const r = sel.getRangeAt(0);
      r.deleteContents();
      const tail = document.createRange();
      tail.setStart(r.startContainer, r.startOffset);
      tail.setEnd(n, n.childNodes.length);
      const holder = document.createElement("span");
      holder.append(tail.extractContents());
      // Like a word processor: no stray space at the end of the line or the
      // start of the new one.
      const trim = (spans, end) => {
        const out = spans.map((x) => ({ ...x }));
        while (out.length) {
          const x = end ? out[out.length - 1] : out[0];
          x.text = end ? x.text.trimEnd() : x.text.trimStart();
          if (x.text) break;
          end ? out.pop() : out.shift();
        }
        return out;
      };
      const edit = { action: "split", spans: trim(domSpans(n), true), rest: trim(domSpans(holder), false) };
      await restructure(v, n, edit, { src: `${o.block}:${o.line + 1}`, offset: 0 });
      return;
    }
    if (e.key === "Backspace" && o.line > 0 && getSelection().isCollapsed && caretOffset(n) === 0) {
      e.preventDefault();
      autosave.cancel();
      await commit(n);
      const prev = page.querySelector(`[data-cue="${CSS.escape(o.cue)}"] [data-src="${o.block}:${o.line - 1}"]`);
      await restructure(v, n, { action: "join_previous" }, { src: `${o.block}:${o.line - 1}`, offset: prev?.textContent.length ?? 0 });
    }
  });
}

function markSelected(v, scroll) {
  for (const n of v.page.querySelectorAll(".entry.selected")) n.classList.remove("selected");
  const n = app.selectedId && v.page.querySelector(`[data-cue="${CSS.escape(app.selectedId)}"]`);
  if (!n) return;
  n.classList.add("selected");
  if (scroll) n.scrollIntoView({ block: "nearest" });
}

/** "Reading texts…" turns gold while a reading still has no text. */
function showMissing() {
  const n = [...styles.values()].filter((x) => x.needs_text).length;
  for (const b of document.querySelectorAll(".doc-toolbar .reading-texts")) {
    b.classList.toggle("warn", n > 0);
    b.textContent = n ? `Reading texts… (${n} to add)` : "Reading texts…";
  }
}

/**
 * The scripture text of each reading. With the parish's Bible file imported
 * (once, into the library folder) readings fill in from their citations;
 * otherwise "Open" shows the passage in the browser to copy and paste.
 * `focus`: a cue id to start at.
 */
export async function openReadings(focus = null) {
  if (!app.service) return toast("Open a service first.", "warn");
  const version = el(
    "select",
    { onchange: () => render() },
    [["NRSVue", "NRSVue (Updated Edition, 2021)"], ["NRSV", "NRSV (1989)"]].map(([v, l]) =>
      el("option", { value: v, selected: v === (app.service?.bible ?? "NRSVue") }, l),
    ),
  );
  const fileBox = el("div.bible-file");
  const list = el("div.reading-list");
  let rows = [];

  const fillFrom = async (ids, replace = false) => {
    const r = await call("reading_fill", { ids, version: version.value, replace });
    if (!r) return;
    if (r.filled) toast(`${r.filled} reading${r.filled === 1 ? "" : "s"} filled in from the ${version.value} Bible file`);
    for (const f of r.failed) toast(f, "warn");
    if (!r.filled && !r.failed.length) toast("Nothing to fill: the readings already have their text.");
    await render();
  };

  const importFile = async () => {
    const path = await call("bible_pick_file");
    if (!path) return;
    const books = await call("bible_import", { path, version: version.value });
    if (books == null) return;
    toast(`${version.value} Bible file imported (${books} books). New readings now fill in by themselves.`);
    await render();
    if ([...styles.values()].some((x) => x.needs_text)) await fillFrom(null);
  };

  async function render() {
    const [list_, bibles] = await Promise.all([call("bulletin_styles"), call("bible_status")]);
    styles = new Map((list_ ?? []).map((x) => [x.id, x]));
    showMissing();
    const have = (bibles ?? []).find((b) => b.version === version.value);
    const readings = (app.service?.cues ?? []).filter((c) => styles.get(c.id)?.reading);
    const missing = readings.filter((c) => styles.get(c.id)?.needs_text).length;
    fileBox.replaceChildren(
      have
        ? el(
            "div.bible-file-row",
            {},
            el("span", {}, `✓ ${version.value} Bible file in the library folder (${have.books} books). Readings fill in from their citations.`),
            el("span.spacer"),
            missing ? el("button.primary", { onclick: () => fillFrom(null) }, `Fill ${missing} missing reading${missing === 1 ? "" : "s"}`) : null,
            el("button", { onclick: importFile, title: "Replace the file" }, "Replace file…"),
          )
        : el(
            "div.bible-file-row",
            {},
            el(
              "span.small",
              {},
              `No ${version.value} Bible file yet. If the parish has the ${version.value} text as a JSON file, import it once: it is copied into the parish library folder and every reading fills in from its citation. Otherwise paste each reading below.`,
            ),
            el("button.primary", { onclick: importFile }, "Import Bible file…"),
          ),
    );
    rows = readings.map((c) => {
      const info = styles.get(c.id);
      const citation = info.citation ?? "";
      const status = el("span.small" + (info.needs_text ? ".warn-text" : ".muted"), {}, info.needs_text ? "Text not added" : `✓ ${info.bible ?? "text"} added`);
      const paste = el("textarea.reading-paste", {
        rows: 2,
        placeholder: citation ? `Or paste ${citation} here. Verse numbers, headings, footnotes and the copyright line are removed.` : "This reading has no citation.",
        disabled: !citation,
      });
      const apply = async () => {
        if (!paste.value.trim()) return toast("Paste the reading first.", "warn");
        const n = await call("reading_set_text", { id: c.id, text: paste.value, version: version.value, words: 40 });
        if (n != null) {
          toast(`${c.title}: text added`);
          await render();
        }
      };
      return el(
        "div.reading-item",
        { dataset: { cue: c.id } },
        el(
          "div.reading-item-head",
          {},
          el("b", {}, c.title),
          el("span", {}, citation || "—"),
          status,
          el("span.spacer"),
          have && citation
            ? el("button", { onclick: () => fillFrom([c.id], true), title: "Take the text from the Bible file" }, info.needs_text ? "Fill" : "Refill")
            : null,
          el("button", { disabled: !citation, onclick: () => call("open_passage", { citation, version: version.value }) }, "Open ↗"),
          el("button", { disabled: !citation, onclick: apply }, "Use pasted"),
        ),
        paste,
      );
    });
    list.replaceChildren(...(rows.length ? rows : [el("p.muted", {}, "This service has no readings yet. Add the propers from the Library.")]));
  }

  const overlay = el(
    "div.modal-overlay",
    { onmousedown: (e) => e.target === overlay && overlay.remove() },
    el(
      "div.modal.wide",
      {},
      el("h3", {}, "Reading texts"),
      el(
        "p.muted.small",
        {},
        "The NRSV and NRSVue are copyright the National Council of Churches, so the app does not include their text. The text appears in the bulletin, the script and on the slides.",
      ),
      el("label.field.inline", {}, el("span", {}, "Bible"), version),
      fileBox,
      list,
      el("div.modal-actions", {}, el("button", { onclick: () => overlay.remove() }, "Done")),
    ),
  );
  document.body.append(overlay);
  await render();
  const start = rows.find((r) => r.dataset.cue === focus) ?? rows.find((r) => styles.get(r.dataset.cue)?.needs_text) ?? rows[0];
  start?.scrollIntoView({ block: "nearest" });
}

/** The bar for the selected item: how it appears in the bulletin. */
function renderItem() {
  const v = views.bulletin;
  if (!v) return;
  const cue = app.selectedCue();
  const entry = cue && v.page.querySelector(`.entry[data-cue="${CSS.escape(cue.id)}"]`);
  // An item left out has no place on the page: its bar stays in view so it
  // can be brought back.
  const leftOut = cue && !entry && styles.get(cue.id)?.style === "hidden";
  v.item.hidden = !(entry || leftOut) || app.readonly;
  if (v.item.hidden) return;
  const info = styles.get(cue.id);
  const current = info?.style ?? "full";
  const seg = el(
    "div.seg.doc-styles",
    {},
    STYLES.map(([key, label, tip]) =>
      el(
        "button" + (key === current ? ".active" : "") + (info?.default === key ? ".usual" : ""),
        {
          title: tip + (info?.default === key ? " (the usual for this item)" : ""),
          onclick: async () => {
            const style = key === info?.default ? null : key;
            await call("set_cue_bulletin", { id: cue.id, style });
          },
        },
        label,
      ),
    ),
  );
  v.item.replaceChildren(el("span.doc-item-name", { title: cue.title }, leftOut ? `“${cue.title}” is left out:` : "In the bulletin:"), seg);
  if (v.item.parentNode !== v.page) v.page.append(v.item);
  // Just above the item, at its right; a left-out item's at the top of the view.
  const top = entry ? entry.offsetTop - v.item.offsetHeight - 4 : v.scroll.scrollTop - v.page.offsetTop + 8;
  v.item.style.top = `${Math.max(0, top)}px`;
}

async function exportDoc(kind) {
  if (!app.service) return toast("Open a service first.", "warn");
  const path = await call("bulletin_export", { kind });
  if (path) toast(`Saved ${String(path).split(/[\\/]/).pop()}; opening it in Word.`);
}

/** The parish details dialog. */
export async function openParish() {
  const info = (await call("parish_get")) ?? {};
  let logo = info.logo ?? null;
  const input = (key, placeholder) => el("input", { value: info[key] ?? "", placeholder });
  const area = (key, rows, placeholder) => el("textarea", { rows, placeholder }, info[key] ?? "");
  const f = {
    name: input("name", "e.g. St. James Episcopal Church"),
    place: input("place", "e.g. Anytown, Ohio"),
    liturgy_title: input("liturgy_title", "The Holy Eucharist"),
    address: area("address", 2, "Street address"),
    phone: input("phone", ""),
    website: input("website", ""),
    email: input("email", ""),
    licence: area("licence", 2, "e.g. Music reprinted under OneLicense #A-000000. All rights reserved."),
    welcome: area("welcome", 4, "A word to visitors for the back page. Leave a blank line between paragraphs."),
    paper: el(
      "select",
      {},
      [
        ["tabloid", "11 × 17, folded (8½ × 11 pages)"],
        ["letter", "8½ × 11, folded (5½ × 8½ pages)"],
      ].map(([v, l]) => el("option", { value: v, selected: v === (info.paper ?? "tabloid") }, l)),
    ),
    service_times: el("input", {
      value: (info.service_times ?? []).map((t) => t.replace(/^0/, "")).join(", "),
      placeholder: "e.g. 8:00, 10:30",
    }),
    speaker_labels: el(
      "select",
      {},
      [
        ["once", "Once for each block by the same speaker"],
        ["every", "On every paragraph"],
      ].map(([v, l]) => el("option", { value: v, selected: v === (info.speaker_labels ?? "once") }, l)),
    ),
  };
  const logoImg = el("img.parish-logo");
  const showLogo = () => {
    logoImg.hidden = !logo;
    if (logo) logoImg.src = assetUrl(null, logo);
    removeBtn.hidden = !logo;
  };
  const removeBtn = el("button", { onclick: () => ((logo = null), showLogo()) }, "Remove");
  const field = (label, node, wide) => el("label.field" + (wide ? ".wide" : ""), {}, el("span", {}, label), node);
  // The clergy, offered as preachers in a sermon's details.
  const clergy = {};
  const clergyRow = (key, label, title) => {
    const c = (clergy[key] = {
      honorific: el("input", { value: info[key]?.honorific ?? "", placeholder: key === "bishop" ? "The Rt. Rev." : "The Rev.", list: "parish-honorifics" }),
      name: el("input", { value: info[key]?.name ?? "", placeholder: "e.g. Jane Doe" }),
      title: el("input", { value: info[key]?.title || title, placeholder: title }),
    });
    return el("div.clergy-row", {}, el("span.clergy-label", {}, label), c.honorific, c.name, c.title);
  };
  const overlay = el(
    "div.modal-overlay",
    { onmousedown: (e) => e.target === overlay && overlay.remove() },
    el(
      "div.modal.wide",
      {},
      el("h3", {}, "Parish details and bulletin options"),
      el("p.muted.small", {}, "Printed on the bulletin's cover and back page. Kept in the parish library folder, so everyone shares them."),
      el(
        "div.parish-grid",
        {},
        field("Parish name", f.name),
        field("Place", f.place),
        field("Cover title", f.liturgy_title),
        el(
          "div.field",
          {},
          el("span", {}, "Logo (PNG or JPEG)"),
          el(
            "div.parish-logo-row",
            {},
            logoImg,
            el(
              "button",
              {
                onclick: async () => {
                  const p = await call("parish_pick_logo");
                  if (p) ((logo = p), showLogo());
                },
              },
              "Choose…",
            ),
            removeBtn,
          ),
        ),
        field("Address", f.address, true),
        field("Phone", f.phone),
        field("Website", f.website),
        field("Email", f.email),
        el("span"),
        field("Music licence notice", f.licence, true),
        field("Welcome for visitors", f.welcome, true),
        field("Speaker labels (People, All, Celebrant…) in the bulletin and script", f.speaker_labels),
        field("Sunday service times (24-hour, e.g. 8:00, 10:30, 17:00)", f.service_times),
        field("Bulletin paper", f.paper),
        el(
          "div.field.wide",
          {},
          el("span", {}, "Clergy (offered as preachers for the sermon): title, name and position"),
          el("datalist", { id: "parish-honorifics" }, ["The Rev.", "The Very Rev.", "The Rt. Rev.", "The Most Rev.", "The Ven.", "The Rev. Canon", "The Rev. Dr.", "The Rev. Deacon"].map((h) => el("option", { value: h }))),
          clergyRow("rector", "Rector", "Rector"),
          clergyRow("assistant", "Assistant", "Assistant Rector"),
          clergyRow("bishop", "Bishop", "Bishop"),
        ),
      ),
      el(
        "div.modal-actions",
        {},
        el("button", { onclick: () => overlay.remove() }, "Cancel"),
        el(
          "button.primary",
          {
            onclick: async () => {
              const out = { logo };
              for (const [k, n] of Object.entries(f)) out[k] = n.value;
              // "8:00, 10:30" → ["08:00", "10:30"]
              out.service_times = f.service_times.value
                .split(/[,;\s]+/)
                .map((t) => /^(\d{1,2})(?::(\d{2}))?$/.exec(t.trim()))
                .filter((m) => m && Number(m[1]) < 24 && Number(m[2] ?? 0) < 60)
                .map((m) => `${m[1].padStart(2, "0")}:${m[2] ?? "00"}`);
              for (const [k, c] of Object.entries(clergy)) out[k] = { honorific: c.honorific.value.trim(), name: c.name.value.trim(), title: c.title.value.trim() };
              if (await call("parish_set", { info: out })) {
                overlay.remove();
                toast("Parish details saved");
              }
            },
          },
          "Save",
        ),
      ),
    ),
  );
  showLogo();
  document.body.append(overlay);
  f.name.focus();
}

export function initBulletin() {
  const vs = ["bulletin", "script"].map(makeView).filter(Boolean);
  if (!vs.length) return;
  const later = debounce(() => vs.forEach(refresh), 250);
  app.on((what) => {
    if (what === "service") later();
    if (what === "selection") {
      vs.forEach((v) => markSelected(v, true));
      renderItem();
    }
  });
  listen("episco://parish", later);
  // Showing a tab brings it up to date.
  for (const v of vs) {
    document.querySelector(`.tabs button[data-tab="${v.kind}"]`)?.addEventListener("click", () => {
      if (v.stale) refresh(v);
    });
  }
}
