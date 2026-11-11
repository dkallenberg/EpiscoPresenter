// Center pane, "Library" tab: reusable items (built-in Rite II texts and
// the operator's own saved items), browsable by type, with previews; plus the
// "Import propers" dialog for readings and collects pasted from the lectionary.

import { mountStage, renderOutput } from "/js/render.js";
import { app, call, el, titleCase, toast } from "/js/control/util.js";

const lib = { entries: [], category: "all", query: "", selected: null, mine: false };

/** New items go after the selected cue, else at the end. */
function insertionPoint() {
  const i = app.selectedIndex();
  return i >= 0 ? i : null;
}

export function initLibrary() {
  document.querySelector('.tabs button[data-tab="library"]').addEventListener("click", refresh);
}

export async function refresh() {
  lib.entries = (await call("library_list")) ?? [];
  render();
}

function matches(e) {
  if (lib.mine && e.builtin) return false;
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
  );
  const list = el(
    "div.lib-list",
    {},
    shown.length
      ? shown.map((e) =>
          el(
            `div.lib-row${e.id === lib.selected ? ".selected" : ""}`,
            {
              onclick: () => { lib.selected = e.id; render(); },
              ondblclick: () => add([e.id]),
            },
            el(
              "div.lib-title",
              {},
              e.title,
              e.builtin ? el("span.badge", {}, "Built-in") : el("span.badge.own", {}, "Mine"),
            ),
            el("div.lib-meta", {}, [titleCase(e.category), e.citation, e.season, `${e.slides} slide${e.slides === 1 ? "" : "s"}`].filter(Boolean).join(" · ")),
            el("div.lib-summary", {}, e.summary),
          ),
        )
      : el("p.muted", {}, "Nothing matches."),
  );
  const preview = el("div.lib-preview#lib-preview");
  root.replaceChildren(
    el(
      "div.lib-side",
      {},
      el(
        "div.lib-toolbar",
        {},
        search,
        el("button.small", { title: "Paste readings and the collect from the lectionary page", onclick: openPropers }, "Import propers…"),
      ),
      chips,
      list,
      el("p.hint.muted", {}, "Double-click an item to add it after the selected cue. Save your own items from the Editor with “Save to Library”."),
    ),
    preview,
  );
  if (lib.selected) showPreview(lib.selected);
}

async function showPreview(id) {
  const host = document.getElementById("lib-preview");
  const entry = lib.entries.find((e) => e.id === id);
  const cue = await call("library_preview", { id });
  if (!cue || lib.selected !== id) return;
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
        el("button.primary", { disabled: app.readonly || !app.service, onclick: () => add([id]) }, "Add to service"),
      ),
    ),
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

async function add(ids) {
  if (!app.service) return toast("Open a service first (☰ Services).", "warn");
  const added = await call("library_add", { ids, after: insertionPoint() });
  if (added?.length) {
    app.select(added[0]);
    toast(`Added to the service: ${lib.entries.find((e) => e.id === ids[0])?.title ?? ""}`);
  }
}

async function remove(entry) {
  const ok = await call("confirm", { title: "Delete from library", message: `Delete “${entry.title}” from your library?`, okLabel: "Delete" });
  if (!ok) return;
  await call("library_delete", { id: entry.id });
  await refresh();
}

/** "Save to Library" for the cue in the editor. */
export async function saveCueToLibrary(cue) {
  let id;
  try {
    id = await window.__TAURI__.core.invoke("library_save_cue", { id: cue.id, replace: false });
  } catch (e) {
    const msg = String(e);
    if (!msg.startsWith("exists:")) return toast(msg, "error");
    const ok = await call("confirm", {
      title: "Replace library item",
      message: `Your library already has “${msg.slice(7)}”. Replace it with this version?`,
      okLabel: "Replace",
    });
    if (!ok) return;
    id = await call("library_save_cue", { id: cue.id, replace: true });
  }
  if (id) toast(`Saved “${cue.title}” to your library`);
}

// ---------- Import propers ----------

const KIND_LABELS = {
  collect: "Collect of the Day",
  first_reading: "First Reading",
  psalm: "Psalm",
  second_reading: "Second Reading",
  gospel: "Gospel",
  reading: "Reading",
};

export function openPropers() {
  const text = el("textarea.propers-text", {
    placeholder:
      "On the lectionary page for the day, select everything (⌘A), copy (⌘C), and paste here (⌘V).\n\nThe app finds the day, the collect and the scripture citations. Reading texts are not imported: the screens show the citation and responses.",
  });
  const results = el("div.propers-results");
  let parsed = null;

  const remember = (k, v) => { try { localStorage.setItem(k, v); } catch {} };
  const recall = (k, d) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };

  const find = async () => {
    parsed = await call("parse_propers", { text: text.value });
    if (!parsed) return;
    if (!parsed.items.length) {
      results.replaceChildren(el("p.muted", {}, "No collect or scripture citations found in that text."));
      return;
    }
    const variantsOf = (pred) => [...new Set(parsed.items.filter(pred).map((i) => i.variant).filter(Boolean))];
    const rites = variantsOf((i) => i.kind === "collect");
    const tracks = variantsOf((i) => i.kind !== "collect");
    const choice = {
      rite: rites.includes(recall("propers.rite", "Rite II")) ? recall("propers.rite", "Rite II") : rites.at(-1),
      track: tracks.includes(recall("propers.track", "Track 1")) ? recall("propers.track", "Track 1") : tracks[0],
    };
    const boxes = [];
    const applyChoice = () =>
      boxes.forEach(({ item, box }) => {
        const pick = item.kind === "collect" ? choice.rite : choice.track;
        box.checked = !item.alternative && (!item.variant || item.variant === pick);
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
      if (item.kind === "psalm" && !item.verses?.length) {
        return el(
          "div.propers-psalm",
          {},
          row,
          el(
            "p.warn-note.propers-psalm-opts",
            {},
            "No verses were found for this psalm in the pasted text. It will be added as a title slide; open it in the Editor and paste the verses into Psalm layout.",
          ),
        );
      }
      if (item.kind !== "psalm") return row;
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
          el("span.muted.small", {}, `${item.verses.length} verses → ${Math.ceil(item.verses.length / 2)} slides, choir plain / congregation bold`),
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
      el("h3", {}, "Import propers"),
      el(
        "p.muted.small",
        {},
        "Paste the day's page from lectionary.episcopalchurch.org (or any list of readings). Tick the items to add; they go after the selected cue.",
      ),
      text,
      el("div.modal-actions.left", {}, el("button", { onclick: find }, "Find readings & collect")),
      results,
      el("div.modal-actions", {}, el("button", { onclick: close }, "Cancel"), el("button.primary", { onclick: addChosen }, "Add to service")),
    ),
  );
  document.body.append(overlay);
  text.focus();
}
