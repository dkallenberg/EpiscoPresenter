// Center pane, "Slides" tab: numbered thumbnails of every slide in the
// service, grouped by cue. Rebuilt only when the service changes; state
// changes just move the on-air highlight.

import { send } from "/js/bridge.js";
import { mountStage, renderOutput } from "/js/render.js";
import { app, el, slideSummary } from "/js/control/util.js";

let grid;
let builtFor = null; // service object the grid was built from
let output = "sanctuary"; // which output the thumbnails show

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
  app.on((what) => {
    if (what === "service" || builtFor !== app.service) build();
    if (what !== "link") highlight(what === "selection");
  });
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
    const slides = cue.content.kind === "liturgy" && cue.content.slides.length ? cue.content.slides : [null];
    const cards = slides.map((slide, si) => {
      const thumb = el("div.thumb");
      const card = el(
        "button.slide-card",
        {
          dataset: { cue: ci, slide: si },
          title: slideSummary(slide),
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
          slide?.heading ?? (cue.content.kind === "announcements" ? "Google Slides deck" : ""),
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
  });
  grid.replaceChildren(...sections);
  jobs.forEach((render) => render());
}

function highlight(scrollToSelection) {
  const a = app.state.active;
  grid.querySelectorAll(".slide-card.on-air").forEach((c) => c.classList.remove("on-air"));
  grid.querySelectorAll(".slide-group.selected").forEach((g) => g.classList.remove("selected"));
  const live = a && grid.querySelector(`.slide-card[data-cue="${a.cue}"][data-slide="${a.slide}"]`);
  live?.classList.add("on-air");
  const group = app.selectedId && grid.querySelector(`.slide-group[data-id="${CSS.escape(app.selectedId)}"]`);
  group?.classList.add("selected");
  if (scrollToSelection) group?.scrollIntoView({ block: "start", behavior: "smooth" });
  else live?.scrollIntoView({ block: "nearest" });
}
