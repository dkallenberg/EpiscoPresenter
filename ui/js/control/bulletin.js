// EpiscoPlanner's Bulletin and Script tabs: a live preview of the printed
// bulletin (half-letter pages) and the officiant's script, built by the app
// from the open service, with Export to Word. Clicking an item in the
// preview selects it; the bar above sets how it appears in the bulletin.

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
  const item = el("div.doc-item");
  const page = el("div.doc-page");
  const scroll = el("div.doc-scroll", {}, page);
  const exportLabel = "Export to Word";
  const toolbar = el(
    "div.doc-toolbar",
    {},
    el("label.field.inline", {}, el("span", {}, "Service time"), time),
    kind === "bulletin" ? item : el("span.muted.small", {}, "Every word, with rubrics in red. The Great Thanksgiving starts a new page."),
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
  host.replaceChildren(toolbar, scroll);
  const v = { kind, host, time, item, page, scroll, stale: true };
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
  if (html == null) return;
  const top = v.scroll.scrollTop;
  v.page.innerHTML = html;
  for (const img of v.page.querySelectorAll("img[data-path]")) img.src = assetUrl(null, img.dataset.path);
  v.scroll.scrollTop = top;
  markSelected(v, false);
  renderItem();
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
        "The NRSV and NRSVue are copyright the National Council of Churches, so the app does not include their text. The text appears in the bulletin, the script and on the slides, with the version's initials after it.",
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
  if (!cue) {
    v.item.replaceChildren(el("span.muted.small", {}, "Click an item to choose how it appears."));
    return;
  }
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
  v.item.replaceChildren(el("b.doc-item-name", { title: cue.title }, cue.title), seg);
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
  };
  const logoImg = el("img.parish-logo");
  const showLogo = () => {
    logoImg.hidden = !logo;
    if (logo) logoImg.src = assetUrl(null, logo);
    removeBtn.hidden = !logo;
  };
  const removeBtn = el("button", { onclick: () => ((logo = null), showLogo()) }, "Remove");
  const field = (label, node, wide) => el("label.field" + (wide ? ".wide" : ""), {}, el("span", {}, label), node);
  const overlay = el(
    "div.modal-overlay",
    { onmousedown: (e) => e.target === overlay && overlay.remove() },
    el(
      "div.modal.wide",
      {},
      el("h3", {}, "Parish details"),
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
