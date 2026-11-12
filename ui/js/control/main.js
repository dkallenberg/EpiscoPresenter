// Operator control window: boot, top bar, tabs and keyboard shortcuts.

import { connectStore, inTauri, invoke, listen, send, setPlannerMode } from "/js/bridge.js";
import { initBulletin } from "/js/control/bulletin.js";
import { initStorage } from "/js/control/storage.js";
import { initEditor } from "/js/control/editor.js";
import { initHandoff, initHome, showHome } from "/js/control/home.js";
import { openLayouts } from "/js/control/layouts.js";
import { initOutputs } from "/js/control/outputs.js";
import { load as loadDeck, onDeckChange, pageCount } from "/js/deckpdf.js";
import { initLibrary } from "/js/control/library.js";
import { openServiceSettings } from "/js/control/looks.js";
import { initSlides } from "/js/control/slides.js";
import { initStage } from "/js/control/stage.js";
import { initTimeline } from "/js/control/timeline.js";
import { app, call, reportWarnings, toast } from "/js/control/util.js";

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
    btn.textContent = day ? "☾" : "☀";
    btn.title = day ? "Night colours" : "Day colours";
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
$("btn-save").addEventListener("click", () => saveService(true));
$("save-status").addEventListener("click", async () => {
  // A service with no file (the example) can be kept as a scheduled service.
  if (app.service && !app.service.plan_path) {
    const today = new Date().toISOString().slice(0, 10);
    const path = await call("service_keep", { date: app.service.date ?? today });
    if (path) toast("Kept. This service now saves automatically.");
  } else if (saveError) {
    toast(saveError, "error");
  }
});
listen("episco://autosave", (s) => {
  saveError = s.ok ? null : s.error;
  renderTopbar();
});
$("btn-settings").addEventListener("click", openServiceSettings);
$("btn-layouts").addEventListener("click", openLayouts);
initOutputs();

// The announcements deck: download it (once per computer) when a service has
// one, and record its slide count so a single pass steps through every slide.
let deckUrl = null;
const syncDeckCount = (url) => {
  const n = pageCount(url);
  const s = app.service;
  if (n && s && !app.readonly && s.announcements_deck === url && s.deck_slides !== n) call("deck_set_slides", { slides: n });
};
app.on((what) => {
  if (what !== "service") return;
  const url = app.service?.announcements_deck ?? null;
  if (url && url !== deckUrl) loadDeck(url, true).then(() => syncDeckCount(url));
  deckUrl = url;
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
  $("service-title").textContent = s ? `${isTemplate ? "Template: " : ""}${s.title || "Untitled service"}` : "";
  const status = $("save-status");
  status.hidden = !s || app.readonly;
  if (s) {
    const [text, cls, title] = !s.plan_path
      ? ["Not saved · Keep…", "warn", "This service has no file. Click to keep it with your scheduled services; it then saves automatically."]
      : saveError
        ? ["⚠ Save failed", "error", saveError]
        : hasUnsaved()
          ? ["Saving…", "pending", "Changes are saved automatically a moment after you stop editing."]
          : ["✓ Saved", "ok", `Saved automatically to ${s.plan_path}`];
    status.textContent = text;
    status.className = `save-status ${cls}`;
    status.title = title;
  }
  $("btn-services").disabled = app.readonly;
  $("btn-settings").disabled = app.readonly || !s;
  $("btn-save").disabled = app.readonly || !s;

  const link = app.snap.link ?? {};
  const role = { master: "Master", slave: "Slave", standalone: "Standalone" }[link.role] ?? "—";
  let text = `${role} · ${link.online ? "online" : "offline"}`;
  if (link.peer) text += ` · ${link.peer}`;
  if (link.latency_us != null) text += ` · ${(link.latency_us / 1000).toFixed(2)} ms`;
  if (link.error) text += ` · ${link.error}`;
  $("link").textContent = text;
  $("link").title = link.endpoint ?? "";
  $("link").classList.toggle("ok", Boolean(link.online));
}

// ---------- boot ----------
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
  app.vocab = await invoke("vocabulary");
  let last = { service: undefined, state: undefined, link: undefined };
  connectStore((snap) => {
    app.snap = snap;
    app.readonly = snap.link?.role === "slave";
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
