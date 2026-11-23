// Operator control window: boot, top bar, tabs and keyboard shortcuts.

import { doneLibraryItem, isLibraryEdit, saveLibraryItem } from "/js/control/libedit.js";
import { connectStore, inTauri, invoke, listen, send, setPlannerMode } from "/js/bridge.js";
import { initBulletin, openParish } from "/js/control/bulletin.js";
import { initStorage } from "/js/control/storage.js";
import { initEditor } from "/js/control/editor.js";
import { initHandoff, initHome, showHome } from "/js/control/home.js";
import { initInUse, viewOnly } from "/js/control/inuse.js";
import { initUndo } from "/js/control/undo.js";
import { initUpdates, openUpdates } from "/js/control/update.js";
import { openLayouts } from "/js/control/layouts.js";
import { initOutputs } from "/js/control/outputs.js";
import { deckError, load as loadDeck, onDeckChange, pageCount } from "/js/deckpdf.js";
import { initLibrary } from "/js/control/library.js";
import { openServiceSettings } from "/js/control/looks.js";
import { initSlides } from "/js/control/slides.js";
import { initStage } from "/js/control/stage.js";
import { initTimeline } from "/js/control/timeline.js";
import { app, call, friendly, reportWarnings, toast } from "/js/control/util.js";
import { openConnection } from "/js/control/connection.js";

const $ = (id) => document.getElementById(id);

// ---------- tabs ----------
document.body.classList.add("slides-tab");
document.querySelectorAll(".tabs button[data-tab]").forEach((b) =>
  b.addEventListener("click", () => {
    document.querySelectorAll(".tabs button[data-tab], .tab").forEach((x) => x.classList.remove("active"));
    b.classList.add("active");
    $(`tab-${b.dataset.tab}`).classList.add("active");
    document.body.classList.toggle("slides-tab", b.dataset.tab === "slides");
  }),
);

// EpiscoPlanner: day and night colours (the presenter is always dark).
{
  const btn = document.getElementById("btn-theme");
  const show = () => {
    const day = document.body.classList.contains("day");
    btn.textContent = day ? "☾ Night colours" : "☀ Day colours";
    btn.title = day ? "Switch to night colours" : "Switch to day colours";
  };
  btn?.addEventListener("click", () => {
    const day = document.body.classList.toggle("day");
    try {
      localStorage.setItem("planner.theme", day ? "day" : "night");
    } catch (e) {}
    show();
  });
  if (btn) show();
}

// EpiscoPlanner opens on the bulletin straight away (it says so before the
// page loads; see run_planner).
if (window.__EPISCO_MODE__ === "planner") document.querySelector('.tabs button[data-tab="bulletin"]')?.click();

// ---------- file actions ----------
const hasUnsaved = () => app.service && (app.service.modified || app.service.cues.some((c) => c.modified));
let saveError = null;

$("btn-services").addEventListener("click", () => showHome());
$("lib-edit-save").addEventListener("click", () => saveLibraryItem());
$("btn-updates").addEventListener("click", () => openUpdates());
$("lib-edit-done").addEventListener("click", () => doneLibraryItem());
$("btn-save").addEventListener("click", () => saveService(true));
$("save-status").addEventListener("click", async () => {
  // A service with no file (the example) can be kept as a scheduled service.
  if (app.service && !app.service.plan_path) {
    const today = new Date().toISOString().slice(0, 10);
    const path = await call("service_keep", { date: app.service.date ?? today });
    if (path) toast("Kept. This service now saves automatically.");
  } else if (saveError) {
    toast(friendly(saveError), "error");
  }
});
listen("episco://autosave", (s) => {
  saveError = s.ok ? null : s.error;
  renderTopbar();
});
$("btn-settings").addEventListener("click", () => openServiceSettings());
$("btn-layouts").addEventListener("click", openLayouts);
$("btn-connection").addEventListener("click", () => openConnection());
$("btn-parish").addEventListener("click", () => openParish());
initOutputs();

// ⚙ Settings: one menu for everything not used every week. Choosing an
// entry closes it (the day/night switch leaves it open to see the change).
{
  const menu = $("settings-menu");
  const close = () => (menu.hidden = true);
  $("btn-menu").addEventListener("click", (e) => {
    e.stopPropagation();
    menu.hidden = !menu.hidden;
  });
  menu.addEventListener("click", (e) => {
    if (e.target.closest("button") && e.target.closest("button").id !== "btn-theme") close();
    else e.stopPropagation();
  });
  document.addEventListener("click", close);
  document.addEventListener("keydown", (e) => e.key === "Escape" && close());
}

// The announcements deck (and Google Slides pages in made slides): shown at
// once from this computer's copy, then downloaded again once per run, so
// both computers show the deck as it is now (an old copy numbers its slides
// differently). The slide count is recorded so a single pass steps through
// every slide; the stream computer downloads again when the sanctuary
// computer counts a different number.
let deckUrl = null;
const refreshed = new Set();
const refreshDeck = (url) => {
  if (refreshed.has(url)) return;
  refreshed.add(url);
  invoke("deck_fetch", { url, refresh: true }).catch(() => {});
};
const syncDeckCount = (url) => {
  const n = pageCount(url);
  const s = app.service;
  if (!n || !s || s.announcements_deck !== url || s.deck_slides === n) return;
  if (!app.readonly) call("deck_set_slides", { slides: n });
  else if (s.deck_slides && !refreshed.has(`${url}#${s.deck_slides}`)) {
    refreshed.add(`${url}#${s.deck_slides}`);
    invoke("deck_fetch", { url, refresh: true }).catch(() => {});
  }
};
const madeDecks = (service) => {
  const urls = new Set();
  for (const cue of service?.cues ?? []) {
    if (cue.content.kind !== "liturgy") continue;
    for (const slide of cue.content.slides ?? []) {
      for (const b of slide.blocks ?? []) {
        const m = /^deck:(.*)#page=\d+$/.exec(b.src ?? "");
        if (m) urls.add(m[1]);
      }
    }
  }
  return urls;
};
app.on((what) => {
  if (what !== "service") return;
  const url = app.service?.announcements_deck ?? null;
  if (url && url !== deckUrl)
    loadDeck(url, true).then((pages) => {
      // Say why, so it can be put right (deckpdf.js).
      if (!pages && deckError(url)) toast(`The announcements' slides can't be shown on this computer: ${deckError(url)}`, "warn");
      syncDeckCount(url);
      refreshDeck(url);
    });
  deckUrl = url;
  madeDecks(app.service).forEach(refreshDeck);
  if (url) syncDeckCount(url);
});
onDeckChange(syncDeckCount);
document.addEventListener("keydown", (e) => {
  // ⌘S saves the selected item (editor.js); ⌥⌘S saves the whole service.
  if ((e.metaKey || e.ctrlKey) && e.altKey && e.key.toLowerCase() === "s") {
    e.preventDefault();
    e.stopImmediatePropagation();
    saveService(e.shiftKey);
  }
}, true);

async function saveService(saveAs) {
  const path = await call("save_service", { saveAs });
  if (path) toast(saveAs ? `Saved a copy to ${path}` : "Saved");
}

// ---------- presentation shortcuts ----------
document.addEventListener("keydown", (e) => {
  if (e.target.closest("textarea, input, select, [contenteditable='true']") || e.metaKey || e.ctrlKey) return;
  // Never drive the screens while a dialog or picker is open: stray typing
  // there would otherwise advance slides or black out the outputs.
  if (document.querySelector(".modal-overlay, .bg-picker, .layout-tool, .outputs-panel")) return;
  const cmd = {
    ArrowRight: "next",
    ArrowDown: "next",
    PageDown: "next",
    " ": "next",
    ArrowLeft: "prev",
    ArrowUp: "prev",
    PageUp: "prev",
    b: "toggle_blackout",
    Escape: "clear_text",
    m: "clear_media",
  }[e.key];
  if (!cmd) return;
  e.preventDefault();
  send({ type: cmd }).catch(console.error);
});

// ---------- top bar ----------
function renderTopbar() {
  const s = app.service;
  const isTemplate = /[\\/]templates[\\/][^\\/]+[\\/]service\.toml$/.test(s?.plan_path ?? "");
  // A Library item being edited on its own (libedit.js).
  const libEdit = isLibraryEdit(s);
  $("service-title").textContent = s ? `${isTemplate ? "Template: " : libEdit ? "Library: " : ""}${s.title || "Untitled service"}` : "";
  $("service-when").textContent = s && !isTemplate && !libEdit ? when(s.date, s.time) : "";
  $("lib-edit-bar").hidden = !libEdit;
  document.body.classList.toggle("library-edit", libEdit);
  const status = $("save-status");
  status.hidden = !s || app.readonly || libEdit;
  if (s) {
    const [text, cls, title] = !s.plan_path
      ? ["Not saved yet · Keep this service", "warn", "This service has no file yet (an example or a copy). Click to keep it with your services; from then on it saves by itself."]
      : saveError
        ? ["⚠ Could not save · Details", "error", `${friendly(saveError)}\nClick for details.`]
        : hasUnsaved()
          ? ["Saving…", "pending", "Changes save by themselves a moment after you stop editing."]
          : ["Saved ✓", "ok", `Saved by itself to ${s.plan_path}`];
    status.textContent = text;
    status.className = `save-status ${cls}`;
    status.title = title;
  }
  // View only because it is open elsewhere still leaves the other services.
  $("btn-services").disabled = app.snap.link?.role === "slave";
  $("btn-settings").disabled = app.readonly || !s || libEdit;
  $("btn-save").disabled = app.readonly || !s;

  const { text, ok, warn } = linkWords(app.snap.link ?? {});
  const link = app.snap.link ?? {};
  $("link").textContent = text;
  $("link").title = [
    link.endpoint ? `Address: ${link.endpoint}` : "",
    link.latency_us != null ? `Delay: ${(link.latency_us / 1000).toFixed(1)} ms` : "",
    link.error ?? "",
    "Click to set up the connection with the other computer.",
  ]
    .filter(Boolean)
    .join("\n");
  $("link").classList.toggle("ok", ok);
  $("link").classList.toggle("warn", Boolean(warn));
}

/** The connection, in a parish's words. */
export function linkWords(link) {
  if (link.role === "slave") {
    return link.online
      ? { text: `Following ${link.peer || "the sanctuary computer"} ✓`, ok: true }
      : { text: "Not connected to the sanctuary computer", ok: false, warn: true };
  }
  if (link.role === "master") {
    if (link.peer) return { text: "Live stream connected ✓", ok: true };
    return link.online ? { text: "Waiting for the live-stream computer", ok: false } : { text: "Network not available", ok: false, warn: true };
  }
  return { text: "One computer", ok: true };
}

/** "Sun 15 Nov · 10:30 am" from a service's date and time. */
export function when(date, time) {
  const parts = [];
  if (date) {
    const [y, m, d] = date.split("-").map(Number);
    const day = new Date(y, m - 1, d);
    parts.push(day.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" }));
  }
  if (time) {
    const [h, min] = time.split(":").map(Number);
    parts.push(`${((h + 11) % 12) + 1}:${String(min).padStart(2, "0")} ${h < 12 ? "am" : "pm"}`);
  }
  return parts.join(" · ");
}

// ---------- boot ----------
$("link").addEventListener("click", () => openConnection());
$("link").setAttribute("role", "button");
initTimeline();
initSlides();
initEditor();
initLibrary();
initHome();
initStage();
initBulletin();
app.on(renderTopbar);

if (!inTauri) {
  $("link").textContent = "Open this page inside the EpiscoPresenter app";
} else {
  // EpiscoPlanner: the same window without the output screens and live controls.
  // The planner announces itself before the page loads; ask only otherwise.
  app.mode = window.__EPISCO_MODE__ ?? (await invoke("app_mode").catch(() => "presenter")) ?? "presenter";
  if (app.mode === "planner") {
    document.body.classList.add("planner");
    setPlannerMode(true);
    document.querySelector(".brand").textContent = "EpiscoPlanner";
    document.title = "EpiscoPlanner";
    // The planner opens on the bulletin; the slides are a tab away.
    document.querySelector('.tabs button[data-tab="bulletin"]')?.click();
  }
  initStorage();
  initHandoff();
  initInUse();
  initUndo();
  initUpdates();
  app.vocab = await invoke("vocabulary");
  let last = { service: undefined, state: undefined, link: undefined };
  connectStore((snap) => {
    app.snap = snap;
    app.readonly = snap.link?.role === "slave" || viewOnly();
    document.body.classList.toggle("readonly", app.readonly);
    // Keep a valid selection: default to the on-air cue, else the first.
    const cues = snap.service?.cues ?? [];
    if (!cues.some((c) => c.id === app.selectedId)) {
      app.selectedId = cues[snap.state.active?.cue ?? 0]?.id ?? null;
    }
    const what = snap.service !== last.service ? "service" : snap.state !== last.state ? "state" : "link";
    last = { service: snap.service, state: snap.state, link: snap.link };
    app.emit(what);
  });
}
