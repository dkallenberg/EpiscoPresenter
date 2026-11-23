// A new version of the app, downloaded in the background (updater.rs):
// "Restart to update" in the top bar. Nothing is installed until someone
// clicks it, so an update never interrupts a service. In EpiscoPresenter the
// offer waits while a service is on the screens (a cue taken and the
// sanctuary not blacked out), and comes back when they are idle again.
//
// ⚙ Settings → Updates… shows the version installed and sets whether this
// computer takes betas (EpiscoPlanner does unless told not to; EpiscoPresenter
// takes only full releases unless told to).

import { invoke, listen } from "/js/bridge.js";
import { app, call, el, toast } from "/js/control/util.js";

let offer = null; // the update waiting: { version, notes }
let later = false;

const appName = () => (app.mode === "planner" || window.__EPISCO_MODE__ === "planner" ? "EpiscoPlanner" : "EpiscoPresenter");

/** A service is on the screens: restarting now would blank them. */
function onScreen() {
  if (app.mode === "planner") return false;
  const s = app.state;
  return Boolean(s?.active) && !s?.blackout;
}

async function install(button) {
  // The screens close for a moment while the app restarts.
  if (app.mode !== "planner") {
    const status = await call("outputs_status").catch(() => null);
    if (status?.outputs?.some((o) => o.open)) {
      const ok = await call("confirm", {
        title: "Restart to update",
        message: "The output screens close while the app restarts, and open again in about a minute.",
        okLabel: "Restart now",
      });
      if (!ok) return;
    }
  }
  button.disabled = true;
  button.textContent = "Installing…";
  // The app closes and opens again; on failure it stays as it was.
  const ok = await call("update_install");
  if (ok === undefined) {
    button.disabled = false;
    button.textContent = "Restart to update";
  }
}

function render() {
  const note = document.getElementById("update-note");
  if (!note) return;
  if (!offer?.version || later || onScreen()) {
    note.hidden = true;
    return;
  }
  if (note.dataset.version !== offer.version) {
    note.dataset.version = offer.version;
    note.replaceChildren(
      el("span", {}, `${appName()} ${offer.version} is ready`),
      el("button.small.primary", { title: "Install the new version and open the app again (screens close for a moment)", onclick: (e) => install(e.target) }, "Restart to update"),
      el(
        "button.small",
        {
          title: "Keep working; the update is offered again next time the app starts",
          onclick: () => {
            later = true;
            render();
            toast(`${appName()} ${offer.version} will be offered again next time it starts`);
          },
        },
        "Later",
      ),
    );
    if (offer.notes) note.title = offer.notes;
  }
  note.hidden = false;
}

function show(info) {
  if (!info?.version) return;
  offer = info;
  render();
}

export async function initUpdates() {
  listen("episco://update", (info) => show(info));
  // Hidden while a service is on the screens; back when they're idle.
  app.on((what) => what === "state" && render());
  // Downloaded before the window was listening.
  const ready = await invoke("update_ready").catch(() => null);
  if (ready) show(ready);
}

/** ⚙ Settings → Updates…: the version installed, betas or not, Check now. */
export async function openUpdates() {
  const s = await call("update_status");
  if (!s) return;
  const name = appName();
  const close = () => overlay.remove();
  const line = el("p.small.muted", {}, s.ready ? `${name} ${s.ready.version} is downloaded and ready: Restart to update in the top bar.` : "");
  const choice = (betas, label, about) =>
    el(
      "label.todo-radio",
      {},
      el("input", {
        type: "radio",
        name: "update-betas",
        checked: s.betas === betas,
        onchange: async () => {
          if ((await call("update_set_betas", { betas })) !== undefined) {
            s.betas = betas;
            toast(betas ? "This computer takes betas too" : "This computer takes only full releases");
          }
        },
      }),
      el("span", {}, el("b", {}, label), el("span.muted", {}, ` · ${about}`)),
    );
  const check = el("button", {
    disabled: !s.can_update,
    onclick: async () => {
      check.disabled = true;
      line.textContent = "Looking for a newer version…";
      try {
        const found = await invoke("update_check_now");
        line.textContent = found ? `${name} ${found.version} is downloaded and ready: Restart to update in the top bar.` : `${name} ${s.version} is the newest ${s.betas ? "version" : "full release"}.`;
      } catch (e) {
        line.textContent = `Couldn't look for updates: ${e}`;
      } finally {
        check.disabled = false;
      }
    },
  }, "Check now");
  const overlay = el(
    "div.modal-overlay.todo-pop",
    { onmousedown: (e) => e.target === overlay && close() },
    el(
      "div.modal",
      {},
      el("h3", {}, "Updates"),
      el("p", {}, `${name} ${s.version} is installed on this computer.`),
      el(
        "div.todo-radios",
        { role: "radiogroup" },
        choice(false, "Full releases only", "tested versions, for the computer that runs Sunday's services"),
        choice(true, "Include betas", "new features to try before they're finished"),
      ),
      el("p.small.muted", {}, `${name} looks for a newer version a few seconds after it starts, downloads it in the background, and installs it only when you click Restart to update${app.mode === "planner" ? "" : " (not offered while a service is on the screens)"}. Choosing full releases doesn't go back from a beta already installed; the next full release replaces it.`),
      s.can_update ? null : el("p.small.warn-text", {}, "This copy of the app can't update itself: install new versions from the Releases page."),
      line,
      el("div.modal-actions", {}, check, el("button.primary", { onclick: close }, "Done")),
    ),
  );
  document.body.append(overlay);
}
