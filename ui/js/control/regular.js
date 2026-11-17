// The parish's regular services ("Sunday 8:00, Rite I, closing hymn and
// postlude only"): their editor in Parish details, and the music checkboxes
// and labels the scheduler uses (crates/episco-core/src/regular.rs).

import { el } from "/js/control/util.js";

/** The music places, in service order. */
export const MUSIC_PLACES = [
  ["prelude", "Prelude"],
  ["entrance_hymn", "Entrance Hymn"],
  ["hymn_of_praise", "Hymn of Praise (Gloria sung)"],
  ["gradual", "Gradual"],
  ["sequence", "Sequence"],
  ["offertory_anthem", "Offertory Anthem"],
  ["offertory_hymn", "Offertory Hymn"],
  ["communion_hymn", "Communion Hymn"],
  ["closing_hymn", "Closing Hymn"],
  ["postlude", "Postlude"],
];
/** The places the standard order has. */
export const USUAL_MUSIC = ["prelude", "entrance_hymn", "hymn_of_praise", "offertory_hymn", "communion_hymn", "closing_hymn", "postlude"];

export const DAYS = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
const dayLabel = (d) => d.charAt(0).toUpperCase() + d.slice(1);

export const PAPERS = [
  ["tabloid", "11 × 17"],
  ["legal", "8½ × 14"],
  ["letter", "8½ × 11"],
];
const paperLabel = (p) => PAPERS.find(([v]) => v === p)?.[1] ?? "";

/** "8:00 am" from "08:00". */
export function clock(t) {
  const m = /^(\d{1,2}):(\d{2})$/.exec(t ?? "");
  if (!m) return t ?? "";
  const h = Number(m[1]);
  return `${h % 12 || 12}:${m[2]} ${h < 12 ? "am" : "pm"}`;
}

/** "Sunday 8:00 am · Early service". */
export function regularLabel(r) {
  return [`${dayLabel(r.day ?? "sunday")} ${clock(r.time)}`.trim(), r.name?.trim()].filter(Boolean).join(" · ");
}

/** "Rite I · Closing Hymn, Postlude · 8½ × 11". */
export function regularSummary(r, parishPaper) {
  const music = r.music ?? USUAL_MUSIC;
  const m = !music.length
    ? "No music"
    : MUSIC_PLACES.every(([k]) => music.includes(k))
      ? "All music"
      : MUSIC_PLACES.filter(([k]) => music.includes(k)).map(([, l]) => l.replace(/ \(.*\)$/, "")).join(", ");
  return [r.rite === "I" ? "Rite I" : "Rite II", m, paperLabel(r.paper ?? parishPaper)].filter(Boolean).join(" · ");
}

/**
 * Checkboxes for the music places, and "No music". `chosen` is the list of
 * places; `onChange(list)` is called with the new list.
 */
export function musicChecks(chosen, onChange = () => {}) {
  let list = [...(chosen ?? USUAL_MUSIC)];
  const boxes = MUSIC_PLACES.map(([key, label]) => {
    const box = el("input", {
      type: "checkbox",
      checked: list.includes(key),
      onchange: () => {
        list = MUSIC_PLACES.map(([k]) => k).filter((k) => (k === key ? box.checked : list.includes(k)));
        none.checked = !list.length;
        onChange(list);
      },
    });
    return [key, box, el("label.check", {}, box, el("span", {}, label))];
  });
  const none = el("input", {
    type: "checkbox",
    checked: !list.length,
    onchange: () => {
      list = none.checked ? [] : [...USUAL_MUSIC];
      for (const [k, box] of boxes) box.checked = list.includes(k);
      onChange(list);
    },
  });
  const node = el("div.music-checks", {}, boxes.map(([, , label]) => label), el("label.check.no-music", {}, none, el("span", {}, "No music")));
  return {
    node,
    value: () => [...list],
    set: (next) => {
      list = [...(next ?? USUAL_MUSIC)];
      for (const [k, box] of boxes) box.checked = list.includes(k);
      none.checked = !list.length;
    },
  };
}

/** The editor for the regular services, in Parish details. Edits `list` in place. */
export function regularEditor(list, parishPaper = () => "tabloid") {
  const host = el("div.regular-list");
  const render = () => {
    host.replaceChildren(
      ...list.map((r, i) => {
        r.music ??= [...USUAL_MUSIC];
        const field = (label, input) => el("label.field", {}, el("span", {}, label), input);
        const name = el("input", { value: r.name ?? "", placeholder: "e.g. Early service", oninput: (e) => (r.name = e.target.value) });
        const day = el(
          "select",
          { onchange: (e) => (r.day = e.target.value) },
          DAYS.map((d) => el("option", { value: d, selected: d === (r.day ?? "sunday") }, dayLabel(d))),
        );
        const time = el("input", { type: "time", value: r.time ?? "", onchange: (e) => (r.time = e.target.value) });
        const rite = el(
          "select",
          { onchange: (e) => (r.rite = e.target.value) },
          [["II", "Rite II"], ["I", "Rite I"]].map(([v, l]) => el("option", { value: v, selected: v === (r.rite ?? "II") }, l)),
        );
        const paper = el(
          "select",
          { onchange: (e) => (r.paper = e.target.value || null) },
          el("option", { value: "", selected: !r.paper }, `The parish's (${paperLabel(parishPaper())})`),
          PAPERS.map(([v, l]) => el("option", { value: v, selected: v === r.paper }, `${l}, folded`)),
        );
        const music = musicChecks(r.music, (m) => (r.music = m));
        return el(
          "div.regular-card",
          {},
          el(
            "div.regular-row",
            {},
            field("Day", day),
            field("Time", time),
            field("Name (optional)", name),
            field("Rite", rite),
            field("Bulletin paper", paper),
            el("button.small.regular-remove", { title: "Remove this service", onclick: () => (list.splice(i, 1), render()) }, "✕"),
          ),
          el("div.field", {}, el("span", {}, "Music"), music.node),
        );
      }),
      ...(list.length ? [] : [el("p.muted.small", {}, "No regular services yet. Add the parish's Sunday services (and any weekday ones).")]),
    );
  };
  render();
  return {
    node: el(
      "div",
      {},
      host,
      el("button.small", { onclick: () => (list.push({ name: "", day: "sunday", time: "", rite: "II", music: [...USUAL_MUSIC], paper: null }), render()) }, "+ Add a service"),
    ),
    render,
  };
}
