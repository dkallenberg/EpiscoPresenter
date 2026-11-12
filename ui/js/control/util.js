// Small DOM and app-state helpers shared by the control-window modules.

import { invoke } from "/js/bridge.js";

/** el("div#id.cls1.cls2", {attrs / on* handlers / dataset}, ...children) */
export function el(spec, props = {}, ...children) {
  // "tag", then any mix of ".class" and "#id" parts, in any order.
  const tag = spec.match(/^[a-z0-9]*/i)[0] || "div";
  const node = document.createElement(tag);
  const id = spec.match(/#([\w-]+)/);
  if (id) node.id = id[1];
  const classes = [...spec.matchAll(/\.([\w-]+)/g)].map((m) => m[1]);
  if (classes.length) node.className = classes.join(" ");
  for (const [k, v] of Object.entries(props ?? {})) {
    if (v == null || v === false) continue;
    if (k.startsWith("on") && typeof v === "function") node.addEventListener(k.slice(2), v);
    else if (k === "dataset") Object.assign(node.dataset, v);
    else if (k === "style" && typeof v === "object") Object.assign(node.style, v);
    else if (k in node && k !== "list") node[k] = v;
    else node.setAttribute(k, v === true ? "" : v);
  }
  for (const c of children.flat()) if (c != null && c !== false) node.append(c);
  return node;
}

export function debounce(fn, ms) {
  let t;
  const wrapped = (...args) => {
    clearTimeout(t);
    t = setTimeout(() => fn(...args), ms);
  };
  wrapped.flush = (...args) => {
    clearTimeout(t);
    fn(...args);
  };
  wrapped.cancel = () => clearTimeout(t);
  return wrapped;
}

export const titleCase = (key) => (key ?? "").replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());

// ---------- toasts ----------
export function toast(message, kind = "info") {
  let host = document.getElementById("toasts");
  if (!host) document.body.append((host = el("div", { id: "toasts" })));
  const t = el(`div.toast.${kind}`, {}, message);
  host.append(t);
  setTimeout(() => t.classList.add("leaving"), kind === "error" ? 7000 : 3500);
  setTimeout(() => t.remove(), kind === "error" ? 7600 : 4100);
}

/** invoke() that reports failures as an error toast and returns undefined. */
export async function call(cmd, args) {
  try {
    return await invoke(cmd, args);
  } catch (e) {
    toast(String(e), "error");
    return undefined;
  }
}

export function reportWarnings(warnings) {
  if (warnings?.length) toast(`${warnings.length} warning(s): ${warnings.slice(0, 3).join(" · ")}`, "warn");
}

// ---------- navigation (mirrors episco_core::state) ----------
export function slideCount(cue) {
  return cue.content.kind === "liturgy" ? Math.max(1, cue.content.slides.length) : 1;
}

export function nextPosition(service, active) {
  const cues = service?.cues ?? [];
  if (!cues.length) return null;
  if (!active) return { cue: 0, slide: 0 };
  if (active.slide + 1 < slideCount(cues[active.cue])) return { cue: active.cue, slide: active.slide + 1 };
  if (active.cue + 1 < cues.length) return { cue: active.cue + 1, slide: 0 };
  return null;
}

/** Plain-text summary of a slide for labels. */
export function slideSummary(slide) {
  if (!slide) return "";
  for (const b of slide.blocks) {
    if (b.kind === "paragraph" && b.role !== "Rubric") return b.lines.map((l) => l.map((s) => s.text).join("")).join(" ");
    if (b.kind === "image") return `♪ ${b.alt || b.src}`;
  }
  return "";
}

// ---------- tiny pub/sub app context ----------
export const app = {
  snap: null,
  vocab: null,
  readonly: false,
  /** "presenter" or "planner" (EpiscoPlanner). */
  mode: "presenter",
  selectedId: null, // cue shown in the slide grid / editor
  listeners: new Set(),
  on(fn) {
    this.listeners.add(fn);
  },
  /** what: "service" | "state" | "link" | "selection" */
  emit(what) {
    for (const fn of this.listeners) fn(what);
  },
  get service() {
    return this.snap?.service ?? null;
  },
  get state() {
    return this.snap?.state ?? {};
  },
  selectedIndex() {
    return this.service?.cues.findIndex((c) => c.id === this.selectedId) ?? -1;
  },
  selectedCue() {
    return this.service?.cues.find((c) => c.id === this.selectedId) ?? null;
  },
  select(id) {
    if (this.selectedId === id) return;
    this.selectedId = id;
    this.emit("selection");
  },
};
