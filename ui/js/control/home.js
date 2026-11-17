// The Services screen: upcoming scheduled services, scheduling ahead, and
// opening services. Shown when the app starts and from "☰ Services".

import { EUCHARISTIC_PRAYERS, PRAYERS_FORMS, parishTexts } from "/js/control/parish-texts.js";
import { invoke, listen } from "/js/bridge.js";
import { app, call, el, reportWarnings, toast } from "/js/control/util.js";
import { openParish } from "/js/control/bulletin.js";
import { PAPERS, USUAL_MUSIC, musicChecks, regularLabel, regularSummary } from "/js/control/regular.js";
import { openConnection } from "/js/control/connection.js";
import { openLibrarySettings, storageInfo } from "/js/control/storage.js";

const SWATCH = {
  Green: "#2e7d32",
  Violet: "#5e2a84",
  White: "#f5f1e6",
  Red: "#b71c1c",
  Blue: "#1f4e9c",
  Black: "#111111",
};

let services = [];
/** The parish's usual service times ("08:00", "10:30"), from Parish details. */
let parishServices = [];
let parishPaper = "tabloid";

/** "08:00" → "8:00 am". */
export function timeLabel(t) {
  const m = /^(\d{1,2}):(\d{2})$/.exec(t ?? "");
  if (!m) return t ?? "";
  const h = Number(m[1]);
  return `${((h + 11) % 12) + 1}:${m[2]} ${h < 12 ? "am" : "pm"}`;
}
/** EpiscoPresenter: services marked Ready in the parish library. */
let ready = [];
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
  // A different library folder: show its services, but never over a
  // service that is open (on the presentation computers above all); the
  // screen, if it is showing, is redrawn.
  listen("episco://storage", () => {
    const home = document.getElementById("home");
    if (!app.service || (home && !home.hidden)) showHome();
  });
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
  let parish;
  [services, templates, parish] = await Promise.all([call("services_list"), call("templates_list"), call("parish_get")]);
  parishInfo = parish ?? {};
  parishServices = parish?.services ?? [];
  parishPaper = parish?.paper ?? "tabloid";
  node = app.mode === "presenter" ? await call("node_info") : null;
  services ??= [];
  templates ??= [];
  render();
  // Services marked Ready in the planner: checking (and copying) them can
  // take a while from a shared folder, so the list is shown first.
  if (app.mode === "presenter") {
    ready = (await call("handoff_status")) ?? [];
    if (!home.hidden) render();
  }
}

let parishInfo = {};
let node = null;

const stepDone = (k) => {
  try {
    return localStorage.getItem(k) === "yes";
  } catch {
    return false;
  }
};
const markStep = (k) => {
  try {
    localStorage.setItem(k, "yes");
  } catch {}
};

/** The first things to set up, each with a button to do it, ticked when
 *  done; hidden once all are done or the operator hides it. */
function setupCard() {
  const presenter = app.mode === "presenter";
  const hiddenKey = `setup.hidden.${app.mode}`;
  const lib = storageInfo();
  const steps = [
    [
      "Choose the parish library folder",
      presenter ? "Where EpiscoPlanner puts the services (e.g. in Google Drive)." : "A shared folder (e.g. in Google Drive) for services, the library and pictures.",
      Boolean(lib?.folder) && !lib?.missing,
      () => openLibrarySettings(false),
    ],
    ...(presenter
      ? [
          [
            "Choose the screens",
            "Open the sanctuary screen (and the live stream) on their monitors once; the app remembers them.",
            stepDone("setup.screens"),
            () => document.getElementById("btn-outputs")?.click(),
          ],
          [
            "Connect the live-stream computer",
            "Only if a second computer runs the live stream.",
            (node?.config?.role ?? "standalone") !== "standalone" || stepDone("setup.one-computer"),
            () => openConnection(),
            ["One computer is enough", () => { markStep("setup.one-computer"); render(); }],
          ],
        ]
      : [
          [
            "Add the parish's details",
            "Name, logo, clergy and service times, for the bulletin cover and scheduling.",
            Boolean(parishInfo.name),
            () => openParish(),
          ],
          [
            "Plan the coming Sundays",
            "“Schedule services…” makes a service for each.",
            services.length > 0,
            () => openScheduler("sundays"),
          ],
        ]),
  ];
  const done = steps.filter((s) => s[2]).length;
  if (done === steps.length || stepDone(hiddenKey)) return null;
  return el(
    "div.home-card.setup-card",
    {},
    el(
      "div.setup-head",
      {},
      el("h3", {}, "Getting started"),
      el("span.muted.small", {}, `${done} of ${steps.length} done`),
      el("span.spacer"),
      el("button.small", { title: "Hide this list (the settings stay in ⚙ Settings)", onclick: () => { markStep(hiddenKey); render(); } }, "Hide"),
    ),
    el(
      "ol.setup-steps",
      {},
      steps.map(([title, help, ok, go, alt]) =>
        el(
          `li${ok ? ".done" : ""}`,
          {},
          el("span.setup-tick", {}, ok ? "✓" : ""),
          el("div.setup-text", {}, el("strong", {}, title), el("span.muted.small", {}, help)),
          ok ? null : el("button.small", { onclick: go }, "Do it…"),
          ok || !alt ? null : el("button.small.link", { onclick: alt[1] }, alt[0]),
        ),
      ),
    ),
  );
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
  // The Prayers of the People and the Eucharistic Prayer: the Prayer
  // Book's, and the parish's own from the library (each remembered for
  // each rite).
  const prayers = el("select", {
    title: "The Prayers of the People (BCP 383-393; Rite I, 328; or the parish's own from the library). Usual: the Rite I prayer in Rite I, Form I in Rite II",
    onchange: (e) => remember(`svc.prayers.${rite.value}`, e.target.value),
  });
  const ep = el("select", {
    title: "A whole Eucharistic Prayer in the service (in place of the opening dialogue; it has the Sanctus and the Proper Preface), or just the dialogue and the Sanctus",
    onchange: (e) => remember(`svc.ep.${rite.value}`, e.target.value),
  });
  let texts = { prayers: [], eucharistic: [] };
  const group = (labelText, list, chosen) =>
    list.length ? el("optgroup", { label: labelText }, list.map(([v, l]) => el("option", { value: v, selected: v === chosen }, l))) : null;
  const fillPrayers = () => {
    const forms = recall(`svc.prayers.${rite.value}`, recall("svc.prayers", ""));
    prayers.replaceChildren(
      el("option", { value: "", selected: !forms }, "Usual"),
      ...PRAYERS_FORMS.map(([v, l]) => el("option", { value: v, selected: v === forms }, l)),
      group("The parish's own", texts.prayers, forms) ?? "",
    );
    const chosen = recall(`svc.ep.${rite.value}`, "");
    ep.replaceChildren(
      el("option", { value: "", selected: !chosen }, "Dialogue and Sanctus only"),
      ...EUCHARISTIC_PRAYERS[rite.value].map(([v, l]) => el("option", { value: v, selected: v === chosen }, l)),
      group("The parish's own", texts.eucharistic, chosen) ?? "",
    );
  };
  const loadTexts = async () => {
    texts = await parishTexts(rite.value);
    fillPrayers();
  };
  rite.addEventListener("change", loadTexts);
  fillPrayers();
  loadTexts();
  const field = (label, input) => el("label.field", {}, el("span", {}, label), input);
  // Rite, Prayers and Eucharistic Prayer: hidden when each regular service
  // brings its own rite (the prayers are then those last chosen for it).
  const perRite = [field("Rite", rite), field("Prayers of the People", prayers), field("Eucharistic Prayer", ep)];
  const own = (v) => v.startsWith("user:");
  const choices = (r, p, e) => ({
    rite: r,
    track: Number(track.value),
    bible: bible.value,
    prayers: p && !own(p) ? p : null,
    prayers_item: own(p) ? p : null,
    eucharistic_prayer: e && !own(e) ? e : null,
    eucharistic_prayer_item: own(e) ? e : null,
  });
  return {
    row: el(
      "div",
      {},
      el("div.home-row.four", {}, perRite[0], field("Readings", track), field("Bible", bible), perRite[1]),
      el("div.home-row.four", {}, perRite[2]),
    ),
    value: () => choices(rite.value, prayers.value, ep.value),
    /** The choices for a service in rite `r` (its prayers as last chosen for that rite). */
    valueFor: (r) => choices(r, recall(`svc.prayers.${r}`, recall("svc.prayers", "")), recall(`svc.ep.${r}`, "")),
    /** Sets the rite (a regular service's). */
    setRite: (r) => {
      if (rite.value === r) return;
      rite.value = r;
      remember("svc.rite", r);
      loadTexts();
    },
    /** Each regular service brings its own rite: hide the rite's choices. */
    perService: (on) => {
      for (const f of perRite) f.hidden = on;
    },
  };
}

/**
 * The kind of service (Holy Week days and ordinations have their own Prayer
 * Book order) and the options: a bishop presiding, Holy Baptism or the
 * Renewal of Baptismal Vows, Confirmation.
 */
function occasionOptions(onKind) {
  let picked = false; // the operator chose a kind; stop following the date
  const kind = el("select", {
    title: "Holy Week services and ordinations use their own Prayer Book order",
    onchange: () => {
      picked = true;
      changed();
    },
  });
  const bishop = el("input", { type: "checkbox", onchange: () => changed() });
  // Remembered: most parishes either always or never have a deacon.
  let deaconUsual = true;
  try {
    deaconUsual = localStorage.getItem("service.deacon") !== "no";
  } catch {}
  const deacon = el("input", {
    type: "checkbox",
    checked: deaconUsual,
    title: "With no deacon, the Deacon's parts are labelled Priest",
    onchange: () => {
      try {
        localStorage.setItem("service.deacon", deacon.checked ? "yes" : "no");
      } catch {}
      changed();
    },
  });
  const baptismal = el(
    "select",
    { onchange: () => changed() },
    [["", "No baptism"], ["baptism", "Holy Baptism"], ["renewal", "Renewal of Baptismal Vows"]].map(([v, l]) => el("option", { value: v }, l)),
  );
  const confirmation = el("input", { type: "checkbox", onchange: () => { if (confirmation.checked) bishop.checked = true; changed(); } });
  const note = el("p.muted.small.occasion-note");
  const labels = new Map();
  const changed = () => {
    const k = kind.value || "eucharist";
    const ordination = k.startsWith("ordination");
    const event = k === "event";
    if (ordination) bishop.checked = true;
    bishop.disabled = ordination || event;
    deacon.disabled = event;
    const noBaptism = ["ash_wednesday", "good_friday", "holy_saturday", "palm_sunday", "maundy_thursday"].includes(k) || ordination || event;
    baptismal.disabled = confirmation.disabled = noBaptism;
    if (noBaptism) {
      baptismal.value = "";
      confirmation.checked = false;
    }
    note.textContent = event
      ? "Not a service: it starts with one blank item of slides. Make slides in the Editor, or add pictures, a PDF, Google Slides or a web video with + Add."
      : k === "easter_vigil"
        ? "The Vigil renews the baptismal vows unless there is a baptism."
        : ordination
          ? "The bishop presides: the ordination's own texts, the Litany for Ordinations and the Prayer Book's readings."
          : confirmation.checked
            ? "Confirmation, Reception and Reaffirmation, with the bishop."
            : bishop.checked
              ? "The Celebrant's parts are labelled Bishop, and the Bishop's Blessing is added before the dismissal."
              : "";
    if (!deacon.checked && !event) note.textContent = `${note.textContent} The Deacon's parts are labelled Priest.`.trim();
    onKind?.(k, labels.get(k));
  };
  const load = async (date) => {
    const list = await invoke("occasions_list", { date: date || null }).catch(() => null);
    if (!list) return;
    if (!kind.options.length) {
      for (const o of list.occasions) {
        labels.set(o.key, o.label);
        kind.append(el("option", { value: o.key }, o.key === "eucharist" ? "Holy Eucharist" : o.label));
      }
    }
    if (!picked) kind.value = list.suggested;
    changed();
  };
  const field = (label, input) => el("label.field", {}, el("span", {}, label), input);
  const check = (input, label) => el("label.field.inline.check", {}, input, el("span", {}, label));
  return {
    row: el(
      "div.occasion",
      {},
      el("div.home-row", {}, field("Service", kind), field("Baptism", baptismal)),
      el("div.home-row.checks", {}, check(bishop, "A bishop presides"), check(deacon, "A deacon is serving"), check(confirmation, "Confirmation, Reception, Reaffirmation")),
      note,
    ),
    load,
    deacon,
    kind: () => kind.value || "eucharist",
    value: () => ({
      occasion: kind.value || "eucharist",
      bishop: bishop.checked,
      baptismal: baptismal.value || null,
      confirmation: confirmation.checked,
      no_deacon: !deacon.checked,
    }),
  };
}

function render() {
  const home = document.getElementById("home");
  const now = today();
  const upcoming = services.filter((s) => !s.date || s.date >= now);
  const past = services.filter((s) => s.date && s.date < now).reverse();

  home.replaceChildren(
    el(
      "div.home-inner",
      {},
      el(
        "div.home-head",
        {},
        el("h1", {}, "Services"),
        el("span.spacer"),
        app.readonly ? null : el("button.primary.schedule-btn", { onclick: () => openScheduler(), title: "One service, or the coming Sundays" }, "Schedule services…"),
        el("button", { onclick: openFile, title: "Open a service plan (.toml) from anywhere" }, "Open file…"),
        el("button", { onclick: openExample, title: "The built-in example service" }, "Example"),
        app.service ? el("button", { onclick: hideHome, title: "Back to the open service" }, "✕ Close") : null,
      ),
      setupCard(),
      templatesCard(),
      app.mode === "presenter" ? readyCard() : null,
      el("h2", {}, "Upcoming"),
      upcoming.length
        ? el("div.svc-list", {}, upcoming.map(row))
        : el("p.muted", {}, "No upcoming services yet. Use “Schedule services…” to plan the coming Sundays."),
      past.length
        ? el("details.past", {}, el("summary", {}, `Past services (${past.length})`), el("div.svc-list", {}, past.map(row)))
        : null,
    ),
  );
}

/** How the scheduler opened last: one service or the coming Sundays. */
const lastMode = () => {
  try {
    return localStorage.getItem("schedule.mode") === "one" ? "one" : "sundays";
  } catch {
    return "sundays";
  }
};

/**
 * Schedule services: one service on a date (any kind: the Eucharist, Holy
 * Week, an ordination, an event), or a service for each coming Sunday at
 * the parish's service times. The settings they share appear once.
 */
export function openScheduler(mode = lastMode()) {
  const now = today();
  const dates = new Set(services.map((s) => s.date).filter(Boolean));
  const field = (label, input) => el("label.field", {}, el("span", {}, label), input);

  // ---- one service ----
  const date = el("input", { type: "date", value: nextSundayWithout(dates) });
  const title = el("input", { placeholder: "Title" });
  const info = el("div.muted.small.day-info");
  let autoTitle = "";
  let dayName = "";
  let kindTitle = null;
  let kind = "eucharist";
  const setTitle = () => {
    // Keep a title the operator typed; otherwise follow the date and kind.
    const want = kindTitle ?? dayName;
    if (!title.value || title.value === autoTitle) title.value = autoTitle = want;
  };
  const template = templateSelect();
  const opts = serviceOptions();
  const occasion = occasionOptions((k, label) => {
    kind = k;
    kindTitle = ["eucharist", "palm_sunday"].includes(k) ? null : k === "event" ? "Event" : label;
    applyKind();
    setTitle();
  });
  // The time: one of the parish's regular services (which also brings its
  // rite, music and paper, all still changeable here), or another time.
  const time = el(
    "select",
    { onchange: () => pickTime() },
    el("option", { value: "" }, "No time"),
    parishServices.map((r, i) => el("option", { value: `r:${i}`, selected: i === 0 }, regularLabel(r))),
    el("option", { value: "other" }, "Other…"),
  );
  const otherTime = el("input", { type: "time", hidden: true });
  const music = musicChecks(USUAL_MUSIC);
  const paperSel = el(
    "select",
    {},
    el("option", { value: "" }, `The parish's (${PAPERS.find(([v]) => v === parishPaper)?.[1] ?? "11 × 17"})`),
    PAPERS.map(([v, l]) => el("option", { value: v }, `${l}, folded`)),
  );
  const musicRow = el("div.home-row.music-row", {}, el("div.field", {}, el("span", {}, "Music"), music.node), field("Bulletin paper", paperSel));
  const regularOf = () => (time.value.startsWith("r:") ? parishServices[Number(time.value.slice(2))] : null);
  function pickTime() {
    otherTime.hidden = time.value !== "other";
    const r = regularOf();
    if (!r) return;
    opts.setRite(r.rite ?? "II");
    music.set(r.music ?? USUAL_MUSIC);
    paperSel.value = r.paper ?? "";
  }
  const updateDay = async () => {
    const day = date.value ? await invoke("liturgical_info", { date: date.value }).catch(() => null) : null;
    info.textContent = day ? `${day.season} · ${day.color} · Year ${day.year}${day.proper ? ` · Proper ${day.proper}` : ""}` : "";
    dayName = day?.name ?? "";
    await occasion.load(date.value);
    setTitle();
  };
  date.addEventListener("change", updateDay);

  // ---- the coming Sundays ----
  const weeks = el("select", {}, [1, 2, 3, 4, 5, 6, 8].map((n) => el("option", { value: n, selected: n === 4 }, `${n} week${n === 1 ? "" : "s"}`)));
  // One service for each regular service that is ticked.
  const schedTimes = parishServices.map((r) => ({ r, box: el("input", { type: "checkbox", checked: true }) }));
  // "A deacon is serving" applies to the Sundays too: the same setting.
  const deacon = el("input", {
    type: "checkbox",
    checked: occasion.deacon.checked,
    onchange: () => {
      occasion.deacon.checked = deacon.checked;
      occasion.deacon.dispatchEvent(new Event("change"));
    },
  });
  occasion.deacon.addEventListener("change", () => (deacon.checked = occasion.deacon.checked));

  const one = el(
    "div.sched-one",
    {},
    el("div.home-row.date-time", {}, field("Date", date), field("Time", el("div.time-pick", {}, time, otherTime)), field("Title", title)),
    info,
    occasion.row,
    musicRow,
  );
  const sundays = el(
    "div.sched-sundays",
    {},
    el(
      "p.muted.small",
      {},
      "Each of the parish's regular services for the weeks ahead, where there is not one yet: named for the day in the church year, with its Collect, readings and psalm from the lectionary, and its own rite, music and bulletin paper.",
    ),
    el("div.home-row", {}, field("How far ahead", weeks)),
    schedTimes.length
      ? el(
          "div.sched-times",
          {},
          schedTimes.map(({ r, box }) =>
            el("label.check.sched-regular", {}, box, el("span", {}, el("b", {}, regularLabel(r)), el("span.muted.small", {}, ` · ${regularSummary(r, parishPaper)}`))),
          ),
        )
      : el("p.muted.small", {}, "No regular services yet: one service is made each Sunday. Add the parish's regular services in ⚙ Settings → Parish details → Services."),
    el("div.home-row.checks.sched-deacon", {}, el("label.field.inline.check", { title: "With no deacon, the Deacon's parts are labelled Priest" }, deacon, el("span", {}, "A deacon is serving"))),
  );
  const go = el("button.primary", { onclick: () => (mode === "one" ? create() : schedule()) });
  const tabs = el("div.seg.sched-tabs");

  // Holy Week and ordinations use their own order, not a template; an
  // event has no rite, readings or prayers to choose.
  function applyKind() {
    const own = mode === "one" && !["eucharist", "maundy_thursday"].includes(kind);
    template.disabled = own;
    template.title = own ? "This service uses its own Prayer Book order" : "";
    opts.row.hidden = mode === "one" && kind === "event";
    musicRow.hidden = kind === "event";
    // Each regular service brings its own rite.
    opts.perService(mode !== "one" && schedTimes.length > 0);
  }
  const show = (m) => {
    mode = m;
    try {
      localStorage.setItem("schedule.mode", m);
    } catch {}
    one.hidden = m !== "one";
    sundays.hidden = m === "one";
    go.textContent = m === "one" ? "Create & open" : "Schedule";
    tabs.replaceChildren(
      ...[["one", "One service"], ["sundays", "Regular services"]].map(([k, l]) =>
        el(`button${k === m ? ".active" : ""}`, { onclick: () => show(k) }, l),
      ),
    );
    applyKind();
  };

  const close = () => overlay.remove();
  const create = async () => {
    if (!date.value) return toast("Choose a date", "warn");
    const plan = await call("service_create", {
      date: date.value,
      title: title.value || null,
      time: (time.value === "other" ? otherTime.value : regularOf()?.time) || null,
      ...startFrom(template, { ...opts.value(), ...occasion.value(), music: kind === "event" ? null : music.value(), paper: paperSel.value || null }),
    });
    if (plan) {
      close();
      hideHome();
      toast("Service created. It saves automatically as you work.");
    }
  };
  const schedule = async () => {
    const no_deacon = occasion.value().no_deacon;
    const chosen = schedTimes.filter((x) => x.box.checked).map((x) => x.r);
    if (schedTimes.length && !chosen.length) return toast("Tick at least one service", "warn");
    // Each rite's services together, with the prayers last chosen for it.
    const rites = chosen.length ? [...new Set(chosen.map((r) => r.rite ?? "II"))] : [null];
    let made = [];
    for (const rite of rites) {
      const list = chosen.filter((r) => (r.rite ?? "II") === rite);
      const got = await call("services_schedule", {
        from: now,
        weeks: Number(weeks.value),
        regular: list.map((r) => ({ day: r.day ?? "sunday", time: r.time, rite: r.rite ?? "II", music: r.music ?? null, paper: r.paper ?? null })),
        // The deacon setting applies to every service too.
        ...startFrom(template, { ...(rite ? opts.valueFor(rite) : opts.value()), no_deacon }),
      });
      if (got === undefined) return;
      made = made.concat(got);
    }
    toast(made.length ? `Scheduled ${made.length} service${made.length === 1 ? "" : "s"}` : "Those services are already scheduled");
    close();
    showHome();
  };

  const overlay = el(
    "div.modal-overlay",
    { onmousedown: (e) => e.target === overlay && close() },
    el(
      "div.modal.wide.scheduler",
      {},
      el("h3", {}, "Schedule services"),
      tabs,
      one,
      sundays,
      field("Start from", template),
      opts.row,
      el("div.modal-actions", {}, el("button", { onclick: close }, "Cancel"), go),
    ),
  );
  document.body.append(overlay);
  show(mode);
  pickTime();
  updateDay();
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
        ? [
            el("span.svc-dow", {}, formatDate(s.date, { weekday: "short" })),
            el("span.svc-day", {}, formatDate(s.date, { month: "short", day: "numeric" })),
            s.time ? el("span.svc-time", {}, timeLabel(s.time)) : el("span.svc-year", {}, formatDate(s.date, { year: "numeric" })),
          ]
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
      s.in_use && !current
        ? el(
            "span.badge.in-use",
            { title: `${s.in_use.name || "Someone"} has had it open since ${new Date(s.in_use.since * 1000).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}. Opening it here gives view only until you take it over.` },
            `Open on ${s.in_use.name ? `${s.in_use.name}'s computer` : "another computer"}`,
          )
        : null,
      app.mode === "planner" ? readyToggle(s) : null,
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

const when = (secs) =>
  new Date(secs * 1000).toLocaleString(undefined, { weekday: "short", month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });

/** EpiscoPlanner: mark a service Ready for the presentation computer. */
function readyToggle(s) {
  const on = Boolean(s.ready);
  return el(
    "button.small.ready-toggle" + (on ? ".on" : ""),
    {
      title: on
        ? `Ready since ${when(s.ready.at)}${s.ready.by ? ` (${s.ready.by})` : ""}. The presentation computer has a copy; later changes are sent as updates. Click to take it back.`
        : "Mark Ready: the presentation computer copies this service (with its pictures) within a minute.",
      onclick: async (e) => {
        e.stopPropagation();
        const mark = await call("service_set_ready", { path: s.path, ready: !on });
        if (mark === undefined) return;
        s.ready = mark;
        toast(mark ? `“${s.title}” is Ready for the presentation computer` : `“${s.title}” is no longer marked Ready`);
        render();
      },
    },
    on ? "✓ Ready" : "Mark Ready",
  );
}

const READY_STATE = {
  new: ["Copying…", "Waiting to be copied to this computer"],
  copied: ["✓ On this computer", "The same as in the planner"],
  updated: ["Updated in the planner", "Changed in the planner since it was copied. It is copied again automatically unless it is open here."],
  changed_here: ["Changed here", "Edited on this computer since it was copied"],
  conflict: ["Changed in both", "Changed in the planner and on this computer. Refresh takes the planner's version."],
};

/** EpiscoPresenter: services marked Ready in the parish library. */
function readyCard() {
  return el(
    "div.home-card.ready-card",
    {},
    el("h3", {}, "From the planner"),
    ready.length
      ? el(
          "div.ready-list",
          {},
          ready.map((r) => {
            const [label, tip] = READY_STATE[r.state] ?? [r.state, ""];
            return el(
              "div.ready-row",
              {},
              el("span.ready-date", {}, r.date ? formatDate(r.date, { weekday: "short", month: "short", day: "numeric" }) : "—"),
              el("span.ready-title", {}, r.title),
              el("span.muted.small", {}, `Ready ${when(r.ready.at)}${r.ready.by ? ` · ${r.ready.by}` : ""}`),
              el(`span.badge.ready-${r.state}`, { title: tip }, label),
              ["updated", "conflict", "changed_here"].includes(r.state)
                ? el("button.small", { title: "Copy the planner's version again (replaces this computer's copy)", onclick: () => refreshReady(r) }, "Refresh")
                : null,
              r.local ? el("button.small", { onclick: () => open({ path: r.local }) }, "Open") : null,
            );
          }),
        )
      : el(
          "p.muted.small",
          {},
          "Services marked Ready in EpiscoPlanner are copied here, with their pictures and the slide layouts, and run from this computer's copy. Choose the parish library folder with 📁 in the top bar.",
        ),
  );
}

async function refreshReady(r) {
  if (r.state === "conflict" || r.state === "changed_here") {
    const ok = await call("confirm", {
      title: "Refresh from the planner",
      message: `“${r.title}” was changed on this computer. Replace this computer's copy with the planner's version?`,
      okLabel: "Replace",
    });
    if (!ok) return;
  }
  const plan = await call("handoff_refresh", { folder: r.folder });
  if (plan) toast(`“${r.title}” refreshed from the planner`);
}

/** Keeps the presenter's list and the open service's notice up to date. */
export function initHandoff() {
  if (app.mode !== "presenter") return;
  listen("episco://handoff", (list) => {
    ready = list ?? [];
    const home = document.getElementById("home");
    if (home && !home.hidden) render();
    handoffNotice();
  });
  app.on((what) => what === "service" && handoffNotice());
}

/** "Updated in the planner — Refresh" for the open service. */
function handoffNotice() {
  const btn = document.getElementById("handoff-note");
  if (!btn) return;
  const open = app.service?.plan_path;
  const r = open ? ready.find((x) => x.local === open && ["updated", "conflict"].includes(x.state)) : null;
  btn.hidden = !r;
  if (!r) return;
  btn.textContent = r.state === "conflict" ? "⟳ Changed in the planner and here" : "⟳ Updated in the planner";
  btn.onclick = () => refreshReady(r);
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
