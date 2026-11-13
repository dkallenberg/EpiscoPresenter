// The Connection dialog (the link pill in the top bar): which part this
// computer plays — the sanctuary computer (master), the live-stream
// computer (slave) that follows it, or one computer for both — and how the
// two find each other. Saving restarts the app.

import { app, call, el, toast } from "/js/control/util.js";

const ROLES = [
  ["master", "Sanctuary computer", "Runs the service and the sanctuary screen. The live-stream computer follows it."],
  ["slave", "Live-stream computer", "Shows the live-stream overlay and follows the sanctuary computer: every slide changes on both at once."],
  ["standalone", "One computer for both", "Drives the sanctuary screen and the live stream from this computer's monitors."],
];

/** The host part of a sync URL (`ws://192.168.1.20:7878/ws` → `192.168.1.20`). */
export function hostOf(url) {
  const m = /^ws:\/\/([^/:]+)(?::(\d+))?/.exec(url ?? "");
  if (!m) return "";
  return m[2] && m[2] !== "7878" ? `${m[1]}:${m[2]}` : m[1];
}

export async function openConnection() {
  if (app.mode === "planner") return;
  const info = await call("node_info");
  if (!info) return;
  const cfg = info.config;
  let role = cfg.role;
  const master = el("input", { value: cfg.role === "slave" ? hostOf(cfg.master_url) : "", placeholder: "e.g. 192.168.1.20 or Sanctuary-Mac-mini.local" });
  const name = el("input", { value: cfg.node_name, placeholder: "e.g. Sanctuary Mac mini" });
  const here = [info.host, ...info.addresses].filter(Boolean);
  const detail = el("div.conn-detail");
  const drawDetail = () => {
    detail.replaceChildren(
      role === "slave"
        ? el(
            "div",
            {},
            el("label.field", {}, el("span", {}, "The sanctuary computer's address"), master),
            el("p.muted.small", {}, "Shown in this window on the sanctuary computer. Both computers must be on the same network."),
          )
        : el(
            "div",
            {},
            el("p", {}, role === "master" ? "On the live-stream computer, choose “Live-stream computer” and type this computer's address:" : "Tablets and other computers can reach this computer at:"),
            here.length
              ? el("ul.conn-addresses", {}, here.map((a) => el("li", {}, el("code", {}, a))))
              : el("p.warn-note.small", {}, "This computer is not on a network."),
            el("p.muted.small", {}, "The first time, macOS may ask whether EpiscoPresenter may accept incoming network connections: choose Allow. On a managed Mac, IT may need to allow it."),
          ),
    );
  };
  const choices = el(
    "div.conn-roles",
    {},
    ROLES.map(([v, label, help]) =>
      el(
        "label.conn-role",
        {},
        el("input", { type: "radio", name: "conn-role", value: v, checked: v === role, onchange: () => { role = v; drawDetail(); } }),
        el("span", {}, el("strong", {}, label), el("span.muted.small", {}, help)),
      ),
    ),
  );
  const link = app.snap?.link ?? {};
  const status = link.role === "slave" ? (link.online ? `Following ${link.peer ?? "the sanctuary computer"}` : `Not connected${link.error ? `: ${link.error}` : ""}`) : link.peer ? `Connected: ${link.peer}` : "";
  const close = () => overlay.remove();
  const save = async () => {
    const same = role === cfg.role && name.value.trim() === cfg.node_name && (role !== "slave" || hostOf(cfg.master_url) === master.value.trim());
    if (same) return close();
    if (role === "slave" && !master.value.trim()) return toast("Type the sanctuary computer's address.", "warn");
    const ok = await call("confirm", { title: "Restart EpiscoPresenter", message: "The app restarts to change how it connects.", okLabel: "Save and restart" });
    if (!ok) return;
    await call("node_settings_save", { role, master: master.value.trim() || null, name: name.value.trim() || null });
  };
  const overlay = el(
    "div.modal-overlay",
    { onmousedown: (e) => e.target === overlay && close() },
    el(
      "div.modal",
      {},
      el("h3", {}, "Connection"),
      status ? el("p.small", {}, status) : null,
      el("p.muted.small", {}, "This computer is the:"),
      choices,
      detail,
      el("label.field", {}, el("span", {}, "This computer's name (shown on the other one)"), name),
      el("div.modal-actions", {}, el("button", { onclick: close }, "Cancel"), el("button.primary", { onclick: save }, "Save")),
    ),
  );
  drawDetail();
  document.body.append(overlay);
}
