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
  services = (await call("services_list")) ?? [];
  render();
}

export function hideHome() {
  document.getElementById("home").hidden = true;
}

function templateSelect() {
  const copies = [...services].filter((s) => s.date).reverse().slice(0, 12);
  return el(
    "select",
    {},
    el("option", { value: "" }, "Standard Rite II order (from the Library)"),
    copies.map((s) => el("option", { value: s.path }, `Copy of ${s.title} — ${formatDate(s.date, { month: "short", day: "numeric", year: "numeric" })}`)),
  );
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
  const create = async () => {
    if (!date.value) return toast("Choose a date", "warn");
    const plan = await call("service_create", { date: date.value, title: title.value || null, template: newTemplate.value || null });
    if (plan) {
      hideHome();
      toast("Service created. It saves automatically as you work.");
    }
  };

  // ---- Schedule ----
  const weeks = el("select", {}, [2, 3, 4, 5, 6, 8].map((n) => el("option", { value: n, selected: n === 4 }, `${n} Sundays`)));
  const schedTemplate = templateSelect();
  const schedule = async () => {
    const made = await call("services_schedule", { from: now, weeks: Number(weeks.value), template: schedTemplate.value || null });
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
          el("div.home-card-actions", {}, el("button.primary", { onclick: create }, "Create & open")),
        ),
        el(
          "div.home-card",
          {},
          el("h3", {}, "Schedule upcoming Sundays"),
          el(
            "p.muted.small",
            {},
            "Creates a service for each coming Sunday that does not have one yet, named for the day in the church year. Readings and the collect are added later with Import propers.",
          ),
          el("div.home-row", {}, field("How far ahead", weeks), field("Start from", schedTemplate)),
          el("div.home-card-actions", {}, el("button.primary", { onclick: schedule }, "Schedule")),
        ),
      ),
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
      s.needs_propers ? el("span.badge.needs", { title: "No readings yet: open it and use + Add → Import propers" }, "Readings needed") : null,
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
