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
const FIT_LABELS = { fill: "Fill (crop)", fit: "Fit (letterbox)", stretch: "Stretch" };

// ---------- layering (mirrors Look::layered / ResolvedLook::complete) ----------

const clean = (look) => Object.fromEntries(Object.entries(look ?? {}).filter(([, v]) => v != null && v !== ""));

export function layered(base, over) {
  const changed = over.layout != null && over.layout !== base.layout;
  return clean({
    background: over.background ?? (changed ? undefined : base.background),
    layout: over.layout ?? base.layout,
    tone: over.tone ?? (changed ? undefined : base.tone),
    fit: over.fit ?? base.fit,
  });
}

export function complete(look) {
  const layout = look.layout ?? "centered";
  return {
    background: look.background ?? LAYOUT_BACKGROUND[layout],
    layout,
    tone: look.tone ?? (layout === "centered" ? "dark" : "light"),
    fit: look.fit ?? "fill",
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

/** Two cards (sanctuary, stream) for the selected cue. */
let panelOpen = true;

export function lookPanel(cue) {
  const panel = el("div.look-panel", {}, OUTPUTS.map(([output, label]) => lookCard(cue, output, label)));
  return el(
    "details.section",
    { open: panelOpen, ontoggle: (e) => (panelOpen = e.target.open) },
    el("summary", {}, "Backgrounds & layout", el("span.muted.small", {}, ` · ${titleCase(cue.category)}`)),
    panel,
  );
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
  return el(
    "div.look-card",
    {},
    el(
      "div.look-head",
      {},
      el("strong", {}, label),
      anyOwn ? el("span.badge.own", { title: "This item has its own look" }, "custom") : el("span.muted.small", {}, `${typeName} default`),
    ),
    preview,
    el("label.field", {}, el("span", {}, "Background"), bgButton),
    el("div.look-row", {},
      el("label.field", {}, el("span", {}, "Layout"), select("layout", LAYOUT_LABELS)),
      el("label.field", {}, el("span", {}, "Text"), select("tone", TONE_LABELS)),
      el("label.field", {}, el("span", {}, "Fit"), select("fit", FIT_LABELS)),
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
    deck: el("input", { value: s.announcements_deck ?? "", placeholder: "https://docs.google.com/presentation/d/e/…/pub" }),
    key: el("select", {}, KEYS.map(([k, l]) => el("option", { value: k, selected: k === key }, l)), el("option", { value: "custom", selected: !known }, "Custom…")),
    customKey: el("input", { value: known ? "" : key, placeholder: "#rrggbb", hidden: known }),
    rite: el("select", {}, [["II", "Rite II (contemporary)"], ["I", "Rite I (traditional)"]].map(([v, l]) => el("option", { value: v, selected: (s.rite ?? "II") === v }, l))),
    track: el("select", {}, [[1, "Track 1"], [2, "Track 2"]].map(([v, l]) => el("option", { value: v, selected: (s.track ?? 1) === v }, l))),
    bible: el("select", {}, [["NRSVue", "NRSVue (Updated Edition)"], ["NRSV", "NRSV"]].map(([v, l]) => el("option", { value: v, selected: (s.bible ?? "NRSVue") === v }, l))),
  };
  f.key.addEventListener("change", () => (f.customKey.hidden = f.key.value !== "custom"));

  const close = () => overlay.remove();
  const save = async () => {
    const stream_key = f.key.value === "custom" ? f.customKey.value.trim() : f.key.value;
    const ok = await call("set_service_details", {
      details: {
        title: f.title.value,
        date: f.date.value || null,
        season: f.season.value || null,
        color: f.color.value || null,
        announcements_deck: f.deck.value || null,
        stream_key: stream_key || null,
        rite: f.rite.value,
        track: Number(f.track.value),
        bible: f.bible.value,
      },
    });
    if (ok !== undefined) close();
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
      row("Announcements deck (published Google Slides URL)", f.deck),
      el("div.look-row", {}, row("Stream key colour (lower thirds)", f.key), row("Custom colour", f.customKey)),
      el("div.modal-actions", {}, el("button", { onclick: close }, "Cancel"), el("button.primary", { onclick: save }, "Save")),
    ),
  );
  document.body.append(overlay);
  f.title.focus();
}

function seasonColor(season) {
  return { Advent: "Blue", Christmas: "White", Epiphany: "Green", Lent: "Violet", HolyWeek: "Red", Easter: "White", Pentecost: "Green" }[season];
}
