// The parish library folder (shared, e.g. in Google Drive) and the name of
// the person at this computer. In EpiscoPlanner the top bar shows the
// folder in use; the first run asks for both.

import { listen } from "/js/bridge.js";
import { app, call, el, toast } from "/js/control/util.js";

let info = null;

const folderName = (p) => (p ? String(p).split(/[\\/]/).filter(Boolean).pop() : "");

/** The library folder as last read (for the Getting started list). */
export const storageInfo = () => info;

async function load() {
  info = (await call("storage_get")) ?? info;
  render();
  return info;
}

function render() {
  const b = document.getElementById("btn-library-folder");
  if (!b || !info) return;
  const shared = Boolean(info.folder) && !info.missing;
  if (info.presenter) {
    // The presentation app only takes Ready services from the library.
    b.textContent = shared ? `Parish library folder: ${folderName(info.folder)}…` : info.missing ? "⚠ Parish library folder missing…" : "Parish library folder: not chosen…";
    document.getElementById("btn-menu")?.classList.toggle("warn", Boolean(info.missing));
    b.classList.toggle("warn", Boolean(info.missing));
    b.title = info.missing
      ? `The parish library ${info.folder} cannot be found. Is Google Drive running? Services already copied to this computer still work.`
      : shared
        ? `Services marked Ready in EpiscoPlanner are copied from ${info.folder} to this computer, which works from its own copy.`
        : "Choose the parish library folder to receive services marked Ready in EpiscoPlanner.";
    return;
  }
  b.textContent = shared ? `Parish library folder: ${folderName(info.folder)}…` : info.missing ? "⚠ Parish library folder missing…" : "Parish library folder: this computer only…";
  // A missing folder needs attention: mark the menu too.
  document.getElementById("btn-menu")?.classList.toggle("warn", Boolean(info.missing));
  b.classList.toggle("warn", Boolean(info.missing) || !info.folder);
  b.title = info.missing
    ? `The library folder ${info.folder} cannot be found. Is Google Drive running? Working from this computer's folder until it is back.`
    : shared
      ? `Parish library: ${info.folder}${info.name ? `\nYou are: ${info.name}` : ""}`
      : "Services and the library are kept on this computer only. Click to choose the shared parish library folder.";
}

/** The library folder and name dialog. `welcome`: first run of the planner. */
export function openLibrarySettings(welcome = false) {
  return new Promise((resolve) => {
    let folder = info?.folder ?? null;
    const path = el("div.lib-path");
    const showPath = () => {
      path.textContent = folder ?? "Not chosen: services and the library stay on this computer.";
      path.classList.toggle("muted", !folder);
    };
    showPath();
    const name = el("input", { value: info?.name ?? "", placeholder: "e.g. Rector, Admin, Music, Tech" });
    const choose = async () => {
      const picked = await call("storage_pick_folder");
      if (picked) {
        folder = picked;
        showPath();
      }
    };
    const done = (v) => {
      overlay.remove();
      resolve(v);
    };
    const presenter = Boolean(info?.presenter);
    const save = async () => {
      if (!presenter && !name.value.trim()) return toast("Enter your name, so the others can see who changed what.", "warn");
      const r = await call("storage_set", { folder, name: name.value.trim() });
      if (r) {
        info = r;
        render();
        toast(
          presenter
            ? folder
              ? `Services marked Ready in “${folderName(folder)}” will be copied to this computer`
              : "No parish library: this computer's services only"
            : folder
              ? `Using the parish library in “${folderName(folder)}”`
              : "Using this computer's folder",
        );
        done(r);
      }
    };
    const overlay = el(
      "div.modal-overlay",
      { onmousedown: (e) => !welcome && e.target === overlay && done(null) },
      el(
        "div.modal",
        {},
        el("h3", {}, welcome ? "Welcome to EpiscoPlanner" : "Parish library"),
        presenter
          ? el(
              "p.small",
              {},
              "EpiscoPresenter does not work from the shared folder. Services marked Ready in EpiscoPlanner are copied to this computer (with their pictures and the slide layouts) and run from that copy, so Google Drive cannot affect a service in progress.",
            )
          : null,
        welcome
          ? el(
              "p.small",
              {},
              "Services, templates, hymns and other library items, pictures and slide layouts are kept in one shared folder, so the rector, administrator, musician and technician all work from the same files.",
            )
          : null,
        el("div.field", {}, el("span", {}, "Parish library folder"), path),
        el(
          "div.modal-actions.left",
          {},
          el("button", { onclick: choose }, folder ? "Change folder…" : "Choose folder…"),
          el("button", { onclick: () => ((folder = null), showPath()) }, "This computer only"),
        ),
        el(
          "p.muted.small",
          {},
          presenter
            ? "The same folder EpiscoPlanner uses, e.g. the shared Google Drive folder. It is checked every minute."
            : "Use a folder everyone can reach, e.g. a shared Google Drive folder synced with Google Drive for desktop. Set it to “Available offline” on the presentation computer. An empty folder is set up for you.",
        ),
        presenter ? null : el("label.field", {}, el("span", {}, "Your name"), name),
        el("div.modal-actions", {}, welcome ? null : el("button", { onclick: () => done(null) }, "Cancel"), el("button.primary", { onclick: save }, welcome ? "Start" : "Save")),
      ),
    );
    document.body.append(overlay);
    (welcome && !folder ? overlay.querySelector("button") : name).focus();
  });
}

export async function initStorage() {
  const b = document.getElementById("btn-library-folder");
  if (b) b.addEventListener("click", () => openLibrarySettings(false));
  listen("episco://storage", () => load());
  await load();
  // First run of the planner: choose the library and enter a name.
  if (app.mode === "planner" && info && !info.name) await openLibrarySettings(true);
}
