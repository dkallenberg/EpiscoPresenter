// Output renderers shared by the full-screen output windows and the control
// window's live previews. Everything is laid out on a fixed 1920x1080 stage
// and scaled to fit, so previews are pixel-faithful miniatures.
//
// Typography (sanctuary): role labels 48pt italic; body 55pt; People text
// bold #000; everyone else #404040. 1pt = 1.333px at 96dpi on the 1080p stage.

const STAGE_W = 1920;
const STAGE_H = 1080;

const COLOR_SWATCH = {
  Green: ["#1d4d22", "#3f8f46"],
  Violet: ["#2e1342", "#6c3a8f"],
  White: ["#cfc6ad", "#f8f4e8"],
  Red: ["#5c0d0d", "#b42020"],
  Blue: ["#0f2850", "#2c5ea8"],
  Black: ["#000000", "#2a2a2a"],
};

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

function renderLine(spans) {
  const line = el("div", "line");
  for (const s of spans) {
    const node = el(s.bold ? "strong" : "span", null, s.text);
    if (s.italic) node.style.fontStyle = "italic";
    line.append(node);
  }
  return line;
}

function renderBlocks(blocks, { labels }) {
  const frag = document.createDocumentFragment();
  for (const b of blocks) {
    if (b.kind === "paragraph") {
      if (b.role === "Rubric") continue; // rubrics are not projected
      const row = el("div", "block");
      row.dataset.role = b.role ?? "";
      if (b.role === "People") row.classList.add("people");
      if (labels && b.role) row.append(el("div", "role-label", b.role));
      const text = el("div", "block-text");
      b.lines.forEach((l) => text.append(renderLine(l)));
      row.append(text);
      frag.append(row);
    } else if (b.kind === "image") {
      const img = el("img", "score");
      img.src = b.src;
      img.alt = b.alt;
      frag.append(img);
    }
  }
  return frag;
}

/** Creates a scaled 1920x1080 stage inside `host` and returns it. */
export function mountStage(host) {
  host.classList.add("stage-host");
  const stage = el("div", "stage");
  host.replaceChildren(stage);
  const fit = () => {
    const scale = Math.min(host.clientWidth / STAGE_W, host.clientHeight / STAGE_H) || 0;
    stage.style.transform = `scale(${scale})`;
  };
  new ResizeObserver(fit).observe(host);
  fit();
  return stage;
}

function backgroundStyle(cue, service) {
  const color = cue?.color ?? service?.color ?? "Green";
  const [a, b] = COLOR_SWATCH[color] ?? COLOR_SWATCH.Green;
  const gradient = `radial-gradient(ellipse at 50% 35%, ${b}, ${a})`;
  // Stained-glass images live in ui/backgrounds/; the gradient shows until
  // (or if) the image is available.
  return cue?.background ? `url("/backgrounds/${cue.background}") center/cover, ${gradient}` : gradient;
}

/** Sanctuary display (85" screen). */
export function renderSanctuary(stage, snap, active) {
  stage.replaceChildren();
  stage.className = "stage sanctuary";
  const st = snap.state ?? {};
  if (st.blackout) {
    stage.classList.add("blackout");
    return;
  }
  const cue = active?.cue;
  const lyrics = cue?.sanctuary_layout === "lyrics_on_black";
  stage.style.background = st.media_cleared || lyrics ? "#000" : backgroundStyle(cue, snap.service);
  if (lyrics) stage.classList.add("lyrics");

  if (!active) {
    // Pre-service: the announcements loop will run here (Google Slides).
    if (snap.service) stage.append(el("div", "title-card", snap.service.title));
    return;
  }
  if (cue.content.kind === "announcements") {
    stage.append(el("div", "title-card", "Announcements"));
    return;
  }
  if (st.text_cleared || !active.slide) return;

  const panel = el("div", "text-panel");
  panel.append(renderBlocks(active.slide.blocks, { labels: !lyrics }));
  stage.append(panel);
}

/** Live-stream overlay (transparent; text confined to its zone). */
export function renderStream(stage, snap, active) {
  stage.replaceChildren();
  stage.className = "stage stream";
  stage.style.background = "transparent";
  const st = snap.state ?? {};
  if (st.blackout || st.text_cleared || !active) return;
  const cue = active.cue;

  const slide = active.slide;
  const hasText = slide?.blocks.some((b) => b.kind === "paragraph" && b.role !== "Rubric");
  // Sheet-music-only slides become a text lower-third (images never go on air).
  const zone = !slide || !hasText ? "lower_third" : cue.stream_zone;

  if (zone === "lower_third") {
    const badge = el("div", "zone lower-third");
    badge.append(el("div", "badge-title", cue.caption || cue.title));
    stage.append(badge);
    return;
  }
  const box = el("div", `zone ${zone === "left_panel" ? "left-panel" : "bottom-bar"}`);
  box.append(renderBlocks(slide.blocks.filter((b) => b.kind !== "image"), { labels: false }));
  stage.append(box);
}
