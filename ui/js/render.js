// Output renderers shared by the full-screen output windows and the control
// window's previews. Everything is laid out on a fixed 1920x1080 stage and
// scaled to fit, so previews are faithful miniatures.
//
// A cue carries a resolved *look* per output ({background, layout, tone,
// fit}); the layout fixes where text goes (boxes measured from the parish's
// slide designs) and the tone picks dark-on-light or light-on-dark text.
// Sanctuary sizes follow the spec: role labels 48pt italic, text 55pt.

import { assetUrl, backgroundUrl } from "/js/bridge.js";
import { builtinUrl, lowerThirdFrame, lowerThirdText } from "/js/backgrounds.js";
import { deckEmbedUrl } from "/js/deck.js";
import { load as loadDeck, loopSlide, slideImage } from "/js/deckpdf.js";
import { isAntiphonSlide, resolveTemplate, slideTypeFor, typeOptions } from "/js/templates.js";

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
  const url = builtinUrl(value, color, seasonOf(service, cue)) ?? backgroundUrl(service, value);
  const size = { fill: "cover", fit: "contain", stretch: "100% 100%" }[fit] ?? "cover";
  return `#000 url("${url}") center / ${size} no-repeat`;
}

/** White text on a season's light lower-third box (White) is drawn dark.
 *  Set while a slide is drawn (drawing is synchronous). */
let lightBox = false;
const ink = (c) => (lightBox && /^(#fff(fff)?|white)$/i.test(c) ? "#1f1f1f" : c);

/** The season, which picks the artwork for violet (Advent's or Lent's). */
const seasonOf = (service, cue) => cue?.content?.document?.meta?.season ?? service?.season ?? null;

/** The parish's lower-third box (cut out of the season's artwork onto a
 *  transparent layer) over the flat key colour, with any key colour. */
function paintLowerThirdFrame(stage, service, cue) {
  const color = cue?.color ?? service?.color ?? "Green";
  const layer = el("div", "bg lt-frame");
  layer.style.background = `url("${lowerThirdFrame(color, seasonOf(service, cue))}") center / 100% 100% no-repeat`;
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
    paintLowerThirdFrame(stage, service, cue);
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

/** A line's indent (its leading tabs) and the line without them. */
export function splitIndent(spans) {
  let level = 0;
  const out = [];
  for (const s of spans) {
    if (!out.length) {
      const rest = s.text.replace(/^\t+/, "");
      level += s.text.length - rest.length;
      if (!rest) continue;
      out.push({ ...s, text: rest });
    } else out.push(s);
  }
  return [level, out];
}

/** Text of a paragraph block: one line each (the Prayer Book's sense
 *  lines, indented as written, too-long lines hanging under themselves),
 *  or run together. `extra(i)` adds indent steps to line i (psalms). */
function paragraphText(b, { reflow, bold, hanging, extra }) {
  const text = el("div", "text");
  b.lines.forEach((l, i) => {
    const [indent, spans] = splitIndent(l);
    if (reflow) {
      if (i > 0) text.append(" ");
      text.append(renderLine(spans, bold));
      return;
    }
    // Sense lines (a paragraph written line by line) hang; prose wraps.
    const line = el("div", hanging && (b.lines.length > 1 || extra) ? "ln hang" : "ln");
    const level = indent + (extra?.(i) ?? 0);
    if (level) line.style.setProperty("--in", level);
    line.append(renderLine(spans, bold));
    text.append(line);
  });
  return text;
}

/** Psalm verses: both halves start level (a half too long for the line
 *  carries on indented under itself); with `alternate`, every other verse
 *  is indented a step. */
function verseIndent(role, alternate) {
  const n = parseInt(role, 10);
  const step = alternate && n % 2 === 0 ? 1 : 0;
  return () => step;
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
    // The people's parts and everyone's together are bold.
    const people = b.role === "People" || b.role === "All";
    if (people) row.classList.add("people");
    if (b.role === "People" && opts.labels && opts.peopleLabel === false) {
      // No "People" label (bold shows it), but keep the text column.
      row.append(el("div", "label"));
    } else if (b.role && opts.labels && !(isVerseNumber(b.role) && !opts.numbers)) {
      row.append(el("div", isVerseNumber(b.role) ? "label number" : "label", b.role));
    } else {
      row.classList.add("span");
    }
    const extra = isVerseNumber(b.role) && opts.psalm ? verseIndent(b.role, opts.alternate) : null;
    row.append(paragraphText(b, { ...opts, extra }));
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
  if (spec.color) node.style.color = ink(spec.color);
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

/** The text box grows up into the space of the boxes above it that this
 *  slide leaves empty (no title, no section heading, a hidden piece). On
 *  the sanctuary's slides it does so across a gap between them (the glass
 *  is one open space), but not past a box that is shown; on the stream's
 *  only into a box that touches it, since its text sits on a drawn panel
 *  or bar (the Bottom Bar's music space above it is glass). */
function textRect(T, shown, key = "") {
  if (!key.startsWith("sanctuary.")) return touchingRect(T, shown);
  const t = { ...T.text };
  const bottom = t.y + t.h;
  const across = (s) => s.x < t.x + t.w && s.x + s.w > t.x;
  const above = Object.entries(T).filter(([name, s]) => name !== "text" && s && s.kind !== "font" && s.x != null && s.y < t.y && across(s));
  const unused = above.filter(([name]) => !shown.has(name)).map(([, s]) => s.y);
  if (unused.length) {
    // Shown boxes above stay clear of the text.
    const floor = Math.max(...above.filter(([name]) => shown.has(name)).map(([, s]) => s.y + s.h), -Infinity);
    t.y = Math.min(t.y, Math.max(Math.min(...unused), floor));
  }
  t.h = bottom - t.y;
  return t;
}

/**
 * Lets the hymn credits grow upward from their box's bottom edge to as many
 * as four lines at their set size (shrinking only beyond that). Returns the
 * credits' top edge, which the verse box above must stay clear of.
 */
function growCredits(node, spec) {
  const bottom = spec.y + spec.h;
  const content = node.firstElementChild ?? node;
  node.style.setProperty("--fit", 1);
  const lineH = parseFloat(getComputedStyle(content).lineHeight) || (spec.size ?? 22) * 1.3;
  const h = Math.max(spec.h, Math.min(content.offsetHeight, Math.ceil(lineH * 4) + 2));
  node.style.top = `${bottom - h}px`;
  node.style.height = `${h}px`;
  // More than four lines: shrink the text until it fits.
  let fit = 1;
  while (content.offsetHeight > h && fit > 0.5) {
    fit = Math.round((fit - 0.04) * 100) / 100;
    node.style.setProperty("--fit", fit);
  }
  return bottom - h;
}

/** The text box grows into empty boxes above it that touch it. */
function touchingRect(T, shown) {
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

function textBox(stage, T, shown, content, cls = "", limits = {}, key = "") {
  const box = el("div", `box tbox ${cls}`);
  const rect = textRect(T, shown, key);
  // Keep clear of the credits when they have grown into the verse box.
  if (limits.bottom != null && rect.y + rect.h > limits.bottom - 12 && limits.bottom - 12 > rect.y) {
    rect.h = limits.bottom - 12 - rect.y;
  }
  styleElement(box, rect);
  box.style.setProperty("--box-h", `${rect.h}px`);
  if (T.labels?.size) box.style.setProperty("--label", `${T.labels.size}px`);
  box.append(content);
  stage.append(box);
  const paint = (sel, spec) => {
    if (!spec) return;
    box.querySelectorAll(sel).forEach((n) => {
      if (spec.color) n.style.color = ink(spec.color);
      if (spec.font) n.style.fontFamily = spec.font;
      if (spec.italic != null) n.style.fontStyle = spec.italic ? "italic" : "normal";
      if (spec.bold != null) n.style.fontWeight = spec.bold ? "700" : "400";
    });
  };
  paint(".label", T.labels);
  if (T.people?.color) box.querySelectorAll(".people .text").forEach((n) => (n.style.color = ink(T.people.color)));
  autoFit(box, 0.5);
  return box;
}

/** The preacher on a sermon slide: the first line of its text is the name
 *  ("The Very Rev. Jane Doe"), the rest the position ("Rector"). */
function preacher(slide) {
  const para = (slide?.blocks ?? []).find((b) => b.kind === "paragraph" && !b.role);
  const lines = (para?.lines ?? []).map((l) => l.map((s) => s.text).join("").trim()).filter(Boolean);
  return { name: lines[0] ?? "", position: lines.slice(1).join(", ") };
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

  if (key === "sanctuary.sermon") {
    const who = preacher(slide);
    p.subtitle = heading ?? cue.title;
    if (who.name) p.title = who.name;
    if (who.position) p.position = who.position;
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
    if (cue.category === "homily" && slide) {
      // The preacher's name in the bar, "The Sermon" and the position above.
      const who = preacher(slide);
      p.title = heading ?? cue.title;
      if (who.position) p.reference = who.position;
      if (who.name) p.big = who.name;
      return p;
    }
    if (cue.category === "anthem" && slide) {
      // "Offertory Anthem" above; the piece and its composer in the bar.
      p.title = heading ?? cue.title;
      p.text = slide.blocks.filter((b) => b.kind !== "image");
      return p;
    }
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
  const limits = {};
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
      if (name === "credits") {
        const top = growCredits(node, spec);
        const t = T.text;
        // Only a box above the credits' column gives way.
        if (t && t.x < spec.x + spec.w && t.x + t.w > spec.x && t.y < top) limits.bottom = top;
      } else autoFit(node, 0.4);
    } else textPiece(stage, name, spec, value);
  }
  if (!T.text || T.text.hidden) return;
  // A title card, or a line of music alone, sits in the middle of its box.
  const centre = (box) => {
    box.style.textAlign = "center";
    box.style.alignItems = "center";
  };
  if (p.big) {
    centre(textBox(stage, T, shown, el("div", key === "stream.lower_third" ? "lt-main" : "title-big", p.big), "title-card", {}, key));
  } else if (p.text) {
    const hymn = key === "stream.hymn";
    const stream = key.startsWith("stream.");
    const blocks = p.text;
    const o = typeOptions(templates, key);
    const lines = { reflow: o.join, hanging: o.hanging, psalm: cue.category === "psalm", alternate: o.alternate };
    const box = textBox(stage, T, shown, renderBlocks(blocks, cue, { labels: true, numbers: true, rubrics: stream, bold: hymn, peopleLabel: !T.people?.hidden, ...lines }), "", limits, key);
    if (key === "stream.lower_third") box.classList.toggle("lt-center", !blocks.some((b) => b.kind === "paragraph" && b.role));
    if (blocks.length && blocks.every((b) => b.kind === "image")) {
      centre(box);
      box.classList.add("music-only");
    }
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
    box.append(el("div", "title-small", "Announcements"), el("div", "deck-missing", "No Google Slides deck is set (⚙ Settings → Service settings)."));
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

// ---------- made slides ----------

/** Slides made in the app (the slide maker, imported PDF and Google Slides
 *  pages): each a title slide, information with a title, or information
 *  alone, as text or a picture. */
export const isMadeSlides = (cue) => extra(cue, "slide_maker") === "true";

/** Which of the three layouts a made slide has. */
export function madeLayout(slide) {
  if (slide?.section_start && slide.level === 1) return "title";
  if (slide?.section_start && slide.heading) return "titled";
  return "plain";
}

/** A picture on a made slide: a library image, or a page of a PDF
 *  (`….pdf#page=3`) or of a Google Slides deck (`deck:<link>#page=3`). */
function madeImage(cue, b) {
  const img = el("img", `made-img fit-${b.fit ?? "center"}`);
  img.alt = b.alt ?? "";
  const page = /^(.*?)#page=(\d+)$/.exec(b.src ?? "");
  if (page) {
    const deck = page[1].startsWith("deck:");
    const url = deck ? page[1].slice(5) : assetUrl(cue, page[1]);
    // A Google Slides deck is downloaded the first time a computer shows it.
    (deck ? loadDeck(url, true) : Promise.resolve())
      .then(() => slideImage(url, Number(page[2])))
      .then((u) => {
        if (u) img.src = u;
        else img.replaceWith(el("div", "score-missing", `▭ ${b.alt || "slide"} ${page[2]}`));
      });
  } else {
    img.src = assetUrl(cue, b.src);
    img.onerror = () => img.replaceWith(el("div", "score-missing", `▭ ${b.alt || b.src}`));
  }
  return img;
}

function madeSlide(stage, cue, slide) {
  if (!slide) return;
  const image = slide.blocks.find((b) => b.kind === "image" && b.src);
  const text = slide.blocks.filter((b) => b.kind === "paragraph");
  if (image?.fit === "full") {
    stage.append(madeImage(cue, image));
    return;
  }
  const layout = madeLayout(slide);
  const body = (cls) => {
    const box = el("div", `box made-body ${cls}`);
    if (image) box.append(madeImage(cue, image));
    else if (text.length) box.append(renderBlocks(text, cue, { labels: false, numbers: false, rubrics: false }));
    return box;
  };
  if (layout === "title") {
    const box = el("div", "box made-title");
    box.append(el("div", "made-title-text", slide.heading ?? ""));
    const sub = text.map((b) => b.lines.map((l) => splitIndent(l)[1].map((x) => x.text).join("")).join("\n")).join("\n");
    if (sub) box.append(el("div", "made-subtitle", sub));
    stage.append(box);
    autoFit(box, 0.4);
    return;
  }
  if (layout === "titled") {
    const head = el("div", "box made-head", slide.heading);
    stage.append(head);
    autoFit(head, 0.5);
  }
  const box = body(layout === "titled" ? "under-title" : "alone");
  stage.append(box);
  if (!image) autoFit(box, 0.4);
}

// ---------- web video ----------

/** The embeddable player for a YouTube, Facebook or Instagram link. */
export function embedUrl(url, { sound = true } = {}) {
  let u;
  try {
    u = new URL(url);
  } catch {
    return null;
  }
  const host = u.hostname.replace(/^(www|m|web)\./, "");
  if (/(^|\.)youtube(-nocookie)?\.com$|^youtu\.be$/.test(host)) {
    const id =
      host === "youtu.be"
        ? u.pathname.slice(1).split("/")[0]
        : u.searchParams.get("v") ?? /\/(?:embed|shorts|live)\/([^/?#]+)/.exec(u.pathname)?.[1];
    if (!id) return null;
    const start = parseInt(u.searchParams.get("t") ?? u.searchParams.get("start") ?? "0", 10) || 0;
    return `https://www.youtube.com/embed/${id}?autoplay=1&mute=${sound ? 0 : 1}&controls=0&rel=0&playsinline=1${start ? `&start=${start}` : ""}`;
  }
  if (host === "facebook.com" || host === "fb.watch") {
    return `https://www.facebook.com/plugins/video.php?href=${encodeURIComponent(url)}&autoplay=true&mute=${sound ? "false" : "true"}&show_text=false&width=1920`;
  }
  if (host === "instagram.com") {
    const m = /\/(p|reel|tv)\/([^/?#]+)/.exec(u.pathname);
    return m ? `https://www.instagram.com/${m[1]}/${m[2]}/embed/` : null;
  }
  return null;
}

/** A still for a web video's previews (YouTube's own thumbnail). */
function videoStill(url) {
  const id = /(?:v=|youtu\.be\/|\/(?:embed|shorts|live)\/)([\w-]{6,})/.exec(url)?.[1];
  return id ? `https://img.youtube.com/vi/${id}/hqdefault.jpg` : null;
}

function webVideo(stage, url, output, opts) {
  stage.style.background = "#000";
  // Sound from the sanctuary screen only; the stream plays it muted.
  const src = embedUrl(url, { sound: output === "sanctuary" });
  if (!opts.live || !src) {
    const card = el("div", "web-video-card");
    const still = videoStill(url);
    if (still) {
      const img = el("img", "web-video-still");
      img.src = still;
      card.append(img);
    }
    const site = /youtu/.test(url) ? "YouTube" : /facebook|fb\.watch/.test(url) ? "Facebook" : /instagram/.test(url) ? "Instagram" : "Web";
    card.append(el("div", "web-video-label", src ? `▶ ${site} video` : "This link cannot be played"));
    stage.append(card);
    return;
  }
  const keep = opts.keepWeb;
  if (keep && keep.dataset.src === src) {
    stage.append(keep);
    return;
  }
  const frame = el("iframe", "web-video");
  frame.dataset.src = src;
  frame.src = src;
  frame.allow = "autoplay; encrypted-media; fullscreen; picture-in-picture";
  frame.referrerPolicy = "strict-origin-when-cross-origin";
  stage.append(frame);
}

// ---------- outputs ----------

/** The slide a screen shows at step `i` of a cue: the stream has its own
 *  slides when it runs the text together or splits it differently. */
export function slideFor(cue, i, output) {
  const c = cue?.content;
  if (c?.kind !== "liturgy") return null;
  if (output === "stream" && c.stream_slides?.length) return c.stream_slides[c.stream_at?.[i] ?? 0] ?? null;
  return c.slides?.[i] ?? null;
}

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
  const keepWeb = stage.querySelector("iframe.web-video");
  lightBox = false;
  stage.replaceChildren();
  stage.className = `stage out-${output}`;
  stage.style.background = output === "stream" ? keyColor(service) : "#000";

  // Blackout is for the sanctuary screen only; the live stream carries on.
  if (st.blackout && output === "sanctuary") return;

  const cue = active?.cue;
  if (!cue) return renderPreService(stage, service, output, { ...opts, keepDeck }, st);
  const slide = output === "stream" ? slideFor(cue, active.pos?.slide ?? 0, "stream") ?? active.slide : active.slide;
  // A slide may have a look of its own (the Sanctus sung within a
  // Eucharistic Prayer is shown as a hymn).
  const look = slide?.[output] ?? cue[output];
  if (!look) return;
  // A season's lower-third box may be light (White): its words are dark.
  const boxText = look.layout === "lower_third" && look.background === "key" && look.tone === "light" ? lowerThirdText(cue.color ?? service?.color, seasonOf(service, cue)) : null;
  stage.classList.add(`layout-${look.layout}`, `tone-${boxText ?? look.tone}`, `cat-${cue.category}`);
  lightBox = boxText === "dark";
  // Planned blank (or an item for the other screen only): black on the
  // sanctuary, just the key colour (the camera) on the stream.
  if (look.blank) return;

  if (cue.content.kind === "announcements" && !st.media_cleared) {
    const single = cue.content.mode !== "loop";
    const steps = cue.content.slides ?? 1;
    return paintDeck(stage, service, { ...opts, keepDeck }, single && steps > 1 ? { slide: (active.pos?.slide ?? 0) + 1, of: steps } : { loop: !single });
  }

  // A YouTube, Facebook or Instagram video, full screen.
  const web = cue.content.kind === "liturgy" ? extra(cue, "web_video") : null;
  if (web) {
    if (st.media_cleared) return;
    return webVideo(stage, web, output, { ...opts, keepWeb });
  }

  if (!st.media_cleared) paintBackground(stage, service, cue, look, { ...opts, keepVideo, sound: output === "sanctuary" });
  else if (look.layout === "lower_third" || output === "stream") stage.style.background = keyColor(service);
  if (st.text_cleared) return;

  // A media item (an image or video from the library) is just its background.
  if (!slide && cue.content.kind === "liturgy" && extra(cue, "media")) return;

  // Slides made in the app, and imported pages.
  if (isMadeSlides(cue)) return madeSlide(stage, cue, slide);

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
    // "The Holy Eucharist on …, November 8, 2026, 10:30 a.m." / "Welcome to
    // the Cathedral of St. James" / "Prelude: Adagio, Johann Sebastian Bach".
    const box = el("div", "box ib");
    const w = service.welcome;
    const line = w?.line ?? [service.title, formatDate(service.date)].filter(Boolean).join(", ");
    box.append(el("div", "ib-small", line), el("div", "ib-big", w?.greeting ?? "Welcome"));
    const prelude = service.prelude;
    if (prelude && (prelude.title || prelude.composer)) {
      const p = el("div", "ib-small ib-prelude", "Prelude: ");
      if (prelude.title) p.append(el("i", null, prelude.composer ? `${prelude.title},` : prelude.title));
      if (prelude.composer) p.append(`${prelude.title ? " " : ""}${prelude.composer}`);
      box.append(p);
    }
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
