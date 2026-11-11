// Output renderers shared by the full-screen output windows and the control
// window's previews. Everything is laid out on a fixed 1920x1080 stage and
// scaled to fit, so previews are faithful miniatures.
//
// A cue carries a resolved *look* per output ({background, layout, tone,
// fit}); the layout fixes where text goes (boxes measured from the parish's
// slide designs) and the tone picks dark-on-light or light-on-dark text.
// Sanctuary sizes follow the spec: role labels 48pt italic, text 55pt.

import { assetUrl, backgroundUrl } from "/js/bridge.js";
import { artworkUrl, builtinUrl, lowerThirdFrameUrl } from "/js/backgrounds.js";

const STAGE_W = 1920;
const STAGE_H = 1080;
const DEFAULT_KEY = "#0000ff";

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
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

// ---------- backgrounds ----------

export const keyColor = (service) => service?.stream_key || DEFAULT_KEY;

/** CSS `background` for a look's background value. */
export function backgroundCss(service, cue, value, fit = "fill") {
  const color = cue?.color ?? service?.color ?? "Green";
  if (!value) return "#000";
  if (value === "key") return keyColor(service);
  if (/^(#|rgb|hsl)/i.test(value)) return value;
  const url = builtinUrl(value, color) ?? backgroundUrl(service, value);
  const size = { fill: "cover", fit: "contain", stretch: "100% 100%" }[fit] ?? "cover";
  return `#000 url("${url}") center / ${size} no-repeat`;
}

/** Key colour plus the framed bar, for lower thirds on the `key` background.
 *  With the default blue key, the parish's own lower-third artwork is used
 *  where there is one; with any other key colour the frame is drawn. */
function paintLowerThirdFrame(stage, service, color) {
  const art = artworkUrl("lower-third", color);
  const layer = el("div", "bg");
  if (art && keyColor(service).toLowerCase() === DEFAULT_KEY) {
    layer.style.background = `url("${art}") center / 100% 100% no-repeat`;
  } else {
    layer.style.background = `url("${lowerThirdFrameUrl(color)}") center / 100% 100% no-repeat`;
  }
  stage.append(layer);
}

function paintBackground(stage, service, cue, look) {
  const layer = el("div", "bg");
  layer.style.background = backgroundCss(service, cue, look.background, look.fit);
  stage.append(layer);
  if (look.background === "key" && look.layout === "lower_third") {
    paintLowerThirdFrame(stage, service, cue?.color ?? service?.color ?? "Green");
  }
}

// ---------- text ----------

const isVerseNumber = (role) => /^\d+[a-z]?$/i.test(role ?? "");

function renderLine(spans, forceBold) {
  const line = el("span", "line");
  for (const s of spans) {
    const node = el(s.bold || forceBold ? "strong" : "span", null, s.text);
    if (s.italic) node.style.fontStyle = "italic";
    line.append(node);
  }
  return line;
}

/** Text of a paragraph block, one element per line (or reflowed). */
function paragraphText(b, { reflow, bold }) {
  const text = el("div", "text");
  b.lines.forEach((l, i) => {
    if (reflow && i > 0) text.append(" ");
    text.append(renderLine(l, bold));
    if (!reflow) text.append(el("br"));
  });
  if (!reflow) text.lastChild?.remove();
  return text;
}

/**
 * Speaker rows: a label column (role in italic, or a verse number) and the
 * text hanging beside it. Untagged text spans both columns.
 */
function renderBlocks(blocks, cue, opts) {
  const rows = el("div", "rows");
  for (const b of blocks) {
    if (b.kind === "image") {
      const img = el("img", "score");
      img.src = assetUrl(cue, b.src);
      img.alt = b.alt;
      img.onerror = () => img.replaceWith(el("div", "score-missing", `♪ ${b.alt || b.src}`));
      rows.append(img);
      continue;
    }
    if (b.kind !== "paragraph") continue;
    if (b.role === "Rubric") {
      if (opts.rubrics) rows.append(el("div", "rubric", b.lines.map((l) => l.map((s) => s.text).join("")).join(" ")));
      continue;
    }
    const row = el("div", "row");
    const people = b.role === "People";
    if (people) row.classList.add("people");
    if (b.role && opts.labels && !(isVerseNumber(b.role) && !opts.numbers)) {
      row.append(el("div", isVerseNumber(b.role) ? "label number" : "label", b.role));
    } else {
      row.classList.add("span");
    }
    row.append(paragraphText(b, opts));
    rows.append(row);
  }
  return rows;
}

/** Shrinks text (via --fit) until `box` no longer overflows. */
function autoFit(box, min = 0.5) {
  let fit = 1;
  box.style.setProperty("--fit", fit);
  while ((box.scrollHeight > box.clientHeight + 1 || box.scrollWidth > box.clientWidth + 1) && fit > min) {
    fit = Math.round((fit - 0.04) * 100) / 100;
    box.style.setProperty("--fit", fit);
  }
}

const extra = (cue, key) => cue.content.document?.meta.extra?.find(([k]) => k === key)?.[1];

/** Reference shown at the right of a header: "Hymn 436", "BCP 355". */
function reference(cue) {
  const meta = cue.content.document?.meta ?? {};
  if (meta.hymn_number) return `Hymn ${meta.hymn_number}`;
  return meta.citation ?? "";
}

// ---------- slide types ----------

function titleCard(box, heading, cue) {
  const meta = cue.content.document?.meta ?? {};
  const main = meta.citation ?? (heading ? "" : cue.title);
  box.classList.add("title-card");
  if (heading) box.append(el("div", "title-small", heading));
  if (main) box.append(el("div", "title-big", main));
}

function lowerThird(stage, cue, active) {
  const box = el("div", "box lt");
  stage.append(box);
  const slide = active?.slide;
  if (!slide || cue.content.kind === "announcements") {
    box.classList.add("lt-center");
    box.append(el("div", "lt-label-big", cue.title));
  } else if (!slide.blocks.length) {
    // Title slide: "The First Reading" / "Joshua 24:1-3a, 14-25"
    box.append(el("div", "lt-label", slide.heading ?? cue.title));
    box.append(el("div", "lt-main", cue.content.document?.meta.citation ?? cue.title));
  } else {
    if (slide.section_start && slide.heading) {
      const head = el("div", "lt-head");
      head.append(el("span", "lt-title", slide.heading));
      const ref = reference(cue);
      if (ref) head.append(el("span", "ref", ref));
      box.append(head);
    }
    box.append(renderBlocks(slide.blocks.filter((b) => b.kind !== "image"), cue, { labels: true, numbers: true, rubrics: true }));
    box.classList.toggle("lt-center", !slide.blocks.some((b) => b.kind === "paragraph" && b.role));
  }
  autoFit(box, 0.6);
}

function streamHeader(cue, slide) {
  const meta = cue.content.document?.meta ?? {};
  const head = el("div", "header");
  const top = el("div", "header-top");
  top.append(el("span", "header-title", cue.title));
  const ref = reference(cue);
  if (ref && ref !== cue.title) top.append(el("span", "ref", ref));
  head.append(top);
  // Hymn title under "Entrance Hymn"; otherwise a section heading.
  const sub = meta.title && meta.title !== cue.title ? meta.title : slide?.heading && slide.heading !== cue.title ? slide.heading : "";
  if (sub && !(slide?.heading && /^verse\b/i.test(slide.heading) && sub === slide.heading)) head.append(el("div", "header-sub", sub));
  return head;
}

function credits(cue) {
  const lines = ["words", "music", "setting"]
    .map((k) => [k, extra(cue, k)])
    .filter(([, v]) => v)
    .map(([k, v]) => `${k[0].toUpperCase()}${k.slice(1)}: ${v}`);
  if (!lines.length) return null;
  const c = el("div", "credits");
  lines.forEach((l) => c.append(el("div", null, l)));
  return c;
}

// ---------- outputs ----------

/**
 * Renders `output` ("sanctuary" | "stream") for the active slide.
 * `active` is { cue, slide, pos } or null (pre-service).
 */
export function renderOutput(stage, snap, active, output) {
  const service = snap.service;
  const st = snap.state ?? {};
  stage.replaceChildren();
  stage.className = `stage out-${output}`;
  stage.style.background = output === "stream" ? keyColor(service) : "#000";

  if (st.blackout) return; // sanctuary black; stream fully keyed (camera only)

  const cue = active?.cue;
  if (!cue) return renderPreService(stage, service, output);
  const look = cue[output];
  if (!look) return;
  stage.classList.add(`layout-${look.layout}`, `tone-${look.tone}`, `cat-${cue.category}`);

  if (!st.media_cleared) paintBackground(stage, service, cue, look);
  else if (look.layout === "lower_third" || output === "stream") stage.style.background = keyColor(service);
  if (st.text_cleared) return;

  if (look.layout === "lower_third") return lowerThird(stage, cue, active);
  if (look.layout === "info_bar") return infoBar(stage, cue, active.slide);

  const slide = active.slide;
  const box = el("div", "box");
  stage.append(box);

  if (cue.content.kind === "announcements") {
    titleCard(box, null, cue);
    return;
  }
  if (!slide) return;
  if (!slide.blocks.length) {
    titleCard(box, slide.heading, cue);
    autoFit(box);
    return;
  }

  const hymn = cue.category === "hymn";
  if (output === "sanctuary") {
    if (look.layout === "lyrics") {
      box.append(renderBlocks(slide.blocks, cue, { labels: false, numbers: false, rubrics: false, bold: true }));
      const cap = el("div", "lyrics-caption", cue.caption);
      stage.append(cap);
    } else {
      if (slide.section_start && slide.heading) {
        const head = el("div", "s-header");
        const left = cue.content.document?.meta.citation ?? cue.title;
        head.append(el("span", null, left));
        if (slide.heading !== left) head.append(el("span", null, slide.heading));
        box.append(head);
      }
      box.append(renderBlocks(slide.blocks, cue, { labels: true, numbers: true, rubrics: false }));
    }
  } else {
    const bar = look.layout === "bottom_bar";
    if (!bar || slide.section_start) box.append(streamHeader(cue, slide));
    if (bar && cue.category === "psalm" && cue.content.document?.meta.citation) {
      stage.append(el("div", "tag", cue.content.document.meta.citation));
    }
    box.append(renderBlocks(slide.blocks, cue, { labels: true, numbers: true, rubrics: true, reflow: hymn, bold: hymn }));
    if (hymn) {
      const c = credits(cue);
      if (c) stage.append(c);
    }
  }
  autoFit(box);
}

/** Text in the bar under the camera window (welcome / postlude design). */
function infoBar(stage, cue, slide) {
  const box = el("div", "box ib");
  stage.append(box);
  if (slide?.heading) box.append(el("div", "ib-small", slide.heading));
  const blocks = slide?.blocks ?? [];
  if (blocks.length) box.append(renderBlocks(blocks.filter((b) => b.kind !== "image"), cue, { labels: true, numbers: true, rubrics: true }));
  else box.append(el("div", "ib-big", cue.title));
  autoFit(box, 0.6);
}

function formatDate(iso) {
  const d = iso && new Date(`${iso}T12:00:00`);
  return d && !isNaN(d) ? d.toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" }) : iso ?? "";
}

function renderPreService(stage, service, output) {
  if (!service) return;
  if (output === "stream") {
    // Welcome: camera window with the service details in the bar beneath.
    stage.classList.add("layout-info_bar", "tone-light");
    const layer = el("div", "bg");
    layer.style.background = backgroundCss(service, null, "builtin:window");
    stage.append(layer);
    const box = el("div", "box ib");
    const line = [service.title, formatDate(service.date)].filter(Boolean).join(", ");
    box.append(el("div", "ib-small", line), el("div", "ib-big", "Welcome"));
    stage.append(box);
    autoFit(box, 0.6);
    return;
  }
  stage.classList.add("layout-centered", "tone-light");
  const layer = el("div", "bg");
  layer.style.background = backgroundCss(service, null, "builtin:glass-dark");
  stage.append(layer);
  const box = el("div", "box title-card");
  box.append(el("div", "title-big", service.title));
  stage.append(box);
  autoFit(box);
}

// Back-compat names used by the output pages and control previews.
export const renderSanctuary = (stage, snap, active) => renderOutput(stage, snap, active, "sanctuary");
export const renderStream = (stage, snap, active) => renderOutput(stage, snap, active, "stream");
