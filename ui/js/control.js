// Operator control window.

import { connectStore, activeSlide, invoke, send, inTauri } from "/js/bridge.js";
import { mountStage, renderSanctuary, renderStream } from "/js/render.js";

const $ = (id) => document.getElementById(id);

const SAMPLE = `---
title: Great Thanksgiving - Rite II
season: Pentecost
color: Green
type: eucharistic_prayer
default_background: "green_stained_glass.png"
---

[Priest] The Lord be with you.
[People] **And also with you.**
[Priest] Lift up your hearts.
[People] **We lift them to the Lord.**
[Priest] Let us give thanks to the Lord our God.
[People] **It is right to give him thanks and praise.**
`;

let roleStyles = {};
let snap = null;
let selectedCue = null; // cue shown in the slide grid (defaults to the active cue)

// ---------- previews ----------
const sanctuaryStage = mountStage($("preview-sanctuary"));
const streamStage = mountStage($("preview-stream"));

// ---------- tabs ----------
document.querySelectorAll(".tabs button").forEach((b) =>
  b.addEventListener("click", () => {
    document.querySelectorAll(".tabs button, .tab").forEach((x) => x.classList.remove("active"));
    b.classList.add("active");
    $(`tab-${b.dataset.tab}`).classList.add("active");
  }),
);

// ---------- commands ----------
const run = (command) => send(command).catch((e) => console.error(e));
document.querySelectorAll("[data-cmd]").forEach((b) =>
  b.addEventListener("click", () => run({ type: b.dataset.cmd })),
);
$("stage-form").addEventListener("submit", (e) => {
  e.preventDefault();
  run({ type: "stage_message", text: $("stage-text").value });
  $("stage-text").value = "";
});
document.addEventListener("keydown", (e) => {
  if (e.target.closest("textarea, input")) return;
  if (["ArrowRight", "PageDown", " "].includes(e.key)) run({ type: "next" });
  else if (["ArrowLeft", "PageUp"].includes(e.key)) run({ type: "prev" });
  else if (e.key === "b") run({ type: "toggle_blackout" });
  else if (e.key === "Escape") run({ type: "clear_text" });
  else return;
  e.preventDefault();
});

// ---------- editor ----------
function renderBlockView(doc) {
  const view = $("block-view");
  view.replaceChildren();
  for (const b of doc.blocks) {
    const row = document.createElement("div");
    row.className = `brow brow-${b.kind}`;
    if (b.kind === "paragraph") {
      const style = roleStyles[b.role]?.style;
      if (style) row.style.background = style.editor_bar_color;
      const role = document.createElement("span");
      role.className = "brole";
      role.textContent = b.role ?? "—";
      const text = document.createElement("span");
      text.className = "btext";
      text.innerHTML = b.lines
        .map((l) => l.map((s) => {
          const t = s.text.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);
          return s.bold ? `<b>${t}</b>` : s.italic ? `<i>${t}</i>` : t;
        }).join(""))
        .join("<br>");
      row.append(role, text);
    } else if (b.kind === "heading") {
      row.textContent = `§ ${b.text}`;
    } else if (b.kind === "image") {
      row.textContent = `🎼 ${b.alt || b.src}`;
    } else {
      row.textContent = "— slide break —";
    }
    view.append(row);
  }
}

async function parseSource() {
  try {
    const parsed = await invoke("parse_ewm", { source: $("ewm-source").value });
    renderBlockView(parsed.document);
    const w = parsed.warnings;
    $("parse-status").textContent = w.length
      ? `${w.length} warning(s): ` + w.map((x) => `line ${x.line}: ${x.message}`).join("; ")
      : `${parsed.document.blocks.length} blocks, OK`;
  } catch (e) {
    $("parse-status").textContent = String(e);
  }
}

$("btn-parse").addEventListener("click", parseSource);
$("btn-present").addEventListener("click", async () => {
  try {
    await invoke("present_ewm", { source: $("ewm-source").value });
    await parseSource();
  } catch (e) {
    $("parse-status").textContent = String(e);
  }
});

// ---------- cue list & slide grid ----------
function renderCues() {
  const list = $("cue-list");
  list.replaceChildren();
  const cues = snap.service?.cues ?? [];
  $("cue-empty").hidden = cues.length > 0;
  $("service-title").textContent = snap.service?.title ?? "";
  const active = snap.state.active;
  cues.forEach((cue, i) => {
    const li = document.createElement("li");
    li.className = "cue";
    if (active?.cue === i) li.classList.add("on-air");
    if (selectedCue === i) li.classList.add("selected");
    const swatch = document.createElement("span");
    swatch.className = `swatch color-${(cue.color ?? "none").toLowerCase()}`;
    const title = document.createElement("span");
    title.className = "cue-title";
    title.textContent = cue.title;
    const meta = document.createElement("span");
    meta.className = "cue-meta";
    const n = cue.content.kind === "liturgy" ? cue.content.slides.length : "GS";
    meta.textContent = `${cue.category.replace("_", " ")} · ${n}`;
    li.append(swatch, title, meta);
    li.addEventListener("click", () => { selectedCue = i; renderAll(); });
    li.addEventListener("dblclick", () => run({ type: "go_to", cue: i, slide: 0 }));
    list.append(li);
  });
}

function renderSlideGrid() {
  const grid = $("slide-grid");
  grid.replaceChildren();
  const cueIndex = selectedCue ?? snap.state.active?.cue ?? 0;
  const cue = snap.service?.cues?.[cueIndex];
  if (!cue) return;
  const slides = cue.content.kind === "liturgy" ? cue.content.slides : [null];
  slides.forEach((slide, i) => {
    const card = document.createElement("button");
    card.className = "slide-card";
    const pos = snap.state.active;
    if (pos?.cue === cueIndex && pos.slide === i) card.classList.add("on-air");
    const thumb = document.createElement("div");
    thumb.className = "thumb";
    const label = document.createElement("span");
    label.textContent = `${i + 1}${slide?.heading ? " · " + slide.heading : ""}`;
    card.append(thumb, label);
    grid.append(card);
    const stage = mountStage(thumb);
    renderSanctuary(stage, { ...snap, state: { revision: 0 } }, { cue, slide, pos: { cue: cueIndex, slide: i } });
    card.addEventListener("click", () => run({ type: "go_to", cue: cueIndex, slide: i }));
  });
}

function renderLink(link) {
  const el = $("link");
  const role = { master: "Master", slave: "Slave", standalone: "Standalone" }[link.role] ?? "—";
  let text = `${role} · ${link.online ? "online" : "offline"}`;
  if (link.peer) text += ` · ${link.peer}`;
  if (link.latency_us != null) text += ` · ${(link.latency_us / 1000).toFixed(2)} ms`;
  if (link.error) text += ` · ${link.error}`;
  el.textContent = text;
  el.title = link.endpoint ?? "";
  el.classList.toggle("ok", Boolean(link.online));
}

function renderAll() {
  const active = activeSlide(snap);
  renderSanctuary(sanctuaryStage, snap, active);
  renderStream(streamStage, snap, active);
  renderCues();
  renderSlideGrid();
  renderLink(snap.link ?? {});
  $("btn-blackout").classList.toggle("armed", Boolean(snap.state.blackout));
  $("stage-current").textContent = snap.state.stage_message ? `Stage: “${snap.state.stage_message}”` : "";
}

// ---------- boot ----------
$("ewm-source").value = SAMPLE;
if (!inTauri) {
  $("link").textContent = "Open this page inside the EpiscoPresenter app";
} else {
  for (const r of await invoke("role_styles")) roleStyles[r.role] = r;
  await parseSource();
  connectStore((s) => {
    const prevCue = snap?.state?.active?.cue;
    snap = s;
    // Follow the live cue in the slide grid when it changes.
    if (s.state.active?.cue !== prevCue) selectedCue = null;
    renderAll();
  });
}
