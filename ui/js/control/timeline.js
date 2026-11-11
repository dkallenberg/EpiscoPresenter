// Left pane: the service cue timeline.

import { send } from "/js/bridge.js";
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
    el("button", { onclick: () => addFiles() }, "Add .ewm files…"),
    el("hr"),
    ...NEW_ITEMS.map(([key, label]) => el("button", { onclick: () => newItem(key) }, `New ${label}`)),
  );
  addBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    menu.hidden = !menu.hidden;
  });
  document.addEventListener("click", () => (menu.hidden = true));

  // Drag-and-drop reordering (event delegation on the list).
  let dragId = null;
  list.addEventListener("dragstart", (e) => {
    const row = e.target.closest(".cue[data-id]");
    if (!row || app.readonly) return e.preventDefault();
    dragId = row.dataset.id;
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", dragId);
    row.classList.add("dragging");
  });
  list.addEventListener("dragend", () => {
    dragId = null;
    list.querySelectorAll(".dragging, .drop-before, .drop-after").forEach((n) =>
      n.classList.remove("dragging", "drop-before", "drop-after"),
    );
  });
  list.addEventListener("dragover", (e) => {
    const row = e.target.closest(".cue[data-id]");
    if (!dragId || !row) return;
    e.preventDefault();
    const after = e.offsetY > row.offsetHeight / 2;
    list.querySelectorAll(".drop-before, .drop-after").forEach((n) => n.classList.remove("drop-before", "drop-after"));
    row.classList.add(after ? "drop-after" : "drop-before");
  });
  list.addEventListener("drop", (e) => {
    const row = e.target.closest(".cue[data-id]");
    if (!dragId || !row) return;
    e.preventDefault();
    const cues = app.service.cues;
    const from = cues.findIndex((c) => c.id === dragId);
    let to = cues.findIndex((c) => c.id === row.dataset.id) + (row.classList.contains("drop-after") ? 1 : 0);
    if (from < to) to -= 1; // removing the dragged cue shifts later indexes
    if (from !== to) call("move_cue", { id: dragId, to });
  });

  app.on((what) => {
    if (what !== "link") render(list);
  });
}

async function addFiles() {
  const r = await call("add_items", { after: insertionPoint(), paths: null });
  if (r?.opened) {
    reportWarnings(r.warnings);
    if (r.ids.length) app.select(r.ids[0]);
  }
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
  list.replaceChildren(...rows);
  list.querySelector(".cue.on-air")?.scrollIntoView({ block: "nearest" });
}

function preServiceRow(service, active) {
  const deck = Boolean(service.announcements_deck);
  const live = !active;
  return el(
    "li.cue.pre-service" + (live ? ".on-air" : ""),
    { title: deck ? service.announcements_deck : "No announcements deck configured", ondblclick: () => send({ type: "reset" }) },
    el("span.cue-num", {}, "–"),
    el(
      "div.cue-body",
      {},
      el("div.cue-title", {}, "Pre-Service"),
      el("div.cue-meta", {}, "Announcements loop · 9 s"),
    ),
    el(
      "div.cue-icons",
      {},
      live ? el("span.badge.live", {}, "LIVE") : null,
      el(`span.badge.gs${deck ? (live ? ".playing" : "") : ".off"}`, {}, deck ? (live ? "▶ GS loop" : "GS") : "GS —"),
    ),
  );
}

function cueRow(cue, i, active, count) {
  const onAir = active?.cue === i;
  const n = slideCount(cue);
  const isAnn = cue.content.kind === "announcements";
  const mode = isAnn ? cue.content.mode : null;
  const colorKey = (cue.color ?? "none").toLowerCase();

  const icons = el(
    "div.cue-icons",
    {},
    onAir ? el("span.badge.live", {}, `LIVE ${n > 1 ? `${active.slide + 1}/${n}` : ""}`) : null,
    cue.modified ? el("span.badge.modified", { title: "Unsaved changes" }, "●") : null,
    isAnn
      ? el(
          `span.badge.gs${onAir ? ".playing" : ""}`,
          { title: `Google Slides · ${mode === "loop" ? "auto-advance loop" : "single pass, manual advance"}` },
          `${onAir ? "▶ " : ""}GS ${mode === "loop" ? "loop" : "1×"}`,
        )
      : el("span.badge.count", { title: `${n} slide${n === 1 ? "" : "s"}` }, `${cue.content.slides.length}`),
    el(`span.swatch.color-${colorKey}`, {
      title: `${cue.color ?? "No colour"}${cue.background ? ` · ${cue.background}` : ""}`,
    }),
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
      draggable: !app.readonly,
      onclick: () => app.select(cue.id),
      ondblclick: () => send({ type: "go_to", cue: i, slide: 0 }),
    },
    el("span.cue-num", {}, String(i + 1)),
    el(
      "div.cue-body",
      {},
      el("div.cue-title", {}, el("span.cue-icon", {}, ICONS[cue.category] ?? "•"), cue.title),
      el(
        "div.cue-meta",
        {},
        mode === "loop" && i === count - 1 ? "Post-service loop" : titleCase(cue.category),
        cue.caption && cue.caption !== cue.title ? ` · ${cue.caption}` : "",
      ),
    ),
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
