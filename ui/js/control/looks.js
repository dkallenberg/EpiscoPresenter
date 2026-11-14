// Per-cue backgrounds and layouts ("looks") for each output, the background
// picker, and the service settings panel.
//
// Layering mirrors episco_core::look: built-in default for the item type <
// service default for the type < the .ewm file's background (sanctuary) <
// the cue's own override. The editor edits the cue override; "Use for all"
// promotes it to the service default.

import { activeSlide } from "/js/bridge.js";
import { BUILTIN_LABELS } from "/js/backgrounds.js";
import { backgroundCss, mountStage, renderOutput } from "/js/render.js";
import { saveAsTemplate } from "/js/control/home.js";
import { parseDeck } from "/js/deck.js";
import { app, call, el, titleCase, toast } from "/js/control/util.js";

export const OUTPUTS = [
  ["sanctuary", "Sanctuary"],
  ["stream", "Live Stream"],
];

const LAYOUT_LABELS = {
  centered: "Centered (glass frame)",
  lyrics: "Lyrics on black",
  left_panel: "Left panel",
  left_panel_wide: "Left panel, wide (hymns)",
  bottom_bar: "Bottom bar",
  lower_third: "Lower third (keyed)",
  info_bar: "Info bar (welcome window)",
};
const LAYOUT_BACKGROUND = {
  centered: "builtin:glass-light",
  lyrics: "#000000",
  left_panel: "builtin:panel-dark",
  left_panel_wide: "builtin:panel-light",
  bottom_bar: "builtin:bar-bottom",
  lower_third: "key",
  info_bar: "builtin:window",
};
const TONE_LABELS = { dark: "Dark text", light: "Light text" };

/** The usual looks, by name: one choice instead of background, layout,
 *  text colour and fit (those stay under More…). */
const PRESETS = {
  sanctuary: [
    ["glass-light", "Stained glass, light (dark text)", { layout: "centered", background: "builtin:glass-light", tone: "dark" }],
    ["glass-colour", "Stained glass, colour (light text)", { layout: "centered", background: "builtin:glass-dark", tone: "light" }],
    ["glass-charcoal", "Stained glass, charcoal (light text)", { layout: "centered", background: "builtin:glass-charcoal", tone: "light" }],
    ["lyrics", "Black, as hymn lyrics", { layout: "lyrics", background: "#000000", tone: "light" }],
    ["blank", "Blank: a black screen", { blank: true }],
  ],
  stream: [
    ["left", "Text on the left, camera on the right", { layout: "left_panel", background: "builtin:panel-dark", tone: "light" }],
    ["left-wide", "Wide text on the left (hymns)", { layout: "left_panel_wide", background: "builtin:panel-light", tone: "dark" }],
    ["bar", "Bar across the bottom", { layout: "bottom_bar", background: "builtin:bar-bottom", tone: "light" }],
    ["lower-third", "Lower third over the camera", { layout: "lower_third", background: "key", tone: "light" }],
    ["full", "Full screen, as the sanctuary", { layout: "centered", background: "builtin:glass-dark", tone: "light" }],
    ["camera", "Camera only: nothing shown", { blank: true }],
  ],
};

/** The preset an item's own look is, "" for none (the usual), or "custom". */
function presetOf(output, own) {
  if (!Object.keys(own).length) return "";
  const same = (a, b) => Object.keys(a).length === Object.keys(b).length && Object.entries(a).every(([k, v]) => b[k] === v);
  return PRESETS[output].find(([, , look]) => same(look, own))?.[0] ?? "custom";
}
const FIT_LABELS = { fill: "Fill the screen (edges trimmed)", fit: "Whole picture (bars at the sides)", stretch: "Stretch to the screen" };

// ---------- layering (mirrors Look::layered / ResolvedLook::complete) ----------

const clean = (look) => Object.fromEntries(Object.entries(look ?? {}).filter(([, v]) => v != null && v !== ""));

export function layered(base, over) {
  const changed = over.layout != null && over.layout !== base.layout;
  return clean({
    background: over.background ?? (changed ? undefined : base.background),
    layout: over.layout ?? base.layout,
    tone: over.tone ?? (changed ? undefined : base.tone),
    fit: over.fit ?? base.fit,
    blank: over.blank ?? base.blank,
  });
}

export function complete(look) {
  const layout = look.layout ?? "centered";
  return {
    background: look.background ?? LAYOUT_BACKGROUND[layout],
    layout,
    tone: look.tone ?? (layout === "centered" ? "dark" : "light"),
    fit: look.fit ?? "fill",
    blank: look.blank ?? false,
  };
}

/** What the cue would look like on `output` without its own override. */
function inherited(cue, output) {
  const builtin = app.vocab.categories.find((c) => c.key === cue.category)?.defaults[output] ?? {};
  let look = layered(clean(builtin), clean(app.service?.looks?.[cue.category]?.[output]));
  const fileBg = cue.content.document?.meta.default_background;
  if (output === "sanctuary" && fileBg) look = layered(look, { background: fileBg });
  return complete(look);
}

const override = (cue, output) => clean(cue.overrides?.looks?.[output]);

export function backgroundName(value) {
  if (!value) return "—";
  return BUILTIN_LABELS[value] ?? value.split(/[\\/]/).pop().replace(/\.[a-z]+$/i, "");
}

async function setCueLook(cue, output, look) {
  await call("set_cue_look", { id: cue.id, output, look: clean(look) });
}

// ---------- the editor panel ----------

/** Two cards (sanctuary, stream) for the selected cue, in a section that
 *  starts closed (the words come first) and stays as the operator left it. */
let panelOpen = false;

/** The cards inside the Slides section (to refresh it in place). */
export function lookCards(cue) {
  return OUTPUTS.map(([output, label]) => lookCard(cue, output, label));
}

export function lookPanel(cue) {
  // Closed, the cards (and their slide previews) are only made when opened.
  const open = panelOpen;
  const panel = el("div.look-panel", {}, open ? OUTPUTS.map(([output, label]) => lookCard(cue, output, label)) : []);
  return el(
    "details.section.slides-card",
    {
      open,
      dataset: { cue: cue.id },
      ontoggle: (e) => {
        panelOpen = e.target.open;
        // The item as it is now (it may have changed since this was made).
        const now = app.service?.cues.find((x) => x.id === cue.id) ?? cue;
        if (panelOpen && !panel.children.length) panel.replaceChildren(...lookCards(now));
      },
    },
    el(
      "summary",
      {},
      "Appearance",
      // A glance at both screens' backgrounds while closed.
      OUTPUTS.map(([output, label]) =>
        el("span.look-chip", { title: `${label}: ${lookName(cue, output)}`, style: { background: backgroundCss(app.service, cue, cue[output].background, "fill") } }),
      ),
      el("span.muted.small", {}, ` Sanctuary: ${lookName(cue, "sanctuary")} · Live stream: ${lookName(cue, "stream")}`),
    ),
    panel,
  );
}

/** Each card's More… stays open once opened (the cards redraw on changes). */
const moreOpen = { sanctuary: false, stream: false };

/** The name of the look an item has on a screen. */
function lookName(cue, output) {
  const own = override(cue, output);
  const p = presetOf(output, own);
  if (p === "") return "as usual";
  return PRESETS[output].find(([k]) => k === p)?.[1].replace(/ \(.*\)$/, "").replace(/:.*$/, "") ?? "custom";
}

function lookCard(cue, output, label) {
  const own = override(cue, output);
  const inh = inherited(cue, output);
  const eff = cue[output];
  // What "Auto" means for a field: the value it resolves to given this cue's
  // other settings (e.g. choosing Lower third makes the auto background the
  // key colour, not the type's bottom-bar artwork).
  const auto = (field) => complete(layered(inh, clean({ ...own, [field]: undefined })))[field];
  const ro = app.readonly;

  const preview = el("div.look-preview");
  queueMicrotask(() => {
    const stage = mountStage(preview);
    const live = app.state.active;
    const ci = app.service.cues.findIndex((c) => c.id === cue.id);
    const slide = live?.cue === ci ? live.slide : 0;
    const snap = { ...app.snap, state: { revision: 0 } };
    renderOutput(stage, snap, activeSlide({ ...snap, state: { active: { cue: ci, slide } } }), output);
  });

  const set = (field, value) => setCueLook(cue, output, { ...own, [field]: value || undefined });
  const select = (field, labels) =>
    el(
      "select",
      { disabled: ro, onchange: (e) => set(field, e.target.value), className: own[field] ? "overridden" : "" },
      el("option", { value: "", selected: !own[field] }, `Auto · ${labels[auto(field)] ?? auto(field)}`),
      Object.entries(labels).map(([v, l]) => el("option", { value: v, selected: own[field] === v }, l)),
    );

  const bgButton = el(
    "button.bg-button" + (own.background ? ".overridden" : ""),
    { disabled: ro, title: eff.background, onclick: (e) => openPicker(e.currentTarget, cue, output, own, auto("background")) },
    el("span.bg-thumb", { style: { background: backgroundCss(app.service, cue, eff.background, "fill") } }),
    el("span.bg-name", {}, own.background ? backgroundName(eff.background) : `Auto · ${backgroundName(auto("background"))}`),
    el("span.muted", {}, "▾"),
  );

  const anyOwn = Object.keys(own).length > 0;
  const typeName = titleCase(cue.category);
  const current = presetOf(output, own);
  const look = el(
    "select.look-select",
    {
      disabled: ro,
      onchange: (e) => {
        const v = e.target.value;
        if (v === "custom") return;
        const preset = PRESETS[output].find(([k]) => k === v)?.[2] ?? {};
        setCueLook(cue, output, { ...preset });
      },
    },
    el("option", { value: "", selected: current === "" }, `As usual for ${typeName} items`),
    PRESETS[output].map(([k, l]) => el("option", { value: k, selected: current === k }, l)),
    current === "custom" ? el("option", { value: "custom", selected: true }, "Your own (see More…)") : null,
  );
  return el(
    "div.look-card",
    {},
    el(
      "div.look-head",
      {},
      el("strong", {}, label),
      anyOwn ? el("span.badge.own", { title: "This item has its own look" }, "changed") : null,
    ),
    preview,
    el("label.field", {}, el("span", {}, "Look"), look),
    el(
      "details.look-more",
      { open: moreOpen[output] || current === "custom", ontoggle: (e) => (moreOpen[output] = e.target.open) },
      el("summary", {}, "More…"),
    el("label.field", {}, el("span", {}, "Background"), bgButton),
    el("div.look-row", {},
      el("label.field", {}, el("span", {}, "Where the text goes"), select("layout", LAYOUT_LABELS)),
      el("label.field", {}, el("span", {}, "Text colour"), select("tone", TONE_LABELS)),
      el("label.field", {}, el("span", {}, "Picture size"), select("fit", FIT_LABELS)),
    ),
    el(
      "label.toggle.blank-toggle",
      { title: output === "sanctuary" ? "Black screen for this item; the stream is not affected" : "Only the camera (key colour) for this item; the sanctuary is not affected" },
      el("input", {
        type: "checkbox",
        checked: Boolean(eff.blank),
        disabled: ro,
        onchange: (e) => set("blank", e.target.checked ? true : undefined),
      }),
      output === "sanctuary" ? "Blank on the sanctuary screen" : "Camera only on the stream (nothing shown)",
    ),
    ro
      ? null
      : el(
          "div.look-actions",
          {},
          el(
            "button.small",
            {
              disabled: !anyOwn,
              title: `Make this look the default for every ${typeName} item in the service`,
              onclick: () => promote(cue, output, own),
            },
            `Use for all ${typeName} items`,
          ),
          el("button.small", { disabled: !anyOwn, onclick: () => setCueLook(cue, output, {}) }, "Reset"),
        ),
    ),
  );
}

async function promote(cue, output, own) {
  const current = clean(app.service.looks?.[cue.category]?.[output]);
  await call("set_default_look", { category: cue.category, output, look: { ...current, ...own } });
  await setCueLook(cue, output, {});
  toast(`Now the ${output} default for all ${titleCase(cue.category)} items`);
}

// ---------- background picker ----------

let openPopover = null;
function closePicker() {
  openPopover?.remove();
  openPopover = null;
}
document.addEventListener("mousedown", (e) => {
  if (openPopover && !openPopover.contains(e.target) && !e.target.closest(".bg-button")) closePicker();
});
document.addEventListener("keydown", (e) => e.key === "Escape" && closePicker());

async function openPicker(anchor, cue, output, own, autoBackground) {
  closePicker();
  const choose = async (value) => {
    closePicker();
    await setCueLook(cue, output, { ...own, background: value || undefined });
  };
  const tile = (value, name, current) =>
    el(
      "button.bg-tile" + (current ? ".current" : ""),
      { title: value ?? "", onclick: () => choose(value) },
      el("span.bg-thumb", { style: { background: backgroundCss(app.service, cue, value ?? autoBackground, "fill") } }),
      el("span.bg-tile-name", {}, name),
    );

  const builtins = [...app.vocab.builtin_backgrounds, "key", "#000000"];
  const pop = el(
    "div.bg-picker",
    {},
    el("div.picker-title", {}, `${output === "sanctuary" ? "Sanctuary" : "Stream"} background · ${cue.title}`),
    el("div.bg-grid", {}, tile(null, `Auto · ${backgroundName(autoBackground)}`, !own.background)),
    el("div.picker-sub", {}, "Built-in (drawn in the liturgical colour)"),
    el("div.bg-grid", {}, builtins.map((b) => tile(b, BUILTIN_LABELS[b] ?? b, own.background === b))),
    el("div.picker-sub", {}, "Library"),
    el("div#bg-library.bg-grid", {}, el("span.muted.small", {}, "Loading…")),
    el(
      "div.picker-actions",
      {},
      el("button.small", { onclick: async () => { const v = await call("pick_background"); if (v) choose(v); } }, "Choose image file…"),
      el("button.small", {
        onclick: async () => {
          const added = await call("import_backgrounds");
          if (added?.length) choose(added[0].value);
        },
      }, "Import to library…"),
    ),
  );
  document.body.append(pop);
  openPopover = pop;
  const r = anchor.getBoundingClientRect();
  pop.style.left = `${Math.max(8, Math.min(r.left, innerWidth - pop.offsetWidth - 8))}px`;
  pop.style.top = `${Math.min(r.bottom + 4, innerHeight - pop.offsetHeight - 8)}px`;

  const files = (await call("list_backgrounds")) ?? [];
  const lib = pop.querySelector("#bg-library");
  if (!lib) return;
  lib.replaceChildren(
    ...(files.length
      ? files.map((f) => tile(f.value, f.name, own.background === f.value))
      : [el("span.muted.small", {}, "No images yet. Import your slide backgrounds (PNG/JPG) to reuse them across services.")]),
  );
}

// ---------- service settings ----------

const KEYS = [
  ["#0000ff", "Blue (#0000FF)"],
  ["#00ff00", "Green (#00FF00)"],
  ["transparent", "Transparent (alpha)"],
];

export function openServiceSettings() {
  const s = app.service;
  if (!s) return;
  const v = app.vocab;
  const key = s.stream_key ?? v.default_key_color;
  const known = KEYS.some(([k]) => k === key);
  const f = {
    title: el("input", { value: s.title ?? "" }),
    date: el("input", { type: "date", value: s.date ?? "" }),
    season: el("select", {}, el("option", { value: "" }, "—"), v.seasons.map((x) => el("option", { value: x.key, selected: s.season === x.key }, x.label))),
    color: el("select", {}, el("option", { value: "" }, "Auto (from season)"), v.colors.map((x) => el("option", { value: x.key, selected: s.color === x.key && s.color !== seasonColor(s.season) }, x.key))),
    deck: el("input", { value: s.announcements_deck ?? "", placeholder: "Paste the Google Slides link (Share → Copy link)" }),
    deckSlides: el("input", { type: "number", min: 0, step: 1, value: s.deck_slides ?? "", placeholder: "e.g. 8" }),
    key: el("select", {}, KEYS.map(([k, l]) => el("option", { value: k, selected: k === key }, l)), el("option", { value: "custom", selected: !known }, "Custom…")),
    customKey: el("input", { value: known ? "" : key, placeholder: "#rrggbb", hidden: known }),
    rite: el("select", {}, [["II", "Rite II (contemporary)"], ["I", "Rite I (traditional)"]].map(([v, l]) => el("option", { value: v, selected: (s.rite ?? "II") === v }, l))),
    track: el("select", {}, [[1, "Track 1"], [2, "Track 2"]].map(([v, l]) => el("option", { value: v, selected: (s.track ?? 1) === v }, l))),
    bible: el("select", {}, [["NRSVue", "NRSVue (Updated Edition)"], ["NRSV", "NRSV"]].map(([v, l]) => el("option", { value: v, selected: (s.bible ?? "NRSVue") === v }, l))),
    bishop: el("input", { type: "checkbox", checked: Boolean(s.bishop) }),
    deacon: el("input", { type: "checkbox", checked: !s.no_deacon }),
    baptismal: el(
      "select",
      {},
      [["", "Choose…"], ["baptism", "Holy Baptism"], ["renewal", "Renewal of Baptismal Vows"], ["confirmation", "Confirmation, Reception, Reaffirmation"]].map(([v, l]) => el("option", { value: v }, l)),
    ),
  };
  const addBaptismal = async () => {
    const v = f.baptismal.value;
    if (!v) return toast("Choose what to add.", "warn");
    const ok = await call("service_add_baptismal", {
      baptismal: v === "confirmation" ? null : v,
      confirmation: v === "confirmation",
    });
    if (ok !== undefined) {
      if (v === "confirmation") f.bishop.checked = true;
      toast(v === "baptism" ? "Holy Baptism added in place of the Creed" : v === "renewal" ? "Renewal of Baptismal Vows added in place of the Creed" : "Confirmation added, with the bishop presiding");
    }
  };
  f.key.addEventListener("change", () => (f.customKey.hidden = f.key.value !== "custom"));
  const deckNote = el("p.small.deck-note");
  const checkDeck = () => {
    const v = f.deck.value.trim();
    const d = parseDeck(v);
    deckNote.className = `small deck-note ${!v ? "muted" : d ? "ok" : "warn-note"}`;
    deckNote.textContent = !v
      ? "Plays before the service (sanctuary) and for Announcements items. Share the deck as “Anyone with the link can view”."
      : d
        ? `✓ Google Slides deck found. It must be shared as “Anyone with the link can view” (or File → Share → Publish to web) for the screens to show it.`
        : "This doesn't look like a Google Slides link (docs.google.com/presentation/…).";
  };
  f.deck.addEventListener("input", checkDeck);
  const refreshDeck = async () => {
    const url = f.deck.value.trim();
    if (!parseDeck(url)) return toast("Paste the Google Slides link first.", "warn");
    const file = await call("deck_fetch", { url, refresh: true });
    if (file) toast("Slides downloaded");
    else if (file === null) toast("This kind of link (Publish to web) can't be downloaded: use the deck's sharing link instead.", "warn");
  };
  checkDeck();

  const close = () => overlay.remove();
  const save = async () => {
    const stream_key = f.key.value === "custom" ? f.customKey.value.trim() : f.key.value;
    const ok = await call("set_service_details", {
      details: {
        title: f.title.value,
        date: f.date.value || null,
        season: f.season.value || null,
        color: f.color.value || null,
        announcements_deck: f.deck.value.trim() || null,
        deck_slides: Number(f.deckSlides.value) > 0 ? Number(f.deckSlides.value) : null,
        stream_key: stream_key || null,
        rite: f.rite.value,
        track: Number(f.track.value),
        bible: f.bible.value,
      },
    });
    if (ok === undefined) return;
    if (f.bishop.checked !== Boolean(app.service?.bishop)) {
      const added = await call("set_service_bishop", { bishop: f.bishop.checked });
      if (added) toast("The Bishop's Blessing was added before the dismissal");
    }
    if (f.deacon.checked === Boolean(app.service?.no_deacon)) await call("set_service_deacon", { deacon: f.deacon.checked });
    close();
  };
  const row = (label, input) => el("label.field", {}, el("span", {}, label), input);
  const overlay = el(
    "div.modal-overlay",
    { onmousedown: (e) => e.target === overlay && close() },
    el(
      "div.modal",
      {},
      el("h3", {}, "Service settings"),
      row("Title", f.title),
      el("div.look-row", {}, row("Date", f.date), row("Season", f.season), row("Colour", f.color)),
      el("div.look-row", {}, row("Rite", f.rite), row("Readings (after Pentecost)", f.track), row("Bible for reading texts", f.bible)),
      el("p.muted.small", {}, "The rite and track apply to texts added from now on (Library → Propers…, new items)."),
      el(
        "label.field.check",
        { title: "The Celebrant's and Priest's parts are labelled Bishop on the slides, bulletin and script" },
        f.bishop,
        el("span", {}, "A bishop presides (labels the Celebrant's parts “Bishop” and adds the Bishop's Blessing)"),
      ),
      el(
        "label.field.check",
        { title: "With no deacon, a priest says the Deacon's parts: they are labelled Priest on the slides, bulletin and script" },
        f.deacon,
        el("span", {}, "A deacon is serving (if not, the Deacon's parts are labelled “Priest”)"),
      ),
      el(
        "div.settings-rites",
        {},
        row("Add to this service", f.baptismal),
        el("button.small", { onclick: addBaptismal, title: "Added in place of the Creed, with the Prayer Book's changes to the order" }, "Add"),
      ),
      row("Announcements deck (Google Slides link)", f.deck),
      deckNote,
      el(
        "div.look-row",
        {},
        row("Slides in the deck", f.deckSlides),
        el("div.deck-refresh", {}, el("button.small", { onclick: refreshDeck, title: "Download the deck again after changing it in Google Slides" }, "↻ Refresh slides")),
        el("p.muted.small.deck-help", {}, "Found automatically when the deck downloads. A single-pass Announcements item steps through every slide; click any slide to show it."),
      ),
      el("div.look-row", {}, row("Stream key colour (lower thirds)", f.key), row("Custom colour", f.customKey)),
      el(
        "div.modal-actions",
        {},
        el("button", { title: "Save this service's order (without its readings) as a named template for new services", onclick: () => { close(); saveAsTemplate(); } }, "Save order as template…"),
        el("span.spacer"),
        el("button", { onclick: close }, "Cancel"),
        el("button.primary", { onclick: save }, "Save"),
      ),
    ),
  );
  document.body.append(overlay);
  f.title.focus();
}

function seasonColor(season) {
  return { Advent: "Blue", Christmas: "White", Epiphany: "Green", Lent: "Violet", HolyWeek: "Red", Easter: "White", Pentecost: "Green" }[season];
}
