// The Layout tool: for each slide type (Stream: Left side, Bottom Bar, Lower
// Third, Hymn; Sanctuary: Prayer, Title, Psalm) drag the text box, title,
// reference and other pieces into place and set their font, size and colour.
// Changes apply to every service and save as soon as they are made.

import { backgroundCss, isMadeSlides, mountStage, renderTemplated } from "/js/render.js";
import { FONTS, SLIDE_TYPES, resolveTemplate, slideType, slideTypeFor, typeOptions } from "/js/templates.js";
import { app, call, debounce, el, toast } from "/js/control/util.js";

const STAGE_W = 1920;
const STAGE_H = 1080;

const lt = {
  key: SLIDE_TYPES[0].key,
  selected: "text",
  templates: {}, // the operator's changes, as saved
  sample: 0, // index into the samples for the current type
  root: null,
};

// ---------- sample content ----------

const para = (role, ...lines) => ({
  kind: "paragraph",
  role,
  lines: lines.map((t) => [{ text: t, bold: role === "People" || role === "Antiphon" }]),
});

function makeCue(title, category, meta, slides, extra = []) {
  return {
    id: `sample-${category}`,
    title,
    category,
    caption: title,
    color: app.service?.color ?? "Green",
    content: { kind: "liturgy", source: null, document: { meta: { title, ...meta, extra }, blocks: [] }, slides },
  };
}

/** Built-in sample slides for a slide type, used when the service has none. */
function builtinSamples(type) {
  switch (type.sample) {
    case "psalm":
      return [
        {
          label: "Psalm verses",
          cue: makeCue("Psalm 96", "psalm", { citation: "Psalm 96" }, [], [["setting_image", ""]]),
          slide: {
            heading: "Psalm 96",
            section_start: true,
            blocks: [
              para("1", "Sing to the Lord a new song; *", "sing to the Lord, all the whole earth."),
              { ...para("2", "Sing to the Lord and bless his Name; *", "proclaim the good news of his salvation from day to day."), lines: [[{ text: "Sing to the Lord and bless his Name; *", bold: true }], [{ text: "proclaim the good news of his salvation from day to day.", bold: true }]] },
            ],
          },
        },
        {
          label: "Antiphon",
          cue: makeCue("Psalm 96", "psalm", { citation: "Psalm 96" }, []),
          slide: { heading: "Antiphon", section_start: true, blocks: [para("Antiphon", "Sing to the Lord a new song.")] },
        },
      ];
    case "reading":
      return [
        {
          label: "Title",
          cue: makeCue("First Reading", "reading", { citation: "Joshua 24:1-3a, 14-25" }, []),
          slide: { heading: "The First Reading", section_start: true, blocks: [] },
        },
        {
          label: "Response",
          cue: makeCue("First Reading", "reading", { citation: "Joshua 24:1-3a, 14-25" }, []),
          slide: { heading: "The First Reading", section_start: false, blocks: [para("Reader", "The word of the Lord."), para("People", "Thanks be to God.")] },
        },
      ];
    case "hymn":
      return [
        {
          label: "Verse",
          cue: makeCue("Entrance Hymn", "hymn", { hymn_number: "436", title: "Lift Up Your Heads, Ye Mighty Gates" }, [], [
            ["words", "Georg Weissel (1590-1635)"],
            ["music", "Truro"],
          ]),
          slide: { heading: "Verse 1", section_start: true, blocks: [para("1", "Lift up your heads, ye mighty gates;", "behold, the King of glory waits;", "the King of kings is drawing near;", "the Savior of the world is here!")] },
        },
      ];
    case "sermon":
      return [
        {
          label: "Sermon",
          cue: makeCue("The Sermon", "homily", {}, []),
          slide: { heading: "The Sermon", section_start: true, blocks: [para(null, "The Very Rev. Jane Doe", "Rector")] },
        },
      ];
    case "title":
      return [
        {
          label: "Title",
          cue: makeCue("First Reading", "reading", { citation: "Joshua 24:1-3a, 14-25" }, []),
          slide: { heading: "The First Reading", section_start: true, blocks: [] },
        },
      ];
    default:
      return [
        {
          label: "Prayer",
          cue: makeCue("Collect for Purity", "collect", { citation: "BCP 355" }, []),
          slide: {
            heading: "Collect for Purity",
            section_start: true,
            blocks: [
              para("Celebrant", "Almighty God, to you all hearts are open, all desires known, and from you no secrets are hid: Cleanse the thoughts of our hearts by the inspiration of your Holy Spirit, that we may perfectly love you, and worthily magnify your holy Name; through Christ our Lord."),
              para("People", "Amen."),
            ],
          },
        },
      ];
  }
}

/** Slides from the open service that use this slide type, then the built-ins. */
function samples(type) {
  const out = [];
  for (const cue of app.service?.cues ?? []) {
    const look = cue[type.output];
    if (!look || cue.content.kind !== "liturgy") continue;
    // Made slides and web videos have their own layouts.
    if (isMadeSlides(cue) || cue.content.document?.meta.extra?.some(([k]) => k === "web_video")) continue;
    const own = type.output === "stream" ? cue.content.stream_slides : null;
    const slides = own?.length ? own : cue.content.slides ?? [];
    for (let i = 0; i < slides.length && out.length < 40; i++) {
      if (slideTypeFor(cue, slides[i], type.output, look) === type.key) {
        out.push({ label: `${cue.title} · slide ${i + 1}`, cue, slide: slides[i], look });
        break; // one per item is enough
      }
    }
  }
  for (const s of builtinSamples(type)) out.push({ ...s, label: `Sample: ${s.label}` });
  return out;
}

// ---------- saving ----------

const save = debounce(async () => {
  const t = lt.templates[lt.key] ?? { elements: {} };
  const saved = await call("layouts_set", { key: lt.key, template: t });
  if (saved) lt.templates = saved;
}, 400);

/** Shows or hides the "People" label on every slide type that has one. */
async function setPeopleLabel(show) {
  for (const type of SLIDE_TYPES.filter((t) => t.elements.people)) {
    const t = (lt.templates[type.key] ??= { elements: {} });
    const e = ((t.elements ??= {}).people ??= {});
    if (show) delete e.hidden;
    else e.hidden = true;
    if (!Object.keys(e).length) delete t.elements.people;
    const saved = await call("layouts_set", { key: type.key, template: t });
    if (saved) lt.templates = saved;
  }
  toast(show ? "“People” label shown on the slides" : "“People” label left off the slides");
  draw();
}

/** Sets one property of the selected element (null: back to default). */
function setProp(name, prop, value) {
  const def = slideType(lt.key).elements[name];
  const t = (lt.templates[lt.key] ??= { elements: {} });
  t.elements ??= {};
  const e = (t.elements[name] ??= {});
  if (value == null || value === "" || value === def[prop]) delete e[prop];
  else e[prop] = value;
  if (!Object.keys(e).length) delete t.elements[name];
  save();
  draw();
}

// ---------- UI ----------

export function openLayouts() {
  if (lt.root) return;
  lt.templates = structuredClone(app.service?.templates ?? {});
  call("layouts_get").then((t) => {
    if (t) {
      lt.templates = t;
      draw();
    }
  });
  lt.root = el("div.layout-tool", { tabIndex: -1, onkeydown: keys });
  document.body.append(lt.root);
  build();
  lt.root.focus();
}

function close() {
  lt.root?.remove();
  lt.root = null;
}

let stageHost, stage, overlay, inspector, sampleSel;

function build() {
  const groups = ["stream", "sanctuary"].map((output) =>
    el(
      "div.lyt-group",
      {},
      el("h2", {}, output === "stream" ? "Live Stream" : "Sanctuary"),
      SLIDE_TYPES.filter((t) => t.output === output).map((t) =>
        el(
          `button.lyt-type${t.key === lt.key ? ".active" : ""}`,
          {
            dataset: { key: t.key },
            onclick: () => {
              lt.key = t.key;
              lt.sample = 0;
              lt.selected = "text" in t.elements ? "text" : Object.keys(t.elements)[0];
              build();
            },
          },
          t.label,
          lt.templates[t.key] ? el("span.badge", {}, "changed") : null,
        ),
      ),
    ),
  );
  const type = slideType(lt.key);
  stageHost = el("div.lyt-stage-host");
  overlay = el("div.lyt-overlay");
  inspector = el("div.lyt-inspector");
  const list = samples(type);
  if (lt.sample >= list.length) lt.sample = 0;
  sampleSel = el(
    "select",
    { onchange: (e) => { lt.sample = Number(e.target.value); draw(); } },
    list.map((s, i) => el("option", { value: i, selected: i === lt.sample }, s.label)),
  );
  lt.root.replaceChildren(
    el(
      "div.lyt-head",
      {},
      el("h1", {}, "Slide layouts"),
      el("span.muted", {}, "Drag the boxes to place each piece; set fonts on the right. Changes apply to every service."),
      el("span.spacer"),
      app.readonly ? el("span.warn-note", {}, "Read-only on a slave node: edit layouts on the master.") : null,
      el("button.primary", { onclick: close }, "Done"),
    ),
    el(
      "div.lyt-body",
      {},
      el("div.lyt-types", {}, groups),
      el(
        "div.lyt-main",
        {},
        el(
          "div.lyt-toolbar",
          {},
          el("strong", {}, `${type.output === "stream" ? "Live Stream" : "Sanctuary"} · ${type.label}`),
          el("span.muted.small", {}, type.hint),
          el("span.spacer"),
          el("label.field.inline", {}, el("span", {}, "Preview"), sampleSel),
        ),
        el("div.lyt-canvas", {}, stageHost),
        el("p.muted.small", {}, "Click a box to select it · drag to move · drag the corner to resize · arrow keys nudge (⇧ for 10 px). Dashed boxes are not shown on this slide."),
      ),
      inspector,
    ),
  );
  stage = mountStage(stageHost);
  stageHost.append(overlay);
  new ResizeObserver(scaleOverlay).observe(stageHost);
  scaleOverlay();
  draw();
}

const scale = () => Math.min(stageHost.clientWidth / STAGE_W, stageHost.clientHeight / STAGE_H) || 1;

function scaleOverlay() {
  overlay.style.transform = `scale(${scale()})`;
}

function draw() {
  if (!lt.root) return;
  const type = slideType(lt.key);
  const list = samples(type);
  const sample = list[lt.sample] ?? list[0];
  const T = resolveTemplate(lt.templates, lt.key);

  // The slide, rendered with the layout being edited.
  const cue = sample.cue;
  const look = { background: type.background, layout: type.layout, tone: type.layout === "centered" ? "dark" : "light", fit: "fill", ...(sample.look ?? {}) };
  stage.replaceChildren();
  stage.className = `stage out-${type.output} layout-${look.layout} tone-${look.tone} cat-${cue.category}`;
  stage.style.background = type.output === "stream" ? app.service?.stream_key || "#0000ff" : "#000";
  const bg = el("div.bg");
  bg.style.background = backgroundCss(app.service, cue, look.background, look.fit);
  stage.append(bg);
  renderTemplated(stage, lt.key, { ...cue, [type.output]: look }, sample.slide, lt.templates);
  const shownNames = new Set([...stage.querySelectorAll("[class*='piece-'], .tbox")].flatMap((n) =>
    [...n.classList].filter((c) => c.startsWith("piece-")).map((c) => c.slice(6)).concat(n.classList.contains("tbox") && !n.className.includes("piece-") ? ["text"] : []),
  ));

  // Handles for every placed element.
  overlay.replaceChildren(
    ...Object.entries(T)
      .filter(([, spec]) => spec.kind !== "font")
      .map(([name, spec]) => {
        const box = el(
          `div.lyt-box${name === lt.selected ? ".selected" : ""}${shownNames.has(name) ? "" : ".absent"}${spec.hidden ? ".hidden-el" : ""}`,
          { dataset: { name }, title: spec.label },
          el("span.lyt-box-label", {}, spec.label + (spec.hidden ? " (hidden)" : "")),
          el("span.lyt-resize"),
        );
        Object.assign(box.style, { left: `${spec.x}px`, top: `${spec.y}px`, width: `${spec.w}px`, height: `${spec.h}px` });
        box.addEventListener("pointerdown", (e) => startDrag(e, name, e.target.classList.contains("lyt-resize")));
        return box;
      }),
  );
  drawInspector(T);
  document.querySelectorAll(".lyt-type").forEach((b) => {
    const changed = Boolean(lt.templates[b.dataset.key]);
    const badge = b.querySelector(".badge");
    if (changed && !badge) b.append(el("span.badge", {}, "changed"));
    if (!changed && badge) badge.remove();
  });
}

function startDrag(e, name, resize) {
  if (app.readonly) return;
  e.preventDefault();
  e.stopPropagation();
  lt.selected = name;
  const T = resolveTemplate(lt.templates, lt.key);
  const start = { ...T[name] };
  const s = scale();
  const x0 = e.clientX;
  const y0 = e.clientY;
  const target = e.currentTarget;
  target.setPointerCapture(e.pointerId);
  let moved = false;
  const snap = (v) => Math.round(v / 2) * 2;
  const move = (ev) => {
    const dx = (ev.clientX - x0) / s;
    const dy = (ev.clientY - y0) / s;
    if (Math.abs(dx) + Math.abs(dy) < 2 && !moved) return;
    moved = true;
    const t = (lt.templates[lt.key] ??= { elements: {} });
    const el2 = ((t.elements ??= {})[name] ??= {});
    if (resize) {
      el2.w = Math.max(40, snap(start.w + dx));
      el2.h = Math.max(20, snap(start.h + dy));
    } else {
      el2.x = Math.min(STAGE_W - 20, Math.max(-start.w + 20, snap(start.x + dx)));
      el2.y = Math.min(STAGE_H - 20, Math.max(-start.h + 20, snap(start.y + dy)));
    }
    target.style.left = `${el2.x ?? start.x}px`;
    target.style.top = `${el2.y ?? start.y}px`;
    target.style.width = `${el2.w ?? start.w}px`;
    target.style.height = `${el2.h ?? start.h}px`;
  };
  const up = () => {
    target.removeEventListener("pointermove", move);
    target.removeEventListener("pointerup", up);
    if (moved) save();
    draw();
  };
  target.addEventListener("pointermove", move);
  target.addEventListener("pointerup", up);
  target.classList.add("selected");
}

function keys(e) {
  if (e.target.closest("input, select, textarea")) return;
  if (e.key === "Escape") return close();
  const d = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
  if (!d || app.readonly) return;
  e.preventDefault();
  e.stopPropagation();
  const T = resolveTemplate(lt.templates, lt.key);
  const spec = T[lt.selected];
  if (!spec || spec.x == null) return;
  const step = e.shiftKey ? 10 : 1;
  setProp(lt.selected, "x", spec.x + d[0] * step);
  setProp(lt.selected, "y", spec.y + d[1] * step);
}

// ---------- inspector ----------

function num(name, prop, spec, label) {
  return el(
    "label.field",
    {},
    el("span", {}, label),
    el("input", {
      type: "number",
      value: spec[prop] ?? "",
      step: 1,
      disabled: app.readonly,
      onchange: (e) => setProp(name, prop, e.target.value === "" ? null : Number(e.target.value)),
    }),
  );
}

const FALLBACK = { bold: "false", italic: "false", align: "left", valign: "center" };

function choice(name, prop, spec, label, options) {
  const cur = spec[prop] == null ? FALLBACK[prop] ?? "" : String(spec[prop]);
  return el(
    "label.field",
    {},
    el("span", {}, label),
    el(
      "select",
      {
        disabled: app.readonly,
        onchange: (e) => {
          const v = e.target.value;
          setProp(name, prop, v === "" ? null : v === "true" ? true : v === "false" ? false : v);
        },
      },
      options.map(([v, l]) => el("option", { value: v, selected: v === cur }, l)),
    ),
  );
}

function drawInspector(T) {
  const type = slideType(lt.key);
  const names = Object.keys(T);
  if (!T[lt.selected]) lt.selected = names[0];
  const name = lt.selected;
  const spec = T[name];
  const def = type.elements[name];
  const custom = lt.templates[lt.key]?.elements?.[name] ?? {};
  const isImage = spec.kind === "image";
  const isFont = spec.kind === "font";

  const fontKnown = FONTS.some(([v]) => v === (spec.font ?? ""));
  const fontSel = el(
    "select",
    {
      disabled: app.readonly,
      onchange: (e) => {
        if (e.target.value === "__custom") {
          const f = prompt("CSS font family (e.g. “Gill Sans”, sans-serif):", spec.font ?? "");
          if (f != null) setProp(name, "font", f.trim() || null);
          else draw();
        } else setProp(name, "font", e.target.value || null);
      },
    },
    FONTS.map(([v, l]) => el("option", { value: v, selected: v === (custom.font ?? "") || (v === spec.font && !custom.font && v !== "") }, l)),
    !fontKnown ? el("option", { value: spec.font, selected: true }, spec.font) : null,
    el("option", { value: "__custom" }, "Other…"),
  );
  const color = el("input", {
    type: "color",
    value: /^#[0-9a-f]{6}$/i.test(spec.color ?? "") ? spec.color : "#ffffff",
    disabled: app.readonly,
    oninput: debounce((e) => setProp(name, "color", e.target.value), 150),
  });

  inspector.replaceChildren(...[
    el(
      "label.field",
      {},
      el("span", {}, "Element"),
      el(
        "select",
        { onchange: (e) => { lt.selected = e.target.value; draw(); } },
        names.map((n) => el("option", { value: n, selected: n === name }, T[n].label)),
      ),
    ),
    isFont ? el("p.muted.small", {}, "Type style only: these sit inside the text box.") : null,
    isFont
      ? null
      : el(
          "div.lyt-grid",
          {},
          num(name, "x", spec, "X"),
          num(name, "y", spec, "Y"),
          num(name, "w", spec, "Width"),
          num(name, "h", spec, "Height"),
        ),
    isImage
      ? el("p.muted.small", {}, name === "music" && type.key === "stream.bottom_bar"
          ? "The image is chosen per psalm: Editor → Psalm layout → Musical setting."
          : "The image comes from the item: the Antiphon’s ♪ Music block.")
      : el(
          "div",
          {},
          el("label.field", {}, el("span", {}, "Font"), fontSel),
          el(
            "div.lyt-grid",
            {},
            num(name, "size", spec, "Size (px)"),
            el(
              "label.field",
              {},
              el("span", {}, "Colour"),
              el(
                "div.lyt-color",
                {},
                color,
                el("button.small", { disabled: app.readonly || custom.color == null, title: def.color ? `Default ${def.color}` : "Follow the background’s text tone", onclick: () => setProp(name, "color", null) }, def.color ? "Default" : "Auto"),
              ),
            ),
            choice(name, "bold", spec, "Bold", [["true", "Bold"], ["false", "Regular"]]),
            choice(name, "italic", spec, "Italic", [["true", "Italic"], ["false", "Upright"]]),
            isFont ? null : choice(name, "align", spec, "Align", [["left", "Left"], ["center", "Centre"], ["right", "Right"]]),
            isFont ? null : choice(name, "valign", spec, "Vertical", [["top", "Top"], ["center", "Middle"], ["bottom", "Bottom"]]),
          ),
        ),
    name === "people"
      ? el(
          "label.toggle",
          {},
          el("input", { type: "checkbox", checked: !spec.hidden, disabled: app.readonly, onchange: (e) => setPeopleLabel(e.target.checked) }),
          "Show the “People” label on every slide type (bold already marks the people's parts)",
        )
      : null,
    isFont
      ? null
      : el(
          "label.toggle",
          {},
          el("input", { type: "checkbox", checked: !spec.hidden, disabled: app.readonly, onchange: (e) => setProp(name, "hidden", !e.target.checked) }),
          "Show this element",
        ),
    textOptions(type, T),
    el(
      "div.lyt-actions",
      {},
      el("button.small", { disabled: app.readonly || !Object.keys(custom).length, onclick: () => resetElement(name) }, "Reset element"),
      el("button.small", { disabled: app.readonly || !lt.templates[lt.key], onclick: resetType }, `Reset ${type.label}`),
    ),
    el(
      "p.muted.small",
      {},
      "Positions are on the 1920 × 1080 screen. Text shrinks below its size when it would not fit its box. The text box grows upward into a title’s space on slides without a title.",
    ),
  ].filter(Boolean));
}

/** A slide type's own settings: how many lines a slide holds and how lines
 *  are laid out (the slides are re-made to fit). */
function setTypeOption(prop, value) {
  const t = (lt.templates[lt.key] ??= { elements: {} });
  if (value == null || value === "") delete t[prop];
  else t[prop] = value;
  save();
  draw();
}

function textOptions(type, T) {
  if (!T.text) return null;
  const t = lt.templates[lt.key] ?? {};
  const o = typeOptions(lt.templates, lt.key);
  const stream = type.output === "stream";
  // What the text box holds at its size (line height 1.25), as in fit.rs.
  const holds = Math.max(1, Math.round(T.text.h / (T.text.size * 1.25)));
  const lines = el("input", {
    type: "number",
    min: 1,
    max: 30,
    step: 1,
    value: t.max_lines ?? "",
    placeholder: stream ? `${holds} (fills the box)` : "As written",
    disabled: app.readonly,
    onchange: (e) => setTypeOption("max_lines", Number(e.target.value) > 0 ? Math.round(Number(e.target.value)) : null),
  });
  const breaks = el(
    "select",
    { disabled: app.readonly, onchange: (e) => setTypeOption("line_breaks", e.target.value || null) },
    [["", `Default (${o.join && !t.line_breaks ? "run together" : "each line"})`], ["keep", "Each line on its own line"], ["join", "Run a paragraph's lines together"]].map(([v, l]) =>
      el("option", { value: v, selected: v === (t.line_breaks ?? "") }, l),
    ),
  );
  const check = (prop, value, label, title) =>
    el(
      "label.toggle",
      { title },
      el("input", { type: "checkbox", checked: value, disabled: app.readonly, onchange: (e) => setTypeOption(prop, e.target.checked) }),
      label,
    );
  return el(
    "div.lyt-type-opts",
    {},
    el("h4", {}, `Text on ${type.label} slides`),
    el("label.field", {}, el("span", {}, "Lines per slide"), lines),
    el(
      "p.muted.small",
      {},
      stream
        ? "The live stream runs written slides together while they fit and splits longer ones, at the end of a line or the nearest punctuation, keeping a response with the words it answers."
        : "Empty: the slides stay as written. A number splits longer slides the same way the live stream does.",
    ),
    el("label.field", {}, el("span", {}, "Line breaks"), breaks),
    check("hanging", o.hanging, "Hanging indent for long lines", "A line too long for the box carries on indented under itself, as in the Prayer Book (the Creed, the Lord's Prayer, psalm verses)"),
    type.sample === "psalm" ? check("alternate_verses", o.alternate, "Indent every other psalm verse", "Even-numbered verses are indented a step") : null,
  );
}

function resetElement(name) {
  const t = lt.templates[lt.key];
  if (t?.elements) delete t.elements[name];
  save();
  draw();
}

async function resetType() {
  const ok = await call("confirm", { title: "Reset layout", message: `Put every piece of “${slideType(lt.key).label}” back where it started?`, okLabel: "Reset" });
  if (!ok) return;
  const saved = await call("layouts_reset", { key: lt.key });
  if (saved) lt.templates = saved;
  toast("Layout reset");
  build();
}
