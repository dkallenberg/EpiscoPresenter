// The parish library folder (shared, e.g. in Google Drive) and the name of
// the person at this computer. In EpiscoPlanner the top bar shows the
// folder in use; the first run asks for both.

import { listen } from "/js/bridge.js";
import { app, call, el, toast } from "/js/control/util.js";

let info = null;
let drive = null; // the Google Drive connection (drive_status)

const folderName = (p) => (p ? String(p).split(/[\\/]/).filter(Boolean).pop() : "");

/** The library folder as last read (for the Getting started list). */
export const storageInfo = () => info;

async function load() {
  info = (await call("storage_get")) ?? info;
  render();
  return info;
}

async function loadDrive() {
  if (!window.__TAURI__) return drive;
  drive = (await window.__TAURI__.core.invoke("drive_status").catch(() => null)) ?? drive;
  render();
  return drive;
}

/** "Google Drive · Parish Library" (or the folder's own name). */
const libraryName = () => (info?.drive ? `Google Drive · ${info.drive.name}` : folderName(info?.folder));

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
  if (info.drive) {
    // A Google Drive folder reached directly: say how its copy stands.
    const trouble = drive && (drive.error || !drive.account);
    b.textContent = `${trouble ? "⚠ " : ""}Parish library: ${libraryName()}…`;
    document.getElementById("btn-menu")?.classList.toggle("warn", Boolean(trouble));
    b.classList.toggle("warn", Boolean(trouble));
    b.title = !drive
      ? "Parish library in Google Drive"
      : !drive.account
        ? "Not signed in to Google on this computer: sign in to keep the parish library in step."
        : drive.error
          ? `Google Drive: ${drive.error}`
          : !drive.ready
            ? `Copying “${info.drive.name}” from Google Drive to this computer…`
            : `Parish library: “${info.drive.name}” in Google Drive, signed in as ${drive.account}${info.name ? `\nYou are: ${info.name}` : ""}`;
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
    // A folder on this computer (e.g. synced by Google Drive for desktop).
    let folder = info?.drive ? null : (info?.folder ?? null);
    // A Google Drive folder reached directly.
    let picked = info?.drive ?? null;
    let mode = info?.drive || (!info?.folder && welcome) ? "drive" : "folder";
    const path = el("div.lib-path");
    const showPath = () => {
      path.textContent = folder ?? "Not chosen: services and the library stay on this computer.";
      path.classList.toggle("muted", !folder);
    };
    showPath();
    const name = el("input", { value: info?.name ?? "", placeholder: "e.g. Rector, Admin, Music, Tech" });
    const choose = async () => {
      const p = await call("storage_pick_folder");
      if (p) {
        folder = p;
        showPath();
      }
    };
    const done = (v) => {
      if (typeof offDrive === "function") offDrive();
      overlay.remove();
      resolve(v);
    };
    const presenter = Boolean(info?.presenter);

    // ---- Google Drive: the client file, the account, the folder.
    const drivePanel = el("div.drive-panel");
    let folders = null; // the list, while choosing
    let searching = "";
    const step = (n, title, state, ...body) =>
      el(`div.drive-step${state === "done" ? ".done" : state === "todo" ? ".todo" : ""}`, {}, el("span.drive-n", {}, state === "done" ? "✓" : String(n)), el("div", {}, el("strong", {}, title), ...body));
    const listFolders = async () => {
      folders = "loading";
      renderDrive();
      folders = (await call("drive_folders", { search: searching || null })) ?? [];
      renderDrive();
    };
    const renderDrive = () => {
      const d = drive ?? {};
      const client = step(
        1,
        "The parish's Google client file",
        d.client ? "done" : "todo",
        el("p.muted.small", {}, d.client ? "Added on this computer." : "From the parish's Google Cloud project (a JSON file, done once by whoever set the project up; see the setup steps)."),
        el("button.small", { onclick: async () => { if (await call("drive_import_client")) { await loadDrive(); renderDrive(); toast("Google client file added"); } } }, d.client ? "Replace…" : "Add the client file…"),
      );
      const account = step(
        2,
        "Your Google account",
        d.account ? "done" : d.client ? "todo" : "",
        el("p.muted.small", {}, d.account ? `Signed in as ${d.account}.` : "Sign in in your browser. Google may say it hasn't verified the app: choose Advanced, then Go to … (unsafe), then Allow. It only reaches the parish's own Google project."),
        d.account
          ? el("button.small", { onclick: async () => { await call("drive_sign_out"); await loadDrive(); folders = null; renderDrive(); } }, "Sign out")
          : el(
              "button.small",
              {
                disabled: !d.client,
                onclick: async (e) => {
                  e.target.disabled = true;
                  e.target.textContent = "Finish signing in in your browser…";
                  const email = await call("drive_sign_in");
                  await loadDrive();
                  renderDrive();
                  if (email) {
                    toast(`Signed in as ${email}`);
                    listFolders();
                  }
                },
              },
              "Sign in with Google…",
            ),
      );
      const search = el("input.drive-search", {
        type: "search",
        placeholder: "Search your folders and those shared with you",
        value: searching,
        onkeydown: (e) => {
          if (e.key === "Enter") {
            searching = e.target.value.trim();
            listFolders();
          }
        },
      });
      const list =
        folders === "loading"
          ? el("p.muted.small", {}, "Looking in Google Drive…")
          : Array.isArray(folders)
            ? folders.length
              ? el(
                  "div.drive-folders",
                  {},
                  folders.map((f) =>
                    el(
                      `button.drive-folder${picked?.id === f.id ? ".on" : ""}`,
                      { onclick: () => { picked = { id: f.id, name: f.name }; renderDrive(); } },
                      el("span", {}, "📁 ", f.name),
                      f.owner ? el("span.muted.small", {}, ` shared by ${f.owner}`) : "",
                    ),
                  ),
                )
              : el("p.muted.small", {}, searching ? "No folder by that name." : "No folders found.")
            : null;
      const chooser = step(
        3,
        "The parish folder",
        picked && folders === null ? "done" : d.account ? "todo" : "",
        el("p.muted.small", {}, picked ? `“${picked.name}”` : "The shared folder in Google Drive (its owner shares it with everyone's Google account first)."),
        d.account && folders === null ? el("button.small", { onclick: listFolders }, picked ? "Choose another…" : "Choose the folder…") : "",
        d.account && folders !== null ? el("div.drive-choose", {}, search, list ?? "") : "",
      );
      const status = info?.drive && drive
        ? el(
            "p.small.drive-status",
            {},
            drive.error
              ? `⚠ ${drive.error}`
              : !drive.ready
                ? drive.busy ? "Copying the folder to this computer…" : "Waiting to copy the folder to this computer…"
                : `✓ In step with Google Drive${drive.at ? ` · checked ${new Date(drive.at * 1000).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })}` : ""}`,
          )
        : "";
      drivePanel.replaceChildren(client, account, chooser, status);
    };
    // Keep the steps current while the dialog is open (sign-in, first copy).
    let offDrive = null;
    listen("episco://drive", async () => {
      await loadDrive();
      renderDrive();
    }).then((off) => (offDrive = off));

    // ---- A folder on this computer.
    const folderPanel = el(
      "div.folder-panel",
      {},
      el("div.field", {}, el("span", {}, "Parish library folder"), path),
      el(
        "div.modal-actions.left",
        {},
        el("button", { onclick: choose }, "Choose folder…"),
        el("button", { onclick: () => ((folder = null), showPath()) }, "This computer only"),
      ),
      el(
        "p.muted.small",
        {},
        presenter
          ? "The same folder EpiscoPlanner uses, e.g. the shared Google Drive folder. It is checked every minute."
          : "A folder everyone can reach, e.g. a shared Google Drive folder synced with Google Drive for desktop. An empty folder is set up for you.",
      ),
    );
    const tabs = el(
      "div.seg.lib-modes",
      {},
      [["drive", "Google Drive"], ["folder", "A folder on this computer"]].map(([m, l]) =>
        el(`button${mode === m ? ".active" : ""}`, { dataset: { mode: m }, onclick: () => { mode = m; showMode(); } }, l),
      ),
    );
    const showMode = () => {
      tabs.querySelectorAll("button").forEach((b) => b.classList.toggle("active", b.dataset.mode === mode));
      drivePanel.hidden = mode !== "drive";
      folderPanel.hidden = mode !== "folder";
    };

    const save = async () => {
      if (!presenter && !name.value.trim()) return toast("Enter your name, so the others can see who changed what.", "warn");
      let r;
      if (mode === "drive") {
        if (!picked) return toast("Choose the parish folder in Google Drive first.", "warn");
        r = await call("drive_use_folder", { id: picked.id, name: picked.name, user: name.value.trim() });
      } else {
        r = await call("storage_set", { folder, name: name.value.trim() });
      }
      if (r) {
        info = r;
        await loadDrive();
        render();
        toast(
          mode === "drive"
            ? `Using “${picked.name}” in Google Drive. The first copy to this computer can take a few minutes.`
            : presenter
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
        tabs,
        drivePanel,
        folderPanel,
        presenter ? null : el("label.field", {}, el("span", {}, "Your name"), name),
        el("div.modal-actions", {}, welcome ? null : el("button", { onclick: () => done(null) }, "Cancel"), el("button.primary", { onclick: save }, welcome ? "Start" : "Save")),
      ),
    );
    document.body.append(overlay);
    showMode();
    loadDrive().then(() => {
      renderDrive();
      // Signed in with no folder yet: show the folders straight away.
      if (mode === "drive" && drive?.account && !picked) listFolders();
    });
    renderDrive();
    (welcome ? overlay.querySelector(".lib-modes button") : name).focus();
  });
}

export async function initStorage() {
  const b = document.getElementById("btn-library-folder");
  if (b) b.addEventListener("click", () => openLibrarySettings(false));
  listen("episco://storage", () => load());
  listen("episco://drive", () => loadDrive());
  await load();
  loadDrive();
  // First run of the planner: choose the library and enter a name.
  if (app.mode === "planner" && info && !info.name) await openLibrarySettings(true);
}
