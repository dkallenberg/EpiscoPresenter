// The Services screen: upcoming scheduled services, scheduling ahead, and
// opening services. Shown when the app starts and from "☰ Services".

import { invoke } from "/js/bridge.js";
import { app, call, el, reportWarnings, toast } from "/js/control/util.js";

const SWATCH = {
  Green: "#2e7d32",
  Violet: "#5e2a84",
  White: "#f5f1e6",
  Red: "#b71c1c",
  Blue: "#1f4e9c",
  Black: "#111111",
};

let services = [];
let templates = [];
const SEASONS = [
  ["Advent", "Advent"],
  ["Christmas", "Christmas"],
  ["Epiphany", "Epiphany"],
  ["Lent", "Lent"],
  ["HolyWeek", "Holy Week"],
  ["Easter", "Easter"],
  ["Pentecost", "After Pentecost"],
];
const seasonLabel = (k) => SEASONS.find(([v]) => v === k)?.[1] ?? k;
let shownAtStart = false;

const pad = (n) => String(n).padStart(2, "0");
/** Today as YYYY-MM-DD in local time. */
export function today() {
  const d = new Date();
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function addDays(iso, n) {
  const d = new Date(`${iso}T12:00:00`);
  d.setDate(d.getDate() + n);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function nextSundayWithout(dates) {
  const d = new Date(`${today()}T12:00:00`);
  let iso = addDays(today(), (7 - d.getDay()) % 7);
  while (dates.has(iso)) iso = addDays(iso, 7);
  return iso;
}

function formatDate(iso, opts) {
  const d = new Date(`${iso}T12:00:00`);
  return isNaN(d) ? iso : d.toLocaleDateString(undefined, opts);
}

export function initHome() {
  app.on((what) => {
    if (what === "link") return;
    // On start (and if the open service is closed), show the Services screen.
    if (!app.service && !app.readonly && (!shownAtStart || what === "service")) {
      shownAtStart = true;
      showHome();
    }
    if (app.service) shownAtStart = true;
  });
}

export async function showHome() {
  const home = document.getElementById("home");
  home.hidden = false;
  home.replaceChildren(el("p.muted.home-loading", {}, "Loading services…"));
  [services, templates] = await Promise.all([call("services_list"), call("templates_list")]);
  services ??= [];
  templates ??= [];
  render();
}

export function hideHome() {
  document.getElementById("home").hidden = true;
}

/** "Start from": by season, the standard order, a template, or a copy. */
function templateSelect() {
  const copies = [...services].filter((s) => s.date).reverse().slice(0, 12);
  const seasonal = templates.some((t) => t.seasons.length);
  return el(
    "select",
    {},
    seasonal ? el("option", { value: "auto", selected: true }, "By season (your templates, else the standard order)") : null,
    el("option", { value: "" }, "Standard order (from the Library)"),
    templates.length
      ? el(
          "optgroup",
          { label: "Templates" },
          templates.map((t) => el("option", { value: t.path }, `${t.name}${t.seasons.length ? ` — ${t.seasons.map(seasonLabel).join(", ")}` : ""}`)),
        )
      : null,
    copies.length
      ? el(
          "optgroup",
          { label: "Copy of a service" },
          copies.map((s) => el("option", { value: s.path }, `${s.title} — ${formatDate(s.date, { month: "short", day: "numeric", year: "numeric" })}`)),
        )
      : null,
  );
}

/** The template and options to send for a "Start from" choice. */
function startFrom(select, opts) {
  const v = select.value;
  return { template: v && v !== "auto" ? v : null, options: { ...opts, by_season: v === "auto" } };
}

// ---------- templates ----------

/** Name and seasons for a template (new or existing). Resolves to {name, seasons} or null. */
function templateDialog(title, initial = { name: "", seasons: [] }) {
  return new Promise((resolve) => {
    const name = el("input", { value: initial.name, placeholder: "e.g. Lent, Rite I" });
    const boxes = SEASONS.map(([v, l]) => {
      const box = el("input", { type: "checkbox", value: v, checked: initial.seasons.includes(v) });
      return el("label.toggle", {}, box, l);
    });
    const done = (v) => {
      overlay.remove();
      resolve(v);
    };
    const ok = () => {
      if (!name.value.trim()) return toast("Name the template", "warn");
      done({ name: name.value.trim(), seasons: boxes.map((b) => b.querySelector("input")).filter((b) => b.checked).map((b) => b.value) });
    };
    const overlay = el(
      "div.modal-overlay",
      { onmousedown: (e) => e.target === overlay && done(null) },
      el(
        "div.modal",
        {},
        el("h3", {}, title),
        el("label.field", {}, el("span", {}, "Name"), name),
        el("div.field", {}, el("span", {}, "Use automatically for"), el("div.season-boxes", {}, boxes)),
        el(
          "p.muted.small",
          {},
          "New and scheduled services started “By season” use this template on Sundays in the ticked seasons. Leave all unticked to choose it by name only.",
        ),
        el("div.modal-actions", {}, el("button", { onclick: () => done(null) }, "Cancel"), el("button.primary", { onclick: ok }, "Save")),
      ),
    );
    document.body.append(overlay);
    name.focus();
    name.addEventListener("keydown", (e) => e.key === "Enter" && ok());
  });
}

/** Saves the open service's order (without its propers) as a template. */
export async function saveAsTemplate() {
  if (!app.service) return toast("Open a service first", "warn");
  templates = (await call("templates_list")) ?? templates;
  const v = await templateDialog("Save order as a template");
  if (!v) return;
  const existing = templates.find((t) => t.name.toLowerCase() === v.name.toLowerCase());
  if (existing) {
    const ok = await call("confirm", { title: "Replace template", message: `Replace the template “${existing.name}” with this order?`, okLabel: "Replace" });
    if (!ok) return;
  }
  const path = await call("template_save", v);
  if (path) {
    toast(`Saved the template “${v.name}”`);
    if (!document.getElementById("home").hidden) showHome();
  }
}

async function editTemplate(t) {
  const v = await templateDialog("Template", t);
  if (!v) return;
  if ((await call("template_update", { path: t.path, ...v })) !== undefined) showHome();
}

async function deleteTemplate(t) {
  const ok = await call("confirm", { title: "Delete template", message: `Delete the template “${t.name}”? Services made from it are not affected.`, okLabel: "Delete" });
  if (!ok) return;
  if ((await call("template_delete", { path: t.path })) !== undefined) showHome();
}

async function openTemplate(t) {
  if ((await call("template_open", { path: t.path })) !== undefined) {
    hideHome();
    toast(`Editing the template “${t.name}”. Changes save automatically.`);
  }
}

function templatesCard() {
  const rows = templates.map((t) =>
    el(
      "div.tpl-row",
      {},
      el(
        "div.tpl-main",
        {},
        el("strong", {}, t.name),
        el(
          "div.muted.small",
          {},
          [t.seasons.length ? t.seasons.map(seasonLabel).join(", ") : "Any season (choose by name)", t.rite === "I" ? "Rite I" : "Rite II", `${t.cues} items`].join(" · "),
        ),
      ),
      el("button.small", { onclick: () => openTemplate(t), title: "Open the template to change its order or texts" }, "Edit order"),
      el("button.small", { onclick: () => editTemplate(t), title: "Rename, or change its seasons" }, "Name & seasons"),
      el("button.small.danger-btn", { onclick: () => deleteTemplate(t) }, "Delete"),
    ),
  );
  return el(
    "div.home-card.tpl-card",
    {},
    el(
      "div.tpl-head",
      {},
      el("h3", {}, "Service order templates"),
      el("span.spacer"),
      el("button.small", { disabled: !app.service, title: app.service ? "Save the open service's order (without its readings) as a template" : "Open a service first", onclick: saveAsTemplate }, "Save open service as template…"),
    ),
    templates.length
      ? el("div.tpl-list", {}, rows)
      : el(
          "p.muted.small",
          {},
          "Arrange a service the way you like it for a season (hymn slots, prayers, rite), then save it here as a named template, e.g. “Lent” or “Easter, Rite I”. New services can start from it, or pick it automatically by season.",
        ),
  );
}

const remember = (k, v) => { try { localStorage.setItem(k, v); } catch {} };
const recall = (k, d) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };

/** Rite, track and Bible version for new services (remembered). */
function serviceOptions() {
  const sel = (key, def, opts, title) =>
    el(
      "select",
      { title, onchange: (e) => remember(key, e.target.value) },
      opts.map(([v, l]) => el("option", { value: v, selected: v === recall(key, def) }, l)),
    );
  const rite = sel("svc.rite", "II", [["II", "Rite II"], ["I", "Rite I"]], "Contemporary (Rite II) or traditional (Rite I) language");
  const track = sel("svc.track", "1", [["1", "Track 1"], ["2", "Track 2"]], "Season after Pentecost: which Old Testament track");
  const bible = sel("svc.bible", "NRSVue", [["NRSVue", "NRSVue"], ["NRSV", "NRSV"]], "Translation for reading texts you paste in");
  const field = (label, input) => el("label.field", {}, el("span", {}, label), input);
  return {
    row: el("div.home-row.three", {}, field("Rite", rite), field("Readings", track), field("Bible", bible)),
    value: () => ({ rite: rite.value, track: Number(track.value), bible: bible.value }),
  };
}

function render() {
  const home = document.getElementById("home");
  const now = today();
  const upcoming = services.filter((s) => !s.date || s.date >= now);
  const past = services.filter((s) => s.date && s.date < now).reverse();
  const dates = new Set(services.map((s) => s.date).filter(Boolean));

  // ---- New service ----
  const date = el("input", { type: "date", value: nextSundayWithout(dates) });
  const title = el("input", { placeholder: "Title" });
  const info = el("div.muted.small.day-info");
  let autoTitle = "";
  const updateDay = async () => {
    const day = date.value ? await invoke("liturgical_info", { date: date.value }).catch(() => null) : null;
    info.textContent = day ? `${day.season} · ${day.color} · Year ${day.year}${day.proper ? ` · Proper ${day.proper}` : ""}` : "";
    // Keep a title the operator typed; otherwise follow the date.
    if (!title.value || title.value === autoTitle) title.value = autoTitle = day?.name ?? "";
  };
  date.addEventListener("change", updateDay);
  updateDay();
  const newTemplate = templateSelect();
  const newOpts = serviceOptions();
  const create = async () => {
    if (!date.value) return toast("Choose a date", "warn");
    const plan = await call("service_create", {
      date: date.value,
      title: title.value || null,
      ...startFrom(newTemplate, newOpts.value()),
    });
    if (plan) {
      hideHome();
      toast("Service created. It saves automatically as you work.");
    }
  };

  // ---- Schedule ----
  const weeks = el("select", {}, [2, 3, 4, 5, 6, 8].map((n) => el("option", { value: n, selected: n === 4 }, `${n} Sundays`)));
  const schedTemplate = templateSelect();
  const schedOpts = serviceOptions();
  const schedule = async () => {
    const made = await call("services_schedule", {
      from: now,
      weeks: Number(weeks.value),
      ...startFrom(schedTemplate, schedOpts.value()),
    });
    if (made === undefined) return;
    toast(made.length ? `Scheduled ${made.length} service${made.length === 1 ? "" : "s"}` : "Those Sundays already have services");
    showHome();
  };

  const field = (label, input) => el("label.field", {}, el("span", {}, label), input);
  home.replaceChildren(
    el(
      "div.home-inner",
      {},
      el(
        "div.home-head",
        {},
        el("h1", {}, "Services"),
        el("span.spacer"),
        el("button", { onclick: openFile, title: "Open a service plan (.toml) from anywhere" }, "Open file…"),
        el("button", { onclick: openExample, title: "The built-in example service" }, "Example"),
        app.service ? el("button", { onclick: hideHome, title: "Back to the open service" }, "✕ Close") : null,
      ),
      el(
        "div.home-cards",
        {},
        el(
          "div.home-card",
          {},
          el("h3", {}, "New service"),
          el("div.home-row", {}, field("Date", date), field("Title", title)),
          info,
          field("Start from", newTemplate),
          newOpts.row,
          el("div.home-card-actions", {}, el("button.primary", { onclick: create }, "Create & open")),
        ),
        el(
          "div.home-card",
          {},
          el("h3", {}, "Schedule upcoming Sundays"),
          el(
            "p.muted.small",
            {},
            "Creates a service for each coming Sunday that does not have one yet, named for the day in the church year, with its Collect, readings and psalm from the lectionary.",
          ),
          el("div.home-row", {}, field("How far ahead", weeks), field("Start from", schedTemplate)),
          schedOpts.row,
          el("div.home-card-actions", {}, el("button.primary", { onclick: schedule }, "Schedule")),
        ),
      ),
      templatesCard(),
      el("h2", {}, "Upcoming"),
      upcoming.length
        ? el("div.svc-list", {}, upcoming.map(row))
        : el("p.muted", {}, "No upcoming services yet. Use “Schedule upcoming Sundays” to plan about a month ahead."),
      past.length
        ? el("details.past", {}, el("summary", {}, `Past services (${past.length})`), el("div.svc-list", {}, past.map(row)))
        : null,
    ),
  );
}

function row(s) {
  const current = app.service?.plan_path === s.path;
  const day = s.day;
  return el(
    "div.svc-row" + (current ? ".current" : ""),
    { onclick: () => open(s), title: s.path },
    el(
      "div.svc-date",
      {},
      s.date
        ? [el("span.svc-dow", {}, formatDate(s.date, { weekday: "short" })), el("span.svc-day", {}, formatDate(s.date, { month: "short", day: "numeric" })), el("span.svc-year", {}, formatDate(s.date, { year: "numeric" }))]
        : el("span.muted", {}, "No date"),
    ),
    el("span.svc-swatch", { style: { background: SWATCH[day?.color] ?? "#555" }, title: day?.color ?? "" }),
    el(
      "div.svc-body",
      {},
      el("div.svc-title", {}, s.title),
      el(
        "div.svc-meta",
        {},
        [day && day.name !== s.title ? day.name : null, day ? `Year ${day.year}${day.proper ? ` · Proper ${day.proper}` : ""}` : null, `${s.cues} items`]
          .filter(Boolean)
          .join(" · "),
      ),
    ),
    el(
      "div.svc-badges",
      {},
      current ? el("span.badge.live-open", {}, "Open") : null,
      s.needs_propers ? el("span.badge.needs", { title: "No readings yet: open it and use Library → Propers…" }, "Readings needed") : null,
    ),
    el(
      "button.icon.danger",
      {
        title: "Delete this service",
        onclick: (e) => {
          e.stopPropagation();
          remove(s);
        },
      },
      "✕",
    ),
  );
}

async function open(s) {
  if (app.service?.plan_path === s.path) return hideHome();
  const r = await call("open_service", { path: s.path });
  if (r?.opened) {
    reportWarnings(r.warnings);
    hideHome();
  }
}

async function openFile() {
  const r = await call("open_service", { path: null });
  if (r?.opened) {
    reportWarnings(r.warnings);
    hideHome();
  }
}

async function openExample() {
  const w = await call("load_example_service");
  if (w) {
    reportWarnings(w);
    hideHome();
  }
}

async function remove(s) {
  const ok = await call("confirm", {
    title: "Delete service",
    message: `Delete “${s.title}”${s.date ? ` (${formatDate(s.date, { dateStyle: "medium" })})` : ""} and its texts? This cannot be undone.`,
    okLabel: "Delete",
  });
  if (!ok) return;
  if ((await call("service_delete", { path: s.path })) !== undefined) showHome();
}
