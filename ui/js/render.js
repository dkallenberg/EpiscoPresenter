// Output renderers shared by the full-screen output windows and the control
// window's previews. Everything is laid out on a fixed 1920x1080 stage and
// scaled to fit, so previews are faithful miniatures.
//
// A cue carries a resolved *look* per output ({background, layout, tone,
// fit}); the layout fixes where text goes (boxes measured from the parish's
// slide designs) and the tone picks dark-on-light or light-on-dark text.
// Sanctuary sizes follow the spec: role labels 48pt italic, text 55pt.

import { assetUrl, backgroundUrl } from "/js/bridge.js";
import { builtinUrl } from "/js/backgrounds.js";
import { deckEmbedUrl } from "/js/deck.js";
import { load as loadDeck, loopSlide, slideImage } from "/js/deckpdf.js";
import { isAntiphonSlide, resolveTemplate, slideTypeFor } from "/js/templates.js";

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

/** The parish's lower-third box (green with the gold border, cut out of
 *  its artwork onto a transparent layer) over the flat key colour, in every
 *  season and with any key colour. */
const LOWER_THIRD_FRAME = "/backgrounds/green/lower-third-frame.webp";

function paintLowerThirdFrame(stage) {
  const layer = el("div", "bg lt-frame");
  layer.style.background = `url("${LOWER_THIRD_FRAME}") center / 100% 100% no-repeat`;
  stage.append(layer);
}

export const isVideo = (value) => /\.(mp4|m4v|mov|webm|ogv)$/i.test(value ?? "");

/** A video background. In the output windows it plays (with sound on the
 *  sanctuary screen only); in previews it shows a still frame. The playing
 *  element is reused across re-renders so the video never restarts. */
function videoLayer(stage, service, look, opts) {
  const src = backgroundUrl(service, look.background);
  const keep = opts.keepVideo;
  const fit = { fill: "cover", fit: "contain", stretch: "fill" }[look.fit] ?? "cover";
  if (opts.live && keep && keep.dataset.src === src) {
    keep.style.objectFit = fit;
    stage.append(keep);
    if (keep.paused) keep.play().catch(() => {});
    return;
  }
  const v = document.createElement("video");
  v.className = "bg bg-video";
  v.dataset.src = src;
  v.style.objectFit = fit;
  v.muted = !(opts.live && opts.sound);
  v.playsInline = true;
  if (opts.live) {
    v.autoplay = true;
    v.loop = true;
    v.src = src;
  } else {
    v.preload = "metadata";
    v.src = `${src}#t=0.5`;
  }
  stage.append(v);
  if (opts.live) v.play().catch(() => { v.muted = true; v.play().catch(() => {}); });
}

function paintBackground(stage, service, cue, look, opts = {}) {
  if (isVideo(look.background)) {
    stage.style.background = "#000";
    return videoLayer(stage, service, look, opts);
  }
  const layer = el("div", "bg");
  layer.style.background = backgroundCss(service, cue, look.background, look.fit);
  stage.append(layer);
  if (look.background === "key" && look.layout === "lower_third") {
    paintLowerThirdFrame(stage);
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
    if (/^antiphon$/i.test(b.role ?? "")) {
      // An antiphon's words: bold, under its music line, no label.
      const row = el("div", "row span antiphon-words");
      row.append(paragraphText(b, { ...opts, bold: true }));
      rows.append(row);
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
  const license = extra(cue, "license");
  if (license) lines.push(license);
  if (!lines.length) return null;
  const c = el("div", "credits");
  lines.forEach((l) => c.append(el("div", null, l)));
  return c;
}

// ---------- templated slides ----------

const JUSTIFY = { top: "flex-start", center: "center", bottom: "flex-end" };

/** Positions `node` at an element's rectangle and applies its type style. */
function styleElement(node, spec) {
  Object.assign(node.style, {
    left: `${spec.x}px`,
    top: `${spec.y}px`,
    width: `${spec.w}px`,
    height: `${spec.h}px`,
  });
  if (spec.font) node.style.fontFamily = spec.font;
  if (spec.color) node.style.color = spec.color;
  if (spec.bold != null) node.style.fontWeight = spec.bold ? "700" : "400";
  if (spec.italic != null) node.style.fontStyle = spec.italic ? "italic" : "normal";
  if (spec.align) {
    node.style.textAlign = spec.align;
    node.style.alignItems = { left: "flex-start", center: "center", right: "flex-end" }[spec.align];
  }
  node.style.justifyContent = JUSTIFY[spec.valign] ?? "center";
  if (spec.size) node.style.setProperty("--body", `${spec.size}px`);
}

/** A single piece of text (title, reference, tag …) shrunk to fit. */
function textPiece(stage, name, spec, text) {
  const node = el("div", `piece piece-${name}`);
  styleElement(node, spec);
  node.append(el("div", "piece-text", text));
  stage.append(node);
  autoFit(node, 0.4);
  return node;
}

function imagePiece(stage, name, spec, cue, block) {
  const node = el("div", `piece piece-image piece-${name}`);
  styleElement(node, spec);
  const img = el("img");
  img.src = assetUrl(cue, block.src);
  img.alt = block.alt ?? "";
  img.onerror = () => img.replaceWith(el("div", "score-missing", `♪ ${block.alt || block.src}`));
  node.append(img);
  stage.append(node);
  return node;
}

/** The text box grows into title space above it that this slide leaves empty. */
function textRect(T, shown) {
  const t = { ...T.text };
  const bottom = t.y + t.h;
  let changed = true;
  while (changed) {
    changed = false;
    for (const [name, spec] of Object.entries(T)) {
      if (name === "text" || shown.has(name) || spec.kind === "font" || spec.x == null) continue;
      const touches = spec.y < t.y && spec.y + spec.h >= t.y - 2 && spec.x < t.x + t.w && spec.x + spec.w > t.x;
      if (touches) {
        t.y = spec.y;
        changed = true;
      }
    }
  }
  t.h = bottom - t.y;
  return t;
}

function textBox(stage, T, shown, content, cls = "") {
  const box = el("div", `box tbox ${cls}`);
  const rect = textRect(T, shown);
  styleElement(box, rect);
  box.style.setProperty("--box-h", `${rect.h}px`);
  if (T.labels?.size) box.style.setProperty("--label", `${T.labels.size}px`);
  box.append(content);
  stage.append(box);
  const paint = (sel, spec) => {
    if (!spec) return;
    box.querySelectorAll(sel).forEach((n) => {
      if (spec.color) n.style.color = spec.color;
      if (spec.font) n.style.fontFamily = spec.font;
      if (spec.italic != null) n.style.fontStyle = spec.italic ? "italic" : "normal";
      if (spec.bold != null) n.style.fontWeight = spec.bold ? "700" : "400";
    });
  };
  paint(".label", T.labels);
  if (T.people?.color) box.querySelectorAll(".people .text").forEach((n) => (n.style.color = T.people.color));
  autoFit(box, 0.5);
  return box;
}

/** What each element shows on this slide (absent: not shown). */
function pieces(key, cue, slide) {
  const meta = cue.content.document?.meta ?? {};
  const citation = meta.citation;
  const p = {};
  const heading = slide?.heading;
  const titleSlide = !slide || cue.content.kind === "announcements" || !slide.blocks.length;

  if (key === "sanctuary.title") {
    let main = cue.content.kind === "announcements" ? cue.title : citation ?? (heading ? "" : cue.title);
    let small = heading && heading !== main ? heading : null;
    if (!main) [main, small] = [small ?? cue.title, null];
    p.title = main;
    if (small) p.subtitle = small;
    return p;
  }

  if (key === "sanctuary.prayer" || key === "sanctuary.psalm") {
    if (key === "sanctuary.psalm" && isAntiphonSlide(slide)) {
      const img = slide.blocks.find((b) => b.kind === "image" && b.src);
      if (img) p.music = img;
      p.antiphon = slide.blocks.filter((b) => b.kind === "paragraph" && b.role !== "Rubric");
      return p;
    }
    if (slide.section_start && heading) {
      p.title = citation ?? cue.title;
      if (heading !== p.title && !(key === "sanctuary.psalm" && heading === citation)) p.subtitle = heading;
    }
    p.text = slide.blocks;
    return p;
  }

  if (key === "stream.lower_third") {
    if (titleSlide) {
      if (cue.content.kind === "announcements" || !slide) p.big = cue.title;
      else {
        p.title = heading ?? cue.title;
        p.big = citation ?? cue.title;
      }
      return p;
    }
    if (slide.section_start && heading) {
      p.title = heading;
      const ref = reference(cue);
      if (ref && ref !== heading) p.reference = ref;
    }
    p.text = slide.blocks.filter((b) => b.kind !== "image");
    return p;
  }

  if (key === "stream.bottom_bar") {
    p.tag = cue.category === "psalm" ? citation ?? cue.title : cue.title;
    const setting = extra(cue, "setting_image");
    if (setting) p.music = { kind: "image", src: setting, alt: "Musical setting" };
    if (slide?.section_start && heading && !isAntiphonSlide(slide)) {
      p.title = cue.title;
      if (heading !== cue.title && heading !== citation) p.subtitle = heading;
    }
    if (titleSlide) p.big = citation ?? cue.title;
    else p.text = slide.blocks;
    return p;
  }

  // stream.left_panel, stream.hymn
  p.title = cue.title;
  const ref = reference(cue);
  if (ref && ref !== cue.title) p.reference = ref;
  const sub = meta.title && meta.title !== cue.title ? meta.title : heading && heading !== cue.title ? heading : "";
  if (sub && !(heading && /^verse\b/i.test(heading) && sub === heading)) p.subtitle = sub;
  if (titleSlide) p.big = citation ?? "";
  else p.text = slide.blocks;
  if (key === "stream.hymn") {
    const c = credits(cue);
    if (c) p.credits = c;
  }
  return p;
}

/** Renders a slide with a slide type's layout. */
export function renderTemplated(stage, key, cue, slide, templates) {
  const T = resolveTemplate(templates, key);
  const p = pieces(key, cue, slide);
  const shown = new Set();
  for (const [name, value] of Object.entries(p)) {
    if (value == null || value === "" || name === "big" || name === "text") continue;
    if (!T[name] || T[name].hidden) continue;
    shown.add(name);
  }
  for (const name of shown) {
    const spec = T[name];
    const value = p[name];
    if (spec.kind === "image") imagePiece(stage, name, spec, cue, value);
    else if (Array.isArray(value)) {
      const box = el("div", `box tbox piece-${name}`);
      styleElement(box, spec);
      box.append(renderBlocks(value, cue, { labels: false, numbers: false, rubrics: false, bold: spec.bold !== false }));
      stage.append(box);
      autoFit(box, 0.5);
    } else if (value instanceof Node) {
      const node = el("div", `piece piece-${name}`);
      styleElement(node, spec);
      node.append(value);
      stage.append(node);
      autoFit(node, 0.4);
    } else textPiece(stage, name, spec, value);
  }
  if (!T.text || T.text.hidden) return;
  if (p.big) {
    textBox(stage, T, shown, el("div", key === "stream.lower_third" ? "lt-main" : "title-big", p.big), "title-card");
  } else if (p.text) {
    const hymn = key === "stream.hymn";
    const stream = key.startsWith("stream.");
    const blocks = p.text;
    const box = textBox(stage, T, shown, renderBlocks(blocks, cue, { labels: true, numbers: true, rubrics: stream, reflow: hymn, bold: hymn }));
    if (key === "stream.lower_third") box.classList.toggle("lt-center", !blocks.some((b) => b.kind === "paragraph" && b.role));
  }
}

// ---------- the Google Slides deck ----------

/**
 * The announcements deck: images of its slides (from the downloaded PDF,
 * ui/js/deckpdf.js). `slide` (1-based) shows that slide; without it the deck
 * loops, a slide every 9 seconds, in step on every screen. If the slides are
 * not available (a "Publish to web" link, or the download failed), the
 * output windows fall back to Google's embedded player and previews show a
 * card.
 */
function paintDeck(stage, service, opts, { slide = null, of = null, loop = false } = {}) {
  const url = service?.announcements_deck;
  const src = deckEmbedUrl(url, { slide, loop });
  if (!src) {
    const box = el("div", "box title-card");
    box.append(el("div", "title-small", "Announcements"), el("div", "deck-missing", "No Google Slides deck is set (⚙ Service)."));
    stage.append(box);
    return;
  }
  const width = opts.live ? 1920 : 640;
  const img = el("img", "deck-img");
  img.alt = "";
  stage.append(img);
  const fallback = () => {
    if (!img.isConnected) return;
    if (opts.live) {
      const keep = opts.keepDeck;
      if (keep && keep.dataset.src === src) return img.replaceWith(keep);
      const frame = document.createElement("iframe");
      frame.className = "deck-frame";
      frame.dataset.src = src;
      frame.src = src;
      frame.allow = "autoplay; fullscreen";
      frame.setAttribute("frameborder", "0");
      img.replaceWith(frame);
      return;
    }
    const card = el("div", "deck-card");
    card.append(
      el("div", "deck-icon", "▤"),
      el("div", "deck-title", "Google Slides"),
      el("div", "deck-sub", slide != null ? `Slide ${slide}${of ? ` of ${of}` : ""}` : loop ? "Loop · every 9 s" : "Plays once · every 9 s"),
    );
    img.replaceWith(card);
  };
  const show = (n) =>
    slideImage(url, n, width).then((u) => {
      if (u) img.src = u;
      else fallback();
    });
  loadDeck(url).then((pages) => {
    if (!pages) return fallback();
    if (slide != null) return show(Math.min(slide, pages));
    // Loop: change slide every 9 s while this image is on screen.
    let current = 0;
    const tick = () => {
      if (!img.isConnected && current) return clearInterval(timer);
      const n = loopSlide(pages);
      if (n !== current) {
        current = n;
        show(n);
      }
    };
    const timer = setInterval(tick, 500);
    tick();
  });
}

// ---------- outputs ----------

/**
 * Renders `output` ("sanctuary" | "stream") for the active slide.
 * `active` is { cue, slide, pos } or null (pre-service).
 * `opts.live` is set by the output windows: videos play (with sound on the
 * sanctuary screen); previews show a still frame.
 */
export function renderOutput(stage, snap, active, output, opts = {}) {
  const service = snap.service;
  const st = snap.state ?? {};
  const keepVideo = stage.querySelector("video.bg-video");
  const keepDeck = stage.querySelector("iframe.deck-frame");
  stage.replaceChildren();
  stage.className = `stage out-${output}`;
  stage.style.background = output === "stream" ? keyColor(service) : "#000";

  if (st.blackout) return; // sanctuary black; stream fully keyed (camera only)

  const cue = active?.cue;
  if (!cue) return renderPreService(stage, service, output, { ...opts, keepDeck }, st);
  const look = cue[output];
  if (!look) return;
  stage.classList.add(`layout-${look.layout}`, `tone-${look.tone}`, `cat-${cue.category}`);
  // Planned blank (or an item for the other screen only): black on the
  // sanctuary, just the key colour (the camera) on the stream.
  if (look.blank) return;

  if (cue.content.kind === "announcements" && !st.media_cleared) {
    const single = cue.content.mode !== "loop";
    const steps = cue.content.slides ?? 1;
    return paintDeck(stage, service, { ...opts, keepDeck }, single && steps > 1 ? { slide: (active.pos?.slide ?? 0) + 1, of: steps } : { loop: !single });
  }

  if (!st.media_cleared) paintBackground(stage, service, cue, look, { ...opts, keepVideo, sound: output === "sanctuary" });
  else if (look.layout === "lower_third" || output === "stream") stage.style.background = keyColor(service);
  if (st.text_cleared) return;

  const slide = active.slide;
  // A media item (an image or video from the library) is just its background.
  if (!slide && cue.content.kind === "liturgy" && extra(cue, "media")) return;

  const key = slideTypeFor(cue, slide, output, look);
  if (key) {
    if (!slide && key !== "stream.lower_third") return;
    return renderTemplated(stage, key, cue, slide, service?.templates);
  }
  if (look.layout === "info_bar") return infoBar(stage, cue, slide);

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
    // Hymn lyrics.
    box.append(renderBlocks(slide.blocks, cue, { labels: false, numbers: false, rubrics: false, bold: true }));
    const cap = el("div", "lyrics-caption", cue.caption);
    stage.append(cap);
  } else {
    box.append(streamHeader(cue, slide));
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

function renderPreService(stage, service, output, opts = {}, st = {}) {
  if (!service) return;
  // The announcements deck loops on the sanctuary screen before the service.
  if (output === "sanctuary" && service.announcements_deck && !st.media_cleared) {
    return paintDeck(stage, service, opts, { loop: true });
  }
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
export const renderSanctuary = (stage, snap, active, opts) => renderOutput(stage, snap, active, "sanctuary", opts);
export const renderStream = (stage, snap, active, opts) => renderOutput(stage, snap, active, "stream", opts);
