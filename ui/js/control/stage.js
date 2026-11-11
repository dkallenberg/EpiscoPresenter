// Right pane: live output previews, now / next, transport, quick actions and
// the stage message box.

import { activeSlide, send } from "/js/bridge.js";
import { mountStage, renderSanctuary, renderStream } from "/js/render.js";
import { app, el, nextPosition, slideCount, slideSummary } from "/js/control/util.js";

export function initStage() {
  const sanctuary = mountStage(document.getElementById("preview-sanctuary"));
  const stream = mountStage(document.getElementById("preview-stream"));

  document.querySelectorAll("[data-cmd]").forEach((b) =>
    b.addEventListener("click", () => send({ type: b.dataset.cmd }).catch(console.error)),
  );

  const form = document.getElementById("stage-form");
  const input = document.getElementById("stage-text");
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    if (!input.value.trim()) return;
    send({ type: "stage_message", text: input.value });
    input.value = "";
  });
  document.querySelectorAll("[data-stage-preset]").forEach((b) =>
    b.addEventListener("click", () => send({ type: "stage_message", text: b.dataset.stagePreset })),
  );

  app.on((what) => {
    if (what === "link" || what === "selection") return;
    const active = activeSlide(app.snap);
    renderSanctuary(sanctuary, app.snap, active);
    renderStream(stream, app.snap, active);
    renderNowNext(active);
    renderArmed();
  });
}

function describe(service, pos) {
  if (!pos) return null;
  const cue = service.cues[pos.cue];
  const slide = cue.content.kind === "liturgy" ? cue.content.slides[pos.slide] : null;
  const n = slideCount(cue);
  return { title: cue.title, where: n > 1 ? `slide ${pos.slide + 1} of ${n}` : "", text: slideSummary(slide) };
}

function line(label, info, fallback) {
  return el(
    "div.nn-line",
    {},
    el("span.nn-label", {}, label),
    info
      ? el("span.nn-body", {}, el("strong", {}, info.title), info.where ? ` · ${info.where}` : "", info.text ? el("div.nn-text", {}, info.text) : null)
      : el("span.nn-body.muted", {}, fallback),
  );
}

function renderNowNext(active) {
  const host = document.getElementById("now-next");
  const service = app.service;
  if (!service) return host.replaceChildren();
  const now = active ? describe(service, active.pos) : null;
  const next = describe(service, nextPosition(service, app.state.active));
  host.replaceChildren(
    line("NOW", now, "Pre-service · announcements loop"),
    line("NEXT", next, "End of service"),
  );
}

function renderArmed() {
  const st = app.state;
  const set = (cmd, on) => document.querySelector(`[data-cmd="${cmd}"]`)?.classList.toggle("armed", Boolean(on));
  set("toggle_blackout", st.blackout);
  set("clear_text", st.text_cleared);
  set("clear_media", st.media_cleared);
  set("reset", !st.active);
  const msg = st.stage_message;
  const current = document.getElementById("stage-current");
  current.hidden = !msg;
  current.querySelector(".msg").textContent = msg ?? "";
  // Banner visible on every control screen (both nodes).
  const banner = document.getElementById("stage-banner");
  banner.hidden = !msg;
  banner.textContent = msg ? `Stage: ${msg}` : "";
}
