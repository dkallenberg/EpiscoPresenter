// Left pane: the service cue timeline.

import { send } from "/js/bridge.js";
import { openPropers } from "/js/control/library.js";
import { addGoogleSlides, addPdfSlides, addSlides, addWebVideo } from "/js/control/made.js";
import { backgroundName } from "/js/control/looks.js";
import { onDeckChange, pageCount } from "/js/deckpdf.js";
import { app, call, el, reportWarnings, slideCount, titleCase } from "/js/control/util.js";

const ICONS = {
  hymn: "♫",
  psalm: "℣",
  reading: "📖",
  gospel: "✠",
  eucharistic_prayer: "✚",
  collect: "🙏",
  prayer: "🙏",
  acclamation: "☩",
  homily: "🎙",
  anthem: "♬",
  announcements: "▤",
  other: "•",
};

const NEW_ITEMS = [
  ["prayer", "Prayer / dialogue"],
  ["collect", "Collect"],
  ["hymn", "Hymn"],
  ["reading", "Reading"],
  ["psalm", "Psalm"],
  ["announcements", "Announcements (Google Slides)"],
];

export function initTimeline() {
  const list = document.getElementById("cue-list");
  const addBtn = document.getElementById("btn-add");
  const menu = document.getElementById("add-menu");

  // Add menu
  menu.replaceChildren(
    el("button", { onclick: () => document.querySelector('.tabs button[data-tab="library"]').click() }, "From library…"),
    el("button", { onclick: () => openPropers() }, "Propers (Collect, readings, psalm)…"),
    el("button", { onclick: () => addFiles() }, "Add .ewm files…"),
    el("button", { onclick: () => addBlank(), title: "The sanctuary screen goes black; the stream shows the camera" }, "Blank (sanctuary only)"),
    el("hr"),
    el("button", { onclick: () => addSlides(insertionPoint()), title: "Title and information slides, with text or pictures" }, "New slides (slide maker)"),
    el("button", { onclick: () => addPdfSlides(insertionPoint()), title: "A PowerPoint, Keynote or Google Slides presentation saved as PDF" }, "Slides from a PDF (PowerPoint, Keynote)…"),
    el("button", { onclick: () => addGoogleSlides(insertionPoint()), title: "A Google Slides presentation, by its link" }, "Google Slides presentation…"),
    el("button", { onclick: () => addWebVideo(insertionPoint()), title: "Plays full screen from YouTube, Facebook or Instagram" }, "Web video (YouTube, Facebook, Instagram)…"),
    el("hr"),
    ...NEW_ITEMS.map(([key, label]) => el("button", { onclick: () => newItem(key) }, `New ${label}`)),
  );
  addBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    menu.hidden = !menu.hidden;
  });
  document.addEventListener("click", () => (menu.hidden = true));

  // Drag to reorder. Pointer events rather than HTML5 drag-and-drop, which
  // the app's webviews do not deliver reliably; the list is not redrawn
  // while a drag is in progress (the on-air state changes constantly).
  let drag = null; // { id, row, startY, active, target, after }
  const clearMarks = () =>
    list.querySelectorAll(".dragging, .drop-before, .drop-after").forEach((n) => n.classList.remove("dragging", "drop-before", "drop-after"));
  list.addEventListener("pointerdown", (e) => {
    const row = e.target.closest(".cue[data-id]");
    if (!row || app.readonly || e.button !== 0 || e.target.closest("button")) return;
    drag = { id: row.dataset.id, row, startY: e.clientY, active: false, target: null, after: false, pointer: e.pointerId };
  });
  window.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.pointer) return;
    if (!drag.active) {
      if (Math.abs(e.clientY - drag.startY) < 6) return;
      drag.active = true;
      drag.row.classList.add("dragging");
      document.body.classList.add("reordering");
    }
    e.preventDefault();
    // Scroll the list when dragging near its edges.
    const box = list.getBoundingClientRect();
    if (e.clientY < box.top + 30) list.scrollTop -= 12;
    else if (e.clientY > box.bottom - 30) list.scrollTop += 12;
    const over = document.elementFromPoint(box.left + box.width / 2, e.clientY)?.closest(".cue[data-id]");
    list.querySelectorAll(".drop-before, .drop-after").forEach((n) => n.classList.remove("drop-before", "drop-after"));
    if (!over || !list.contains(over)) {
      drag.target = null;
      return;
    }
    const r = over.getBoundingClientRect();
    drag.target = over.dataset.id;
    drag.after = e.clientY > r.top + r.height / 2;
    over.classList.add(drag.after ? "drop-after" : "drop-before");
  });
  const endDrag = (e) => {
    if (!drag || (e && e.pointerId !== drag.pointer)) return;
    const d = drag;
    drag = null;
    clearMarks();
    document.body.classList.remove("reordering");
    if (d.active) {
      // The click that ends a drag must not also select or go live.
      const swallow = (ev) => { ev.stopPropagation(); ev.preventDefault(); };
      window.addEventListener("click", swallow, { capture: true, once: true });
      setTimeout(() => window.removeEventListener("click", swallow, { capture: true }), 0);
      const cues = app.service?.cues ?? [];
      const from = cues.findIndex((c) => c.id === d.id);
      if (d.target && from >= 0) {
        let to = cues.findIndex((c) => c.id === d.target) + (d.after ? 1 : 0);
        if (from < to) to -= 1; // removing the dragged cue shifts later indexes
        if (to !== from) call("move_cue", { id: d.id, to });
      }
    }
    if (pending) {
      pending = false;
      render(list);
    }
  };
  window.addEventListener("pointerup", endDrag);
  window.addEventListener("pointercancel", endDrag);
  let pending = false;

  onDeckChange(() => !drag?.active && render(list));
  app.on((what) => {
    if (what === "link") return;
    if (drag?.active) {
      pending = true;
      return;
    }
    render(list);
  });
}

async function addFiles() {
  const r = await call("add_items", { after: insertionPoint(), paths: null });
  if (r?.opened) {
    reportWarnings(r.warnings);
    if (r.ids.length) app.select(r.ids[0]);
  }
}

async function addBlank() {
  const id = await call("blank_add", { after: insertionPoint(), title: null });
  if (id) app.select(id);
}

async function newItem(category) {
  const id = await call("new_item", { after: insertionPoint(), category });
  if (id) {
    app.select(id);
    document.querySelector('.tabs button[data-tab="editor"]')?.click();
  }
}

/** New items go after the selected cue, else at the end. */
function insertionPoint() {
  const i = app.selectedIndex();
  return i >= 0 ? i : null;
}

function render(list) {
  const service = app.service;
  const cues = service?.cues ?? [];
  const active = app.state.active;
  document.getElementById("cue-empty").hidden = cues.length > 0 || !!service;
  document.getElementById("btn-add").disabled = app.readonly;

  const rows = [];
  if (service) rows.push(preServiceRow(service, active));
  cues.forEach((cue, i) => rows.push(cueRow(cue, i, active, cues.length)));
  if (service && !cues.length) {
    rows.push(el("li.cue-hint.muted", {}, "This service is empty. Use + Add above for prayers, hymns, readings and slides, or the Library tab."));
  }
  list.replaceChildren(...rows);
  list.querySelector(".cue.on-air")?.scrollIntoView({ block: "nearest" });
}

function preServiceRow(service, active) {
  const deck = Boolean(service.announcements_deck);
  const live = !active;
  const pages = deck ? pageCount(service.announcements_deck) : 0;
  return el(
    "li.cue.pre-service" + (live ? ".on-air" : ""),
    {
      title: deck
        ? `The announcements (Google Slides), repeating every 9 seconds${pages ? `, ${pages} slides` : ""}, until the service starts.\nDouble-click to go back to them.`
        : "Before the service. No announcements deck yet: add its Google Slides link in ⚙ Settings → Service settings.",
      ondblclick: () => send({ type: "reset" }),
    },
    el("span.cue-num", {}, "–"),
    el("div.cue-body", {}, el("div.cue-title", {}, el("span.cue-icon", {}, ICONS.announcements), "Before the service")),
    el("div.cue-icons", {}, live ? el("span.badge.live", {}, "On screen") : null),
  );
}

/** Everything about an item that the row itself leaves out. */
function rowDetails(cue, n, i, count) {
  const isAnn = cue.content.kind === "announcements";
  const mode = isAnn ? cue.content.mode : null;
  const kind = isAnn
    ? mode === "loop"
      ? i === count - 1
        ? "Announcements after the service (repeating)"
        : "Announcements (repeating)"
      : "Announcements, one slide at a time"
    : titleCase(cue.category);
  return [
    `${kind} · ${n} slide${n === 1 ? "" : "s"}`,
    cue.caption && cue.caption !== cue.title ? cue.caption : null,
    isAnn ? null : `Sanctuary: ${backgroundName(cue.sanctuary.background)}`,
    isAnn ? null : `Live stream: ${backgroundName(cue.stream.background)}`,
    cue.modified ? "Changes not saved yet" : null,
    "Click to select · double-click to show it",
  ]
    .filter(Boolean)
    .join("\n");
}

function cueRow(cue, i, active, count) {
  const onAir = active?.cue === i;
  const n = slideCount(cue);

  const icons = el(
    "div.cue-icons",
    {},
    onAir ? el("span.badge.live", {}, n > 1 ? `On screen ${active.slide + 1}/${n}` : "On screen") : null,
    cue.modified ? el("span.badge.modified", { title: "Changes not saved yet" }, "●") : null,
  );

  const tools = app.readonly
    ? null
    : el(
        "div.cue-tools",
        {},
        el("button.icon", { title: "Move up", disabled: i === 0, onclick: stop(() => call("move_cue", { id: cue.id, to: i - 1 })) }, "↑"),
        el(
          "button.icon",
          { title: "Move down", disabled: i === count - 1, onclick: stop(() => call("move_cue", { id: cue.id, to: i + 1 })) },
          "↓",
        ),
        el("button.icon.danger", { title: "Remove from service", onclick: stop(() => removeCue(cue)) }, "✕"),
      );

  return el(
    "li.cue" + (onAir ? ".on-air" : "") + (app.selectedId === cue.id ? ".selected" : ""),
    {
      dataset: { id: cue.id },
      title: rowDetails(cue, n, i, count),
      onclick: () => app.select(cue.id),
      ondblclick: () => send({ type: "go_to", cue: i, slide: 0 }),
    },
    el("span.cue-num", {}, String(i + 1)),
    el("div.cue-body", {}, el("div.cue-title", {}, el("span.cue-icon", {}, ICONS[cue.category] ?? "•"), cue.title)),
    icons,
    tools,
  );
}

const stop = (fn) => (e) => {
  e.stopPropagation();
  fn();
};

async function removeCue(cue) {
  if (cue.modified) {
    const ok = await call("confirm", {
      title: "Remove item",
      message: `“${cue.title}” has unsaved changes. Remove it from the service anyway?`,
      okLabel: "Remove",
    });
    if (!ok) return;
  }
  if (app.selectedId === cue.id) app.select(null);
  await call("remove_cue", { id: cue.id });
}
