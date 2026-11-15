// Center pane, "Library" tab: reusable items (built-in Rite II texts and
// the operator's own saved items), browsable by type, with previews; plus the
// "Propers" dialog: the Collect, readings and psalm for a date from the lectionary.

import { listen } from "/js/bridge.js";
import { mountStage, renderOutput } from "/js/render.js";
import { renderMedia } from "/js/control/media.js";
import { app, call, el, titleCase, toast } from "/js/control/util.js";

const lib = { entries: [], category: "all", query: "", selected: null, version: null, mine: false, view: "items", rite: "all" };

// ---------- versions of Prayer Book items ----------

/** "13 Nov 2026" from YYYY-MM-DD. */
export function dayLabel(iso) {
  const [y, m, d] = (iso ?? "").split("-").map(Number);
  if (!y) return iso ?? "";
  return `${d} ${["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][m - 1]} ${y}`;
}

const baseName = (e) => (e.builtin ? e.id.replace(/^builtin:/, "") : e.base);

/** A built-in item and the parish's copies of it: BCP first, then by date. */
function versionsOf(e) {
  const name = baseName(e);
  if (!name) return [e];
  const bcp = lib.entries.find((x) => x.id === `builtin:${name}`);
  const copies = lib.entries.filter((x) => x.base === name).sort((a, b) => (a.created ?? "").localeCompare(b.created ?? "") || a.id.localeCompare(b.id));
  return [bcp, ...copies].filter(Boolean);
}

/** "BCP", "Default" or the day the copy was made. */
function versionLabel(v) {
  if (v.builtin) return "BCP";
  return v.default ? "Default" : dayLabel(v.created);
}

/** Items | Media switch at the top of the Library. */
function viewSwitch() {
  return el(
    "div.seg.lib-views",
    {},
    [["items", "Items"], ["media", "Media"]].map(([v, l]) =>
      el(`button${lib.view === v ? ".active" : ""}`, { onclick: () => { lib.view = v; refresh(); } }, l),
    ),
  );
}

/** New items go after the selected cue, else at the end. */
function insertionPoint() {
  const i = app.selectedIndex();
  return i >= 0 ? i : null;
}

export function initLibrary() {
  listen("episco://storage", () => refresh());
  // Files from another computer (sync.rs): the lists only.
  listen("episco://library", () => refresh());
  document.querySelector('.tabs button[data-tab="library"]').addEventListener("click", () => {
    refresh();
    // Fetch the other computers' changes now (the list refreshes if any came).
    if (window.__TAURI__) window.__TAURI__.core.invoke("library_sync").catch(() => {});
  });
}

export async function refresh() {
  if (lib.view === "media") return renderMedia(document.getElementById("library"), viewSwitch());
  lib.entries = (await call("library_list")) ?? [];
  render();
}

function matches(e) {
  if (lib.mine && e.builtin) return false;
  // A copy of a Prayer Book item is listed under it (and under ★ Mine).
  if (e.base && !lib.mine) return false;
  if (lib.rite !== "all" && e.rite && e.rite !== lib.rite) return false;
  if (lib.category !== "all" && e.category !== lib.category) return false;
  const q = lib.query.trim().toLowerCase();
  return !q || `${e.title} ${e.citation ?? ""} ${e.summary}`.toLowerCase().includes(q);
}

function render() {
  const root = document.getElementById("library");
  const cats = ["all", ...new Set(lib.entries.map((e) => e.category))];
  const shown = lib.entries.filter(matches);
  if (!shown.some((e) => e.id === lib.selected)) lib.selected = shown[0]?.id ?? null;

  const search = el("input.lib-search", {
    type: "search",
    placeholder: "Search titles and text…",
    value: lib.query,
    oninput: (e) => {
      lib.query = e.target.value;
      const pos = e.target.selectionStart;
      render();
      const s = document.querySelector(".lib-search");
      s.focus();
      s.setSelectionRange(pos, pos);
    },
  });
  const chips = el(
    "div.chips",
    {},
    cats.map((c) =>
      el(`button.chip${lib.category === c ? ".active" : ""}`, { onclick: () => { lib.category = c; render(); } }, c === "all" ? "All" : titleCase(c)),
    ),
    el(`button.chip${lib.mine ? ".active" : ""}`, { title: "Only items you saved", onclick: () => { lib.mine = !lib.mine; render(); } }, "★ Mine"),
    ["I", "II"].map((r) =>
      el(`button.chip${lib.rite === r ? ".active" : ""}`, { title: `Only Rite ${r} texts (and those common to both)`, onclick: () => { lib.rite = lib.rite === r ? "all" : r; render(); } }, `Rite ${r}`),
    ),
  );
  const list = el(
    "div.lib-list",
    {},
    shown.length
      ? shown.map((e) =>
          el(
            `div.lib-row${e.id === lib.selected ? ".selected" : ""}`,
            {
              onclick: () => { lib.selected = e.id; lib.version = null; render(); },
              ondblclick: () => {
                const id = e.builtin ? (versionsOf(e).find((v) => v.default) ?? e).id : e.id;
                // A hymn on an empty hymn slot fills it (keeping "Entrance Hymn").
                const slot = e.category === "hymn" ? emptySlot() : null;
                return slot && !app.readonly ? fill(id, slot, e.title) : add([id]);
              },
            },
            el(
              "div.lib-title",
              {},
              e.title,
              e.builtin
                ? el("span.badge", { title: "The Book of Common Prayer's text" }, "BCP")
                : e.base
                  ? el("span.badge.own", {}, `Copy · ${versionLabel(e)}`)
                  : el("span.badge.own", {}, "Mine"),
              e.builtin && !e.default ? el("span.badge.own", { title: "New services use the parish's default version" }, "Parish default") : null,
              e.builtin && versionsOf(e).length > 1 ? el("span.muted.small", {}, ` ${versionsOf(e).length - 1} cop${versionsOf(e).length === 2 ? "y" : "ies"}`) : null,
              e.dupes?.length ? el("span.badge.dupe", { title: "This item is in the library more than once. Select it to keep one copy." }, `×${e.dupes.length + 1}`) : null,
            ),
            el("div.lib-meta", {}, [titleCase(e.category), e.rite ? `Rite ${e.rite}` : null, e.citation, e.season, `${e.slides} slide${e.slides === 1 ? "" : "s"}`].filter(Boolean).join(" · ")),
            el("div.lib-summary", {}, e.summary),
          ),
        )
      : el("p.muted", {}, lib.query || lib.category !== "all" || lib.mine || lib.rite !== "all" ? "Nothing matches: try fewer words, or click All." : "The library is empty."),
  );
  const preview = el("div.lib-preview#lib-preview");
  root.replaceChildren(
    el(
      "div.lib-side",
      {},
      el(
        "div.lib-toolbar",
        {},
        viewSwitch(),
        search,
        el("button.small", { title: "The day's Collect, readings and psalm from the lectionary", onclick: openPropers }, "Propers…"),
        el("button.small", { title: "Add hymns to your library from text files (one per hymn, named with its number and title)", onclick: importHymns }, "Import hymns…"),
        el("button.small", { title: "Make a library item from a prayer or rite pasted from your own copy (e.g. Enriching Our Worship)", onclick: () => pasteText() }, "Paste a text…"),
      ),
      chips,
      list,
      syncLine(),
      el("p.hint.muted", {}, "Double-click an item to add it after the selected cue (a hymn fills an empty hymn slot, such as the Entrance Hymn, keeping its name). Save your own items from the Editor with “Save to Library”."),
    ),
    preview,
  );
  if (lib.selected) showPreview(lib.selected);
}

async function showPreview(rowId) {
  const host = document.getElementById("lib-preview");
  const row = lib.entries.find((e) => e.id === rowId);
  // A Prayer Book item: show the chosen version, else the default.
  const versions = row ? versionsOf(row) : [];
  const chosen = versions.find((v) => v.id === lib.version) ?? (row?.builtin ? versions.find((v) => v.default) : row) ?? row;
  const id = chosen?.id ?? rowId;
  const entry = chosen;
  const cue = await call("library_preview", { id });
  if (!cue || lib.selected !== rowId) return;
  const slides = cue.content.slides.length ? cue.content.slides : [null];
  const grid = el("div.lib-slides");
  host.replaceChildren(
    el(
      "div.lib-preview-head",
      {},
      el("div", {}, el("strong", {}, cue.title), el("div.muted.small", {}, [titleCase(cue.category), entry?.citation].filter(Boolean).join(" · "))),
      el(
        "div.lib-actions",
        {},
        entry && !entry.builtin ? el("button.small.danger-btn", { onclick: () => remove(entry) }, "Delete") : null,
        slotFor(cue) ? el("button", { disabled: app.readonly, title: `Put this hymn in the service as the ${slotFor(cue).title}`, onclick: () => fill(id, slotFor(cue)) }, `Use for ${slotFor(cue).title}`) : null,
        el(`button${slotFor(cue) ? "" : ".primary"}`, { disabled: app.readonly || !app.service, onclick: () => add([id]) }, "Add to service"),
      ),
    ),
    // (replaceChildren would print a null as "null".)
    entry?.dupes?.length ? dupeBar(entry) : "",
    baseName(entry ?? {}) ? versionBar(versions, entry) : "",
    el("div.picker-sub", {}, "Sanctuary · Live Stream"),
    grid,
  );
  // Render after attaching so text can be fitted.
  const snap = { service: app.service, state: { revision: 0 } };
  slides.forEach((slide, si) => {
    for (const output of ["sanctuary", "stream"]) {
      const thumb = el("div.thumb");
      grid.append(thumb);
      renderOutput(mountStage(thumb), snap, { cue, slide, pos: { cue: -1, slide: si } }, output);
    }
  });
}

/** The versions of a Prayer Book item: pick one to preview; make a copy,
 *  edit it, or make it the one services use. */
function versionBar(versions, shown) {
  const name = baseName(shown);
  const pick = (v) => {
    lib.version = v.id;
    showPreview(lib.selected);
  };
  const copy = async () => {
    const id = await call("library_copy", { id: shown.id });
    if (!id) return;
    toast("Copy made: edit it with “Add & edit”, then Save to Library");
    lib.version = id;
    await refresh();
  };
  const setDefault = async () => {
    const ok = await call("library_set_default", { base: name, id: shown.builtin ? null : shown.id });
    if (ok === undefined) return;
    toast(shown.builtin ? "New services use the Prayer Book's text" : `New services use the copy of ${dayLabel(shown.created)}`);
    lib.version = shown.id;
    await refresh();
  };
  const edit = async () => {
    if (!app.service) return toast("Open a service first (☰ Services).", "warn");
    const added = await call("library_add", { ids: [shown.id], after: insertionPoint() });
    if (!added?.length) return;
    app.select(added[0]);
    document.querySelector('.tabs button[data-tab="editor"]')?.click();
    toast("Added to the service. Edit it, then Save to Library to keep the changes in this copy.");
  };
  return el(
    "div.lib-versions",
    {},
    el("span.muted.small", {}, "Versions"),
    el(
      "div.seg",
      {},
      versions.map((v) =>
        el(
          `button${v.id === shown.id ? ".active" : ""}`,
          { title: v.builtin ? "The Book of Common Prayer's text" : `The parish's copy, made ${dayLabel(v.created)}`, onclick: () => pick(v) },
          versionLabel(v),
          v.default ? " ★" : "",
        ),
      ),
    ),
    el(
      "div.lib-version-actions",
      {},
      el("button.small", { disabled: app.readonly, title: "A copy of this version for the parish to edit", onclick: copy }, "Make a copy"),
      shown.builtin ? null : el("button.small", { disabled: app.readonly, title: "Add this copy to the service and open it in the Editor", onclick: edit }, "Add & edit"),
      shown.default
        ? el("span.muted.small", {}, "★ New services use this version")
        : el("button.small", { disabled: app.readonly, title: "New services, and adding this item, use this version", onclick: setDefault }, "Set as default"),
    ),
  );
}

/** The selected hymn slot ("Entrance Hymn") a library hymn can go into. */
function slotFor(item) {
  if (item?.category !== "hymn" || !app.service) return null;
  const c = app.service.cues[app.selectedIndex()];
  // A hymn slot, or the Sanctus (said or sung): a hymnal setting can fill it.
  return c?.category === "hymn" || /sanctus/i.test(c?.title ?? "") ? c : null;
}

/** The selected item if it is a hymn slot still waiting for its hymn. */
function emptySlot() {
  const c = app.service?.cues[app.selectedIndex()];
  if (c?.category !== "hymn" || c.content.kind !== "liturgy") return null;
  const doc = c.content.document;
  const text = (doc?.blocks ?? []).map((b) => (b.lines ?? []).flat().map((x) => x.text).join("")).join(" ");
  return !doc?.meta?.hymn_number && /^Choose this hymn/.test(text.trim()) ? c : null;
}

/** Puts a library hymn into a slot; the slot keeps its name. */
async function fill(id, slot, title) {
  const ok = await call("library_fill_cue", { id, cue: slot.id });
  if (ok === undefined) return;
  app.select(slot.id);
  toast(`The ${slot.title} is now “${title ?? lib.entries.find((e) => e.id === lib.selected)?.title ?? "this hymn"}”`);
}

async function add(ids) {
  if (!app.service) return toast("Open a service first (☰ Services).", "warn");
  const added = await call("library_add", { ids, after: insertionPoint() });
  if (added?.length) {
    app.select(added[0]);
    toast(`Added to the service: ${lib.entries.find((e) => e.id === ids[0])?.title ?? ""}`);
  }
}

async function importHymns() {
  const r = await call("library_import_hymns");
  if (!r) return;
  if (r.added) toast(`Added ${r.added} hymn${r.added === 1 ? "" : "s"} to your library`);
  if (r.skipped.length) toast(`Skipped: ${r.skipped.join(", ")}`, "warn");
  lib.category = "hymn";
  await refresh();
}

/**
 * A prayer or rite pasted from the parish's own copy of a book that cannot
 * be built in (e.g. Enriching Our Worship), saved to the parish library.
 */
export function pasteText() {
  const title = el("input", { placeholder: "e.g. Eucharistic Prayer 1 (EOW)" });
  const type = el(
    "select",
    {},
    [["eucharistic_prayer", "Eucharistic Prayer"], ["prayer", "Prayer or dialogue"], ["collect", "Collect"], ["acclamation", "Acclamation"], ["anthem", "Anthem or canticle"], ["other", "Other"]].map(([v, l]) =>
      el("option", { value: v }, l),
    ),
  );
  const citation = el("input", { placeholder: "e.g. EOW 1, p. 57" });
  const text = el("textarea.paste-text", {
    rows: 14,
    placeholder:
      "Paste the text here.\n\nStart each speaker's part with their name, e.g.\nPresider\tThe Lord be with you.\nPeople\tAnd also with you.\n\nLines after it continue that part; a blank line starts a new paragraph. “Presider and People” or “All” is said together (bold). Instructions in [brackets] become rubrics.",
  });
  const field = (label, input) => el("label.field", {}, el("span", {}, label), input);
  const save = async (replace = false) => {
    let id;
    try {
      id = await window.__TAURI__.core.invoke("library_create_from_text", {
        title: title.value,
        category: type.value,
        citation: citation.value || null,
        text: text.value,
        replace,
      });
    } catch (e) {
      const msg = String(e);
      if (msg.startsWith("exists:")) {
        const ok = await call("confirm", { title: "Replace library item", message: `Your library already has “${msg.slice(7)}”. Replace it?`, okLabel: "Replace" });
        if (ok) return save(true);
        return;
      }
      return toast(msg, "error");
    }
    overlay.remove();
    toast(`“${title.value.trim()}” added to your library`);
    lib.selected = id;
    await refresh();
  };
  const overlay = el(
    "div.modal-overlay",
    { onmousedown: (e) => e.target === overlay && overlay.remove() },
    el(
      "div.modal.wide",
      {},
      el("h3", {}, "Paste a text"),
      el(
        "p.muted.small",
        {},
        "For texts the app cannot include, such as the Eucharistic Prayers of Enriching Our Worship (© Church Pension Fund): copy them from your parish's copy and paste here. The item is saved in the parish library folder, where everyone can use it; check it in the Editor afterwards.",
      ),
      el("div.home-row", {}, field("Title", title), field("Type", type)),
      field("Where it comes from (printed on the bulletin)", citation),
      text,
      el("div.modal-actions", {}, el("button", { onclick: () => overlay.remove() }, "Cancel"), el("button.primary", { onclick: () => save(false) }, "Add to library")),
    ),
  );
  document.body.append(overlay);
  title.focus();
}

/** The other copies of an item: keep the one shown, remove the rest. */
function dupeBar(entry) {
  const others = entry.dupes.map((id) => lib.entries.find((e) => e.id === id)).filter(Boolean);
  const merge = async () => {
    const ok = await call("confirm", {
      title: "Keep one copy",
      message: `Keep this “${entry.title}” and remove the other ${others.length === 1 ? "copy" : `${others.length} copies`} (${others.map((o) => `“${o.title}”`).join(", ")}) from the library? Services keep their own words.`,
      okLabel: "Keep this one",
    });
    if (!ok) return;
    const n = await call("library_merge", { keep: entry.id, remove: entry.dupes });
    if (n === undefined) return;
    toast(`Removed ${n} cop${n === 1 ? "y" : "ies"}; “${entry.title}” stays`);
    await refresh();
  };
  return el(
    "div.lib-dupes",
    {},
    el("span", {}, `In the library ${others.length + 1} times: `, others.flatMap((o, i) => [i ? ", " : "", el("button.link", { onclick: () => { lib.selected = o.id; render(); } }, o.title)])),
    el("button.small", { disabled: app.readonly, title: "Keep this copy and remove the others", onclick: merge }, "Keep this one"),
  );
}

/** When the library last came in step with the other computers. */
function syncLine() {
  const line = el("p.lib-sync.muted.small");
  if (!window.__TAURI__) return line;
  window.__TAURI__.core.invoke("library_sync_status").then((st) => {
    if (!st?.shared) return;
    const at = st.at ? new Date(st.at * 1000).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" }) : null;
    line.textContent = st.error
      ? `⚠ Not in step with the shared folder: ${st.error}`
      : st.waiting
        ? `⇣ ${st.waiting} change${st.waiting === 1 ? "" : "s"} from the other computer still downloading (Google Drive)…`
        : at
          ? `✓ In step with the other computers · checked ${at}`
          : "Checking the shared folder…";
    line.classList.toggle("warn", Boolean(st.error));
  }).catch(() => {});
  return line;
}

async function remove(entry) {
  const ok = await call("confirm", { title: "Delete from library", message: `Delete “${entry.title}” from your library?`, okLabel: "Delete" });
  if (!ok) return;
  await call("library_delete", { id: entry.id });
  lib.version = null;
  await refresh();
}

/** "Save to Library" for the cue in the editor. */
export async function saveCueToLibrary(cue) {
  let id;
  const based = cue.content.document?.meta.extra?.find(([k]) => k === "based_on")?.[1];
  try {
    id = await window.__TAURI__.core.invoke("library_save_cue", { id: cue.id, replace: false });
  } catch (e) {
    const msg = String(e);
    if (msg.startsWith("version:")) {
      // It came from one of the parish's copies: update it, or keep both.
      const [created] = msg.slice(8).split("|");
      const how = await choose("Save to Library", `This is the parish's copy of “${cue.title}” made ${dayLabel(created)}.`, [
        ["update", `Update the copy of ${dayLabel(created)}`],
        ["new", "Save as a new copy"],
      ]);
      if (!how) return;
      id = await call("library_save_cue", { id: cue.id, replace: false, version: how });
      if (id) toast(how === "update" ? `Updated the copy of ${dayLabel(created)}` : "Saved as a new copy. To use it in new services, set it as the default in the Library.");
      return;
    }
    if (!msg.startsWith("exists:")) return toast(msg, "error");
    const ok = await call("confirm", {
      title: "Replace library item",
      message: `Your library already has “${msg.slice(7)}”. Replace it with this version?`,
      okLabel: "Replace",
    });
    if (!ok) return;
    id = await call("library_save_cue", { id: cue.id, replace: true });
  }
  if (id && based) toast(`Saved a parish copy of “${cue.title}”, dated today. To use it in new services, set it as the default in the Library.`);
  else if (id) toast(`Saved “${libraryName(cue)}” to your library`);
}

/** What the library calls an item: a hymn by its own name or number, not
 *  its place in the service. */
function libraryName(cue) {
  const m = cue.content.document?.meta ?? {};
  if (cue.category !== "hymn") return cue.title;
  return m.title && m.title !== cue.title ? m.title : `Hymn ${m.hymn_number ?? ""}`.trim();
}

/** A small dialog of choices; resolves to the chosen key, or null. */
function choose(title, message, options) {
  return new Promise((resolve) => {
    const done = (v) => {
      overlay.remove();
      resolve(v);
    };
    const overlay = el(
      "div.modal-overlay",
      { onmousedown: (e) => e.target === overlay && done(null) },
      el(
        "div.modal",
        {},
        el("h3", {}, title),
        el("p", {}, message),
        el(
          "div.modal-actions",
          {},
          el("button", { onclick: () => done(null) }, "Cancel"),
          options.map(([k, l], i) => el(i === 0 ? "button.primary" : "button", { onclick: () => done(k) }, l)),
        ),
      ),
    );
    document.body.append(overlay);
  });
}

// ---------- Propers ----------

const KIND_LABELS = {
  collect: "Collect of the Day",
  first_reading: "First Reading",
  psalm: "Psalm",
  second_reading: "Second Reading",
  gospel: "Gospel",
  reading: "Reading",
};

/** The next Sunday (or today, if Sunday) as YYYY-MM-DD. */
function nextSunday() {
  const d = new Date();
  d.setDate(d.getDate() + ((7 - d.getDay()) % 7));
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

/**
 * The Propers dialog: the day's Collect, readings and psalm from the
 * lectionary (by date), or from a pasted lectionary page.
 */
export function openPropers() {
  const remember = (k, v) => { try { localStorage.setItem(k, v); } catch {} };
  const recall = (k, d) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };
  const svc = app.service;
  // The service's own choices win over the last ones used here.
  const serviceRite = svc ? (svc.rite === "I" ? "Rite I" : "Rite II") : null;
  const serviceTrack = svc ? `Track ${svc.track ?? 1}` : null;

  const results = el("div.propers-results");
  let parsed = null;

  /** Shows importable items with Collect (rite) and Readings (track) pickers. */
  const showItems = (data, note) => {
    parsed = data;
    if (!parsed?.items?.length) {
      results.replaceChildren(el("p.muted", {}, note ?? "No collect or readings found."));
      return;
    }
    const variantsOf = (pred) => [...new Set(parsed.items.filter(pred).map((i) => i.variant).filter(Boolean))];
    const rites = variantsOf((i) => i.kind === "collect");
    const tracks = variantsOf((i) => i.kind !== "collect" && /^Track/.test(i.variant ?? ""));
    const pick = (list, own, key, def) => {
      const want = own ?? recall(key, def);
      return list.includes(want) ? want : list.find((x) => x === def) ?? list[0];
    };
    const choice = {
      rite: pick(rites, serviceRite, "propers.rite", "Rite II"),
      track: pick(tracks, serviceTrack, "propers.track", "Track 1"),
    };
    const boxes = [];
    const applyChoice = () =>
      boxes.forEach(({ item, box }) => {
        const want = item.kind === "collect" ? choice.rite : /^Track/.test(item.variant ?? "") ? choice.track : null;
        box.checked = !item.alternative && (!want || item.variant === want);
      });
    const picker = (label, options, key, store) =>
      options.length > 1
        ? el(
            "label.field.inline",
            {},
            el("span", {}, label),
            el(
              "select",
              {
                onchange: (e) => {
                  choice[key] = e.target.value;
                  remember(store, e.target.value);
                  applyChoice();
                },
              },
              options.map((o) => el("option", { value: o, selected: o === choice[key] }, o)),
            ),
          )
        : null;

    const rows = parsed.items.map((item, i) => {
      const box = el("input", { type: "checkbox", dataset: { index: i } });
      boxes.push({ item, box });
      const kind = el(
        "select",
        { onchange: (e) => (item.kind = e.target.value), disabled: item.kind === "collect" },
        Object.entries(KIND_LABELS)
          .filter(([k]) => (item.kind === "collect") === (k === "collect"))
          .map(([k, l]) => el("option", { value: k, selected: k === item.kind }, l)),
      );
      const row = el(
        "label.propers-row",
        {},
        box,
        kind,
        el("span.propers-what", {}, item.kind === "collect" ? `${item.text.slice(0, 90)}…` : item.citation),
        item.variant ? el("span.badge", {}, item.variant) : null,
        item.alternative ? el("span.badge", {}, "or") : null,
      );
      if (item.kind !== "psalm") return row;
      if (!item.verses?.length) {
        return el(
          "div.propers-psalm",
          {},
          row,
          el("p.warn-note.propers-psalm-opts", {}, "No Prayer Book verses for this one. It will be added as a title slide; paste the text in the Editor (Psalm layout)."),
        );
      }
      // Psalm: verses become slides; optional antiphon.
      const antiphon = el("input.antiphon", {
        placeholder: "Antiphon (optional): sung by all at the start and end",
        oninput: (e) => (item.antiphon = e.target.value.trim() || null),
      });
      const every = el(
        "select",
        { onchange: (e) => (item.antiphon_every = Number(e.target.value)) },
        [[0, "start & end"], [2, "+ every 2 verses"], [4, "+ every 4 verses"], [6, "+ every 6 verses"]].map(([v, l]) =>
          el("option", { value: v }, l),
        ),
      );
      return el(
        "div.propers-psalm",
        {},
        row,
        el(
          "div.propers-psalm-opts",
          {},
          el("span.muted.small", {}, `${item.verses.length} verses → ${Math.ceil(item.verses.length / 2)} slides, choir plain / people bold`),
          antiphon,
          every,
        ),
      );
    });
    applyChoice();
    results.replaceChildren(
      el(
        "div.propers-top",
        {},
        parsed.day ? el("strong", {}, parsed.day) : el("span"),
        el("span.spacer"),
        picker("Collect", rites, "rite", "propers.rite"),
        picker("Readings", tracks, "track", "propers.track"),
      ),
      ...rows,
    );
  };

  // ---- from the lectionary ----
  const date = el("input", { type: "date", value: svc?.date ?? nextSunday() });
  const observance = el("select.observance");
  const dayInfo = el("span.muted.small");
  const load = async (which) => {
    if (!date.value) return;
    const dp = await call("lectionary_propers", { date: date.value, observance: which ?? null });
    if (!dp) return;
    dayInfo.textContent = `${dp.day.name} · Year ${dp.day.year}${dp.day.proper ? ` · Proper ${dp.day.proper}` : ""}`;
    observance.replaceChildren(
      ...(dp.observance ? [] : [el("option", { value: "", selected: true }, "— choose an observance —")]),
      ...dp.observances.map((o) => el("option", { value: o, selected: o === dp.observance }, o)),
    );
    showItems(
      { day: dp.title ?? dp.day.name, items: dp.items },
      dp.observance ? "This observance has no collect or readings in the lectionary." : "No Sunday or holy day on this date: choose an observance (a holy day, the Common of Saints, a lesser feast).",
    );
  };
  date.addEventListener("change", () => load());
  observance.addEventListener("change", () => load(observance.value || null));
  const lectionaryPane = el(
    "div.propers-pane",
    {},
    el(
      "div.propers-when",
      {},
      el("label.field.inline", {}, el("span", {}, "Date"), date),
      el("label.field.inline.grow", {}, el("span", {}, "Observance"), observance),
    ),
    dayInfo,
  );

  // ---- from a pasted page ----
  const text = el("textarea.propers-text", {
    placeholder:
      "On the lectionary page for the day, select everything (⌘A), copy (⌘C), and paste here (⌘V).\n\nThe app finds the day, the collect and the scripture citations.",
  });
  const pastePane = el(
    "div.propers-pane",
    { hidden: true },
    text,
    el(
      "div.modal-actions.left",
      {},
      el("button", { onclick: async () => showItems(await call("parse_propers", { text: text.value }), "No collect or scripture citations found in that text.") }, "Find readings & collect"),
    ),
  );
  const mode = (m) => {
    lectionaryPane.hidden = m !== "lectionary";
    pastePane.hidden = m !== "paste";
    tabs.querySelectorAll("button").forEach((b) => b.classList.toggle("active", b.dataset.mode === m));
    results.replaceChildren();
    parsed = null;
    if (m === "lectionary") load(observance.value || null);
  };
  const tabs = el(
    "div.seg.propers-modes",
    {},
    el("button.active", { dataset: { mode: "lectionary" }, onclick: () => mode("lectionary") }, "Lectionary"),
    el("button", { dataset: { mode: "paste" }, onclick: () => mode("paste") }, "Paste a page"),
  );

  const close = () => overlay.remove();
  const addChosen = async () => {
    if (!parsed) return;
    if (!app.service) return toast("Open a service first (☰ Services).", "warn");
    const chosen = [...results.querySelectorAll("input[type=checkbox]")]
      .filter((c) => c.checked)
      .map((c) => parsed.items[Number(c.dataset.index)]);
    if (!chosen.length) return;
    const ids = await call("add_propers", { items: chosen, day: parsed.day ?? null, after: insertionPoint() });
    if (ids?.length) {
      app.select(ids[0]);
      toast(`Added ${ids.length} item${ids.length === 1 ? "" : "s"} to the service`);
      close();
    }
  };

  const overlay = el(
    "div.modal-overlay",
    { onmousedown: (e) => e.target === overlay && close() },
    el(
      "div.modal.wide",
      {},
      el("div.propers-head", {}, el("h3", {}, "Propers"), tabs),
      el(
        "p.muted.small",
        {},
        "The Collect of the Day (Rite I or II), the readings and the psalm (verses from the Prayer Book) for a date. Tick the items to add; they go after the selected cue.",
      ),
      lectionaryPane,
      pastePane,
      results,
      el("div.modal-actions", {}, el("button", { onclick: close }, "Cancel"), el("button.primary", { onclick: addChosen }, "Add to service")),
    ),
  );
  document.body.append(overlay);
  load();
}
