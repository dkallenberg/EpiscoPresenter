// Operator control window: boot, top bar, tabs and keyboard shortcuts.

import { connectStore, inTauri, invoke, send } from "/js/bridge.js";
import { initEditor } from "/js/control/editor.js";
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

// ---------- file actions ----------
async function confirmDiscard() {
  if (!app.service || !hasUnsaved()) return true;
  return call("confirm", {
    title: "Unsaved changes",
    message: "The current service has unsaved changes. Discard them?",
    okLabel: "Discard",
  });
}

const hasUnsaved = () => app.service && (app.service.modified || app.service.cues.some((c) => c.modified));

$("btn-new").addEventListener("click", async () => {
  if (await confirmDiscard()) await call("new_service");
});
$("btn-open").addEventListener("click", async () => {
  if (!(await confirmDiscard())) return;
  const r = await call("open_service", { path: null });
  if (r?.opened) reportWarnings(r.warnings);
});
$("btn-example").addEventListener("click", async () => {
  if (!(await confirmDiscard())) return;
  const w = await call("load_example_service");
  if (w) reportWarnings(w);
});
$("btn-save").addEventListener("click", (e) => saveService(e.shiftKey));
$("btn-settings").addEventListener("click", openServiceSettings);
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
  if (path) toast(`Service saved to ${path}`);
}

// ---------- presentation shortcuts ----------
document.addEventListener("keydown", (e) => {
  if (e.target.closest("textarea, input, select, [contenteditable='true']") || e.metaKey || e.ctrlKey) return;
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
  $("service-title").textContent = s ? s.title || "Untitled service" : "";
  $("btn-save").classList.toggle("unsaved", Boolean(hasUnsaved()));
  $("btn-save").title = s?.plan_path ? `Save ${s.plan_path} (⌥⌘S; shift-click for Save As)` : "Save service as… (⌥⌘S)";
  for (const id of ["btn-new", "btn-open", "btn-example", "btn-save"]) $(id).disabled = app.readonly;
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
initStage();
app.on(renderTopbar);

if (!inTauri) {
  $("link").textContent = "Open this page inside the EpiscoPresenter app";
} else {
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
