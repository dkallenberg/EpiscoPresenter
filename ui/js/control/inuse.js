// EpiscoPlanner: "Open on another computer". When someone else has the open
// service, it is view only here, with "Take over" in the top bar; when
// someone takes over ours, it turns view only and says who (inuse.rs).

import { listen } from "/js/bridge.js";
import { app, call, toast } from "/js/control/util.js";

let status = { view_only: false };

const who = (m) => (m?.name ? `${m.name}'s computer` : "another computer");
const at = (secs) => new Date(secs * 1000).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });

function show() {
  const btn = document.getElementById("inuse-note");
  if (!btn) return;
  const viewOnly = Boolean(status.view_only && app.service);
  btn.hidden = !viewOnly;
  if (viewOnly) {
    const m = status.holder;
    btn.textContent = m ? `Open on ${who(m)} · Take over` : "View only · Take over to edit";
    btn.title = m
      ? `${m.name || "Someone"} has had this service open since ${at(m.since)}. You can look at it here. Take it over to change it: it reloads as last saved, and it turns view only on ${who(m)}.`
      : "Whoever had this service open has closed it. Take it over to change it (it reloads as last saved).";
  }
  // View only while someone else has it (and always on the live-stream computer).
  const was = app.readonly;
  app.readonly = app.snap?.link?.role === "slave" || viewOnly;
  document.body.classList.toggle("readonly", app.readonly);
  if (was !== app.readonly) app.emit("service");
}

function update(next, fromEvent) {
  if (fromEvent && next?.taken && !status.taken) {
    toast(`${who(next.holder)} took over this service. It is view only here now; what you changed before is saved.`, "warn");
  }
  status = next ?? { view_only: false };
  show();
}

export function initInUse() {
  if (app.mode !== "planner") return;
  listen("episco://in-use", (s) => update(s, true));
  document.getElementById("inuse-note")?.addEventListener("click", async () => {
    const ok = await call("in_use_take_over");
    if (ok !== undefined) toast("You have this service now; it was reloaded as last saved.");
  });
  // A new service: ask (the event may have come before this window was ready).
  app.on(async (what) => {
    if (what !== "service") return;
    update(await call("in_use_status"), false);
  });
}

/** Whether the open service is view only because it is open elsewhere. */
export const viewOnly = () => Boolean(status.view_only);
