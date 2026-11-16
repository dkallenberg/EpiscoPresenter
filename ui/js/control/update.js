// A new version of the app, downloaded in the background (updater.rs):
// "Restart to update" in the top bar. Nothing is installed until someone
// clicks it, so an update never interrupts a service.

import { invoke, listen } from "/js/bridge.js";
import { call, el, toast } from "/js/control/util.js";

let shownFor = null;
let later = false;

function show(info) {
  const note = document.getElementById("update-note");
  if (!note || !info?.version || later) return;
  shownFor = info.version;
  const name = document.title.includes("Planner") || window.__EPISCO_MODE__ === "planner" ? "EpiscoPlanner" : "EpiscoPresenter";
  note.replaceChildren(
    el("span", {}, `${name} ${info.version} is ready`),
    el(
      "button.small.primary",
      {
        title: "Install the new version and open the app again (screens close for a moment)",
        onclick: async (e) => {
          e.target.disabled = true;
          e.target.textContent = "Installing…";
          // The app closes and opens again; on failure it stays as it was.
          const ok = await call("update_install");
          if (ok === undefined) {
            e.target.disabled = false;
            e.target.textContent = "Restart to update";
          }
        },
      },
      "Restart to update",
    ),
    el(
      "button.small",
      {
        title: "Keep working; the update is offered again next time the app starts",
        onclick: () => {
          later = true;
          note.hidden = true;
          toast(`${name} ${shownFor} will be offered again next time it starts`);
        },
      },
      "Later",
    ),
  );
  if (info.notes) note.title = info.notes;
  note.hidden = false;
}

export async function initUpdates() {
  listen("episco://update", (info) => show(info));
  // Downloaded before the window was listening.
  const ready = await invoke("update_ready").catch(() => null);
  if (ready) show(ready);
}
