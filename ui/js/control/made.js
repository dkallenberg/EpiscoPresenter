// Slides made in the app, imported presentations and web videos.
//
// * The slide maker (an Editor panel): an item of simple slides, each a
//   Title slide, Information with a title, or Information without one; the
//   information is text, or a picture from the media library shown in the
//   middle, the box's width, the box's height or full screen.
// * + Add → Slides from a PDF (a PowerPoint, Keynote or Google Slides
//   presentation saved as PDF), Google Slides (a link), Web video.
//
// In the item's document a slide is a `#` heading (Title), a `##` heading
// (Information with title) or neither, then its text or picture, with `---`
// between slides; pictures carry their size as `{center|width|height|full}`.

import { assetUrl } from "/js/bridge.js";
import { load as loadDeck } from "/js/deckpdf.js";
import { mountStage, renderOutput } from "/js/render.js";
import { pickMedia } from "/js/control/media.js";
import { app, call, debounce, el, toast } from "/js/control/util.js";

/** Keeps an empty title or slide from vanishing (the file trims spaces). */
const KEEP = "⁠";

export const LAYOUTS = [
  ["title", "Title"],
  ["titled", "Information with title"],
  ["plain", "Information without title"],
];

export const FITS = [
  ["center", "In the middle (its own size)"],
  ["width", "Box width"],
  ["height", "Box height"],
  ["full", "Full screen"],
];

const lineText = (l) => l.map((s) => s.text).join("").replace(/^\t+/, "");

/** The slides of a document as cards the panel edits. */
export function toCards(blocks) {
  const chunks = [[]];
  for (const b of blocks) {
    if (b.kind === "slide_break" || b.kind === "output_break") chunks.push([]);
    else chunks[chunks.length - 1].push(b);
  }
  return chunks
    .filter((c) => c.length)
    .map((chunk) => {
      const h = chunk[0].kind === "heading" ? chunk[0] : null;
      const rest = h ? chunk.slice(1) : chunk;
      const img = rest.find((b) => b.kind === "image");
      const text = rest
        .filter((b) => b.kind === "paragraph")
        .map((p) => p.lines.map(lineText).join("\n"))
        .join("\n\n")
        .replaceAll(KEEP, "");
      return {
        layout: h ? (h.level === 1 ? "title" : "titled") : "plain",
        title: (h?.text ?? "").replaceAll(KEEP, ""),
        content: img ? "image" : "text",
        text,
        image: img ? { src: img.src, alt: img.alt ?? "", fit: img.fit ?? "center" } : null,
      };
    });
}

/** One card's blocks (without the break before it). */
function cardBlocks(c) {
  const out = [];
  if (c.layout !== "plain") out.push({ kind: "heading", level: c.layout === "title" ? 1 : 2, text: c.title.trim() || KEEP });
  if (c.layout !== "title" && c.content === "image" && c.image?.src) {
    out.push({ kind: "image", alt: c.image.alt ?? "", src: c.image.src, fit: c.image.fit ?? "center" });
  } else if (c.text.trim()) {
    for (const para of c.text.split(/\n\s*\n/)) {
      const lines = para.split("\n").filter((l) => l.trim()).map((l) => [{ text: l.trim() }]);
      if (lines.length) out.push({ kind: "paragraph", lines });
    }
  } else if (c.layout === "plain") {
    out.push({ kind: "paragraph", lines: [[{ text: KEEP }]] }); // a blank slide
  }
  return out;
}

/** The document's blocks for the cards. */
export function toBlocks(cards) {
  const out = [];
  cards.forEach((c, i) => {
    if (i) out.push({ kind: "slide_break" });
    out.push(...cardBlocks(c));
  });
  return out;
}

/** A card as the slide the screens show (for its preview). */
function cardSlide(c) {
  const blocks = cardBlocks(c);
  const heading = blocks[0]?.kind === "heading" ? blocks.shift() : null;
  return { heading: heading?.text ?? null, section_start: !!heading, level: heading?.level ?? null, blocks };
}

const newCard = (layout) => ({ layout, title: "", content: "text", text: "", image: null });

/**
 * The Editor's slide maker for an item of made slides. `doc` is the
 * document being edited (its blocks are rewritten); `onChange` saves.
 */
export function slideMakerPanel(cue, doc, onChange) {
  let cards = toCards(doc.blocks);
  const list = el("div.maker-list");
  const save = () => {
    doc.blocks = toBlocks(cards);
    onChange();
  };
  const previews = [];
  const drawPreview = (i) => {
    const host = previews[i];
    if (!host) return;
    host.replaceChildren();
    const stage = mountStage(host);
    renderOutput(stage, { service: app.service, state: {} }, { cue, slide: cardSlide(cards[i]), pos: { cue: -1, slide: i } }, "sanctuary");
  };
  const later = cards.map(() => null);
  const edited = (i) => {
    save();
    later[i] ??= debounce(() => drawPreview(i), 250);
    later[i]();
  };
  const draw = () => {
    previews.length = 0;
    list.replaceChildren(...cards.map((c, i) => card(c, i)));
    if (!cards.length) list.append(el("p.muted", {}, "No slides yet: add one below."));
    cards.forEach((_, i) => drawPreview(i));
  };
  const restructure = () => {
    save();
    later.length = 0;
    draw();
  };
  const ro = app.readonly;
  const card = (c, i) => {
    const thumb = el("div.thumb.maker-thumb");
    previews[i] = thumb;
    const layout = el(
      "select",
      { disabled: ro, onchange: (e) => { c.layout = e.target.value; restructure(); } },
      LAYOUTS.map(([v, l]) => el("option", { value: v, selected: v === c.layout }, l)),
    );
    const title =
      c.layout === "plain"
        ? null
        : el("input.maker-title", { value: c.title, placeholder: c.layout === "title" ? "Title" : "Title of the slide", disabled: ro, oninput: (e) => { c.title = e.target.value; edited(i); } });
    const text = (placeholder) =>
      el("textarea.maker-text", { rows: 4, value: c.text, placeholder, disabled: ro, oninput: (e) => { c.text = e.target.value; edited(i); } });
    let body;
    if (c.layout === "title") body = text("A line under the title (optional)");
    else {
      const kind = el(
        "select",
        { disabled: ro, onchange: (e) => { c.content = e.target.value; restructure(); } },
        [["text", "Text"], ["image", "Picture"]].map(([v, l]) => el("option", { value: v, selected: v === c.content }, l)),
      );
      let content;
      if (c.content === "image") {
        const img = c.image;
        const fit = el(
          "select",
          { disabled: ro || !img, onchange: (e) => { c.image.fit = e.target.value; edited(i); } },
          FITS.map(([v, l]) => el("option", { value: v, selected: v === (img?.fit ?? "center") }, l)),
        );
        const name = img ? (img.src.startsWith("deck:") ? `Google Slides, slide ${img.src.split("#page=")[1] ?? ""}` : img.src.split(/[\\/]/).pop()) : "No picture yet";
        content = el(
          "div.maker-picture",
          {},
          el("span.muted.small", {}, name),
          ro
            ? null
            : el("button.small", {
                onclick: async () => {
                  const path = await pickMedia({ kind: "image", title: "Choose a picture for the slide" });
                  if (!path) return;
                  c.image = { src: path, alt: c.image?.alt ?? "", fit: c.image?.fit ?? "center" };
                  restructure();
                },
              }, img ? "Change…" : "Choose…"),
          el("label.field.inline", {}, el("span", {}, "Size"), fit),
        );
      } else content = text("The information: a blank line starts a new paragraph");
      body = el("div.maker-body", {}, el("label.field.inline", {}, el("span", {}, "Show"), kind), content);
    }
    const move = (to) => {
      const [x] = cards.splice(i, 1);
      cards.splice(to, 0, x);
      restructure();
    };
    return el(
      "div.maker-card",
      { dataset: { index: i } },
      el(
        "div.maker-side",
        {},
        el("div.maker-head", {}, el("strong", {}, `Slide ${i + 1}`), layout),
        title,
        body,
        ro
          ? null
          : el(
              "div.maker-tools",
              {},
              el("button.icon", { title: "Move up", disabled: i === 0, onclick: () => move(i - 1) }, "↑"),
              el("button.icon", { title: "Move down", disabled: i === cards.length - 1, onclick: () => move(i + 1) }, "↓"),
              el("button.icon", { title: "Copy this slide", onclick: () => { cards.splice(i + 1, 0, structuredClone(c)); restructure(); } }, "⧉"),
              el("button.icon.danger", { title: "Delete this slide", onclick: () => { cards.splice(i, 1); restructure(); } }, "✕"),
            ),
      ),
      thumb,
    );
  };
  const add = (layout) => {
    cards.push(newCard(layout));
    restructure();
    list.lastElementChild?.querySelector("input, textarea")?.focus();
  };
  const panel = el(
    "section.section.maker",
    {},
    el("div.maker-intro", {}, el("strong", {}, "Slides"), el("span.muted.small", {}, " · each slide: a Title, Information with a title, or Information alone (text or a picture)")),
    list,
    ro
      ? null
      : el(
          "div.maker-add",
          {},
          el("span.muted.small", {}, "Add a slide:"),
          LAYOUTS.map(([v, l]) => el("button.small", { onclick: () => add(v) }, `+ ${l}`)),
        ),
  );
  draw();
  return panel;
}

/** Downloads a Google Slides item's presentation again and remakes its
 *  slides (the number of slides may have changed). */
export async function refreshGoogleSlides(doc, url, onChange) {
  const file = await call("deck_fetch", { url, refresh: true });
  if (!file) return toast("The presentation could not be downloaded.", "error");
  const pages = await loadDeck(url, true);
  if (!pages) return toast("The presentation has no slides.", "error");
  doc.blocks = pageSlides(doc.meta.title, pages, (n) => `deck:${url}#page=${n}`).blocks;
  onChange();
  toast(`Downloaded again: ${pages} slide${pages === 1 ? "" : "s"}`);
}

/** The Editor's panel for a web video: its link. */
export function webVideoPanel(link, onLink) {
  const input = el("input.maker-title", { value: link, disabled: app.readonly, oninput: (e) => onLink(e.target.value.trim()) });
  return el(
    "section.section.maker",
    {},
    el("label.field", {}, el("span", {}, "Video link (YouTube, Facebook or Instagram)"), input),
    el(
      "p.muted.small",
      {},
      "It plays full screen when the item is shown: with sound from the sanctuary screen, muted on the live stream (stream it from its own source if the stream needs the sound). It needs the internet, and the video must allow embedding. Going to the next item stops it.",
    ),
  );
}

// ---------- + Add ----------

/** A small dialog with text fields; resolves to their values, or null. */
export function ask(title, fields, okLabel = "Add", note = null) {
  return new Promise((resolve) => {
    const inputs = fields.map(([, placeholder, value = ""]) => el("input", { placeholder, value }));
    const done = (v) => {
      overlay.remove();
      resolve(v);
    };
    const ok = () => done(inputs.map((i) => i.value.trim()));
    inputs.forEach((i) => i.addEventListener("keydown", (e) => e.key === "Enter" && ok()));
    const overlay = el(
      "div.modal-overlay",
      { onmousedown: (e) => e.target === overlay && done(null) },
      el(
        "div.modal",
        {},
        el("h3", {}, title),
        note ? el("p.muted.small", {}, note) : null,
        fields.map(([label], k) => el("label.field", {}, el("span", {}, label), inputs[k])),
        el("div.modal-actions", {}, el("button", { onclick: () => done(null) }, "Cancel"), el("button.primary", { onclick: ok }, okLabel)),
      ),
    );
    document.body.append(overlay);
    inputs[0]?.focus();
  });
}

/** Slides whose pages are full-screen pictures: `src(n)` is page n. */
function pageSlides(title, pages, src) {
  const blocks = [];
  for (let n = 1; n <= pages; n++) {
    if (n > 1) blocks.push({ kind: "slide_break" });
    blocks.push({ kind: "image", alt: `Slide ${n}`, src: src(n), fit: "full" });
  }
  return { meta: { title, extra: [] }, blocks };
}

async function added(id) {
  if (!id) return;
  app.select(id);
  document.querySelector('.tabs button[data-tab="editor"]')?.click();
}

/** + Add → New slides: one title slide to start from. */
export async function addSlides(after) {
  await added(await call("slides_add", { after, title: null, document: null }));
}

/** + Add → Slides from a PDF: a presentation saved as PDF, one slide a page. */
export async function addPdfSlides(after) {
  const path = await call("slides_pick_pdf");
  if (!path) return;
  const pages = await loadDeck(assetUrl(null, path));
  if (!pages) return toast("That PDF could not be read.", "error");
  const title = path.split(/[\\/]/).pop().replace(/\.pdf$/i, "");
  const id = await call("slides_add", { after, title, document: pageSlides(title, pages, (n) => `${path}#page=${n}`) });
  if (id) toast(`${pages} slide${pages === 1 ? "" : "s"} from ${title}`);
  await added(id);
}

/** + Add → Google Slides: a presentation by its link, one slide a page. */
export async function addGoogleSlides(after) {
  const r = await ask(
    "Google Slides",
    [["Link to the presentation", "https://docs.google.com/presentation/d/…"], ["Title", "e.g. Sermon slides"]],
    "Add",
    "Share the presentation as “Anyone with the link can view”. The app downloads it, so it shows without the internet once added; ↻ in the Editor downloads it again after changes.",
  );
  if (!r?.[0]) return;
  const [url, name] = r;
  const file = await call("deck_fetch", { url, refresh: true });
  if (!file) return toast("That presentation could not be downloaded: check the link and its sharing.", "error");
  const pages = await loadDeck(url);
  if (!pages) return toast("The presentation has no slides.", "error");
  const title = name || "Google Slides";
  const doc = pageSlides(title, pages, (n) => `deck:${url}#page=${n}`);
  doc.meta.extra.push(["deck", url]);
  await added(await call("slides_add", { after, title, document: doc }));
}

/** + Add → Web video. */
export async function addWebVideo(after) {
  const r = await ask(
    "Web video",
    [["Link (YouTube, Facebook or Instagram)", "https://www.youtube.com/watch?v=…"], ["Title", "e.g. Bishop's message"]],
    "Add",
    "The video plays full screen when the item is shown. It needs the internet, and its owner must allow it to be embedded.",
  );
  if (!r?.[0]) return;
  await added(await call("web_video_add", { after, url: r[0], title: r[1] || null }));
}
