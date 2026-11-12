// Thin wrapper over the Tauri IPC bridge plus a small reactive store holding
// the latest snapshot ({ state, service, link }) for this node.

const T = window.__TAURI__;

export const inTauri = Boolean(T);

export const EVENTS = {
  state: "episco://state",
  service: "episco://service",
  link: "episco://link",
};

export function invoke(cmd, args) {
  if (!T) return Promise.reject(new Error("Not running inside the EpiscoPresenter app"));
  return T.core.invoke(cmd, args);
}

export function listen(event, cb) {
  if (!T) return Promise.resolve(() => {});
  return T.event.listen(event, (e) => cb(e.payload));
}

let planner = false;
/** In EpiscoPlanner nothing goes on screen: presentation commands do nothing. */
export function setPlannerMode(on) {
  planner = on;
}

/** Sends a presentation command, e.g. send({ type: "next" }). */
export function send(command) {
  if (planner) return Promise.resolve();
  return invoke("send_command", { command });
}

/**
 * Subscribes `onChange(snapshot)` to every state / service / link change.
 * Listeners are attached before the initial snapshot is fetched so that no
 * update can fall in between.
 */
export async function connectStore(onChange) {
  const snap = { state: { revision: 0 }, service: null, link: {} };
  let ready = false;
  const emit = () => ready && onChange(snap);

  await listen(EVENTS.state, (state) => { snap.state = state; emit(); });
  await listen(EVENTS.service, (service) => { snap.service = service; emit(); });
  await listen(EVENTS.link, (link) => { snap.link = link; emit(); });

  if (inTauri) {
    const initial = await invoke("get_snapshot");
    // Prefer anything that arrived while the snapshot was in flight.
    if ((snap.state.revision ?? 0) <= initial.state.revision) {
      snap.state = initial.state;
      snap.service = initial.service;
    }
    snap.link = { ...initial.link, ...snap.link };
  }
  ready = true;
  onChange(snap);
  return snap;
}

/** The cue + slide currently on air, or null. */
export function activeSlide(snap) {
  const pos = snap.state?.active;
  const cue = pos && snap.service?.cues?.[pos.cue];
  if (!cue) return null;
  const slide = cue.content.kind === "liturgy" ? cue.content.slides[pos.slide] ?? null : null;
  return { cue, slide, pos };
}

/**
 * URL for an image referenced from a cue's .ewm file. Relative paths resolve
 * against the .ewm file's folder and load through Tauri's asset protocol.
 * (On a slave node these are the master's paths; the stream overlay never
 * shows images, so that is fine.)
 */
export function assetUrl(cue, src) {
  if (!src || /^(https?:|data:|asset:|blob:)/i.test(src)) return src;
  const absolute = src.startsWith("/") || /^[A-Za-z]:[\\/]/.test(src);
  const source = cue?.content?.source;
  let path = src;
  if (!absolute) {
    if (!source) return src;
    path = source.replace(/[^\\/]*$/, "") + src;
  }
  return T?.core?.convertFileSrc ? T.core.convertFileSrc(path) : path;
}

/**
 * URL for a background image value from a look: relative paths resolve
 * against the service plan's folder.
 */
export function backgroundUrl(service, value) {
  if (!value || /^(https?:|data:|asset:|blob:)/i.test(value)) return value;
  const absolute = value.startsWith("/") || /^[A-Za-z]:[\\/]/.test(value);
  let path = value;
  if (!absolute) {
    const plan = service?.plan_path;
    if (!plan) return value;
    path = plan.replace(/[^\\/]*$/, "") + value;
  }
  return T?.core?.convertFileSrc ? T.core.convertFileSrc(path) : path;
}
