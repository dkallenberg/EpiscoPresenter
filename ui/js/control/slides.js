// Center pane, "Slides" tab: numbered thumbnails of every slide in the
// service, grouped by cue. Rebuilt only when the service changes; state
// changes just move the on-air highlight.

import { send } from "/js/bridge.js";
import { load as loadDeck, loopSlide, onDeckChange, pageCount } from "/js/deckpdf.js";
import { mountStage, renderOutput, slideFor } from "/js/render.js";
import { app, el, slideSummary } from "/js/control/util.js";

let grid;
let builtFor = null; // service object the grid was built from
let output = "sanctuary"; // which output the thumbnails show
let liveAt = null; // the live slide last scrolled to ("cue/slide")

export function initSlides() {
  grid = document.getElementById("slide-grid");
  document.querySelectorAll("[data-thumbs]").forEach((b) =>
    b.addEventListener("click", () => {
      output = b.dataset.thumbs;
      document.querySelectorAll("[data-thumbs]").forEach((x) => x.classList.toggle("active", x === b));
      build();
      highlight(false);
    }),
  );
  // The grid draws every slide: only while it can be seen (the planner
  // keeps it on a tab), catching up when the tab is shown.
  const shown = () => document.getElementById("tab-slides")?.classList.contains("active") ?? true;
  document.querySelector('.tabs button[data-tab="slides"]')?.addEventListener("click", () => {
    if (builtFor !== app.service) build();
    highlight(true);
  });
  app.on((what) => {
    if (!shown()) return;
    if (what === "service" || builtFor !== app.service) build();
    if (what !== "link") highlight(what === "selection");
  });
  // Rebuild when the deck's slides arrive; keep the looping deck's
  // highlight on the slide that is showing.
  onDeckChange((url) => {
    if (url === app.service?.announcements_deck) {
      build();
      highlight(false);
    }
  });
  setInterval(() => highlightLoop(), 1000);
}

/** Cards for a looping deck: every slide, clicking any goes to `go`. */
function deckCards(cue, ci, pages, go, jobs, label) {
  const n = Math.max(1, pages ?? 1);
  return Array.from({ length: n }, (_, k) => {
    const thumb = el("div.thumb");
    const card = el(
      "button.slide-card.deck-card-btn",
      { dataset: { loop: cue ? `cue-${ci}` : "pre", page: k + 1 }, title: label, onclick: go },
      thumb,
      el("span.slide-label", {}, el("span.slide-num", {}, String(k + 1)), pages ? `Slide ${k + 1}` : label),
    );
    jobs.push(() => {
      const stage = mountStage(thumb);
      const snap = { service: app.service, state: { revision: 0 } };
      // A still of slide k+1 (a single pass of one slide shows it still).
      const still = { ...(cue ?? {}), content: { kind: "announcements", mode: "single_pass", slides: n } };
      renderOutput(stage, snap, cue || pages ? { cue: { ...(cue ?? preCue()), ...still }, slide: null, pos: { cue: ci, slide: k } } : null, "sanctuary");
    });
    return card;
  });
}

/** A stand-in cue to render Pre-Service slides like an announcements item. */
function preCue() {
  const any = app.service?.cues.find((c) => c.content.kind === "announcements");
  return any ?? { id: "pre", title: "Before the service", category: "announcements", sanctuary: { background: "#000000", layout: "centered", tone: "light", fit: "fill" }, stream: { background: "key", layout: "centered", tone: "light", fit: "fill" } };
}

/** Pre-Service: the deck looping before the service; click to go back to it. */
function preServiceSection(jobs) {
  const url = app.service?.announcements_deck;
  const pages = url ? pageCount(url) : 0;
  const back = () => send({ type: "reset" });
  const cards = url
    ? deckCards(null, -1, pages, back, jobs, "Announcements loop")
    : [el("p.muted.small", {}, "No announcements deck: set its Google Slides link in ⚙ Settings → Service settings.")];
  return el(
    "section.slide-group.pre-group",
    {},
    el("h3.group-head", { onclick: back, title: "Go back to the announcements shown before the service" }, el("span.cue-num", {}, "–"), "Before the service", el("span.muted", {}, " · the announcements, repeating every 9 seconds")),
    el("div.slide-cards", {}, cards),
  );
}

/** Highlights the slide a looping deck is showing now. */
function highlightLoop() {
  if (!grid) return;
  const url = app.service?.announcements_deck;
  const pages = url ? pageCount(url) : 0;
  const a = app.state.active;
  const liveLoop = !a ? "pre" : app.service?.cues[a.cue]?.content.kind === "announcements" && app.service.cues[a.cue].content.mode === "loop" ? `cue-${a.cue}` : null;
  const now = pages ? loopSlide(pages) : 1;
  grid.querySelectorAll(".deck-card-btn").forEach((c) => c.classList.toggle("on-air", c.dataset.loop === liveLoop && Number(c.dataset.page) === now));
}

function build() {
  builtFor = app.service;
  const cues = app.service?.cues ?? [];
  if (!cues.length) {
    grid.replaceChildren(el("p.muted", {}, app.service ? "This service has no items yet. Use + Add in the timeline." : ""));
    return;
  }
  const jobs = [];
  const sections = cues.map((cue, ci) => {
    if (cue.content.kind === "announcements" && cue.content.mode === "loop") {
      const url = app.service?.announcements_deck;
      const cards = deckCards(cue, ci, url ? pageCount(url) : null, () => {
        app.select(cue.id);
        send({ type: "go_to", cue: ci, slide: 0 });
      }, jobs, "Google Slides loop");
      return group(cue, ci, cards);
    }
    const slides =
      cue.content.kind === "liturgy" && cue.content.slides.length
        ? cue.content.slides
        : Array.from({ length: cue.content.kind === "announcements" ? Math.max(1, cue.content.slides ?? 1) : 1 }, () => null);
    const at = cue.content.stream_at ?? [];
    // A step where this screen keeps the slide before while the other one
    // moves on (the stream runs text together or splits it differently).
    const held = (si) =>
      si > 0 && at.length > 0 && (output === "stream" ? at[si] === at[si - 1] : JSON.stringify(slides[si]) === JSON.stringify(slides[si - 1]));
    const cards = slides.map((slide, si) => {
      const thumb = el("div.thumb");
      const holds = held(si);
      const card = el(
        `button.slide-card${holds ? ".held" : ""}`,
        {
          dataset: { cue: ci, slide: si },
          title: holds
            ? `This screen keeps the slide before; the ${output === "stream" ? "sanctuary" : "live stream"} moves on.`
            : slideSummary(slideFor(cue, si, output) ?? slide),
          onclick: () => {
            app.select(cue.id);
            send({ type: "go_to", cue: ci, slide: si });
          },
        },
        thumb,
        el(
          "span.slide-label",
          {},
          el("span.slide-num", {}, String(si + 1)),
          slide?.heading ?? (cue.content.kind === "announcements" ? `Slide ${si + 1}` : ""),
        ),
      );
      // Rendered once attached, so text can be measured and fitted.
      jobs.push(() => {
        const stage = mountStage(thumb);
        const snap = { service: app.service, state: { revision: 0 } };
        renderOutput(stage, snap, { cue, slide, pos: { cue: ci, slide: si } }, output);
      });
      return card;
    });
    return group(cue, ci, cards);
  });
  const url = app.service?.announcements_deck;
  if (url) loadDeck(url);
  grid.replaceChildren(preServiceSection(jobs), ...sections);
  jobs.forEach((render) => render());
}

function group(cue, ci, cards) {
  return el(
      "section.slide-group",
      { dataset: { id: cue.id } },
      el(
        "h3.group-head",
        { onclick: () => app.select(cue.id) },
        el("span.cue-num", {}, String(ci + 1)),
        cue.title,
        el("span.muted", {}, ` · ${cue.category.replace(/_/g, " ")}`),
      ),
      el("div.slide-cards", {}, cards),
    );
}

function highlight(scrollToSelection) {
  const a = app.state.active;
  grid.querySelectorAll(".slide-card.on-air").forEach((c) => c.classList.remove("on-air"));
  grid.querySelectorAll(".slide-group.selected").forEach((g) => g.classList.remove("selected"));
  const live = a && grid.querySelector(`.slide-card[data-cue="${a.cue}"][data-slide="${a.slide}"]`);
  live?.classList.add("on-air");
  highlightLoop();
  const group = app.selectedId && grid.querySelector(`.slide-group[data-id="${CSS.escape(app.selectedId)}"]`);
  group?.classList.add("selected");
  // The live slide moves to the top when it changes (Next, Back, a click),
  // so the slides coming up are in view; browsing other items scrolls to them.
  const at = a ? `${a.cue}/${a.slide}` : null;
  const moved = at !== liveAt;
  liveAt = at;
  if (scrollToSelection && !moved) group?.scrollIntoView({ block: "start", behavior: "smooth" });
  else if (moved) live?.scrollIntoView({ block: "start", behavior: "smooth" });
  else live?.scrollIntoView({ block: "nearest" });
}
