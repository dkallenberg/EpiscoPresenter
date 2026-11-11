// Top bar → Outputs: open this node's output windows (sanctuary and/or
// stream) on demand, full screen on a monitor (by default the second
// monitor) or as an ordinary window, and close them again.

import { listen } from "/js/bridge.js";
import { call, el } from "/js/control/util.js";

let panel = null;
let status = null;

const button = () => document.getElementById("btn-outputs");

const remember = (k, v) => { try { localStorage.setItem(k, v); } catch {} };
const recall = (k) => { try { return localStorage.getItem(k); } catch { return null; } };

async function refresh() {
  status = (await call("outputs_status")) ?? status;
  if (!status) return;
  const open = status.outputs.filter((o) => o.open).length;
  const b = button();
  b.textContent = `▣ Outputs ${open}/${status.outputs.length}`;
  b.classList.toggle("outputs-off", open === 0);
  b.title = open ? status.outputs.map((o) => `${o.title}: ${describe(o)}`).join("\n") : "The output screens are closed. Click to open them.";
  if (panel) draw();
}

function monitorName(i) {
  const m = status.monitors[i];
  if (!m) return `Monitor ${i + 1}`;
  return `${i + 1}: ${m.name} (${m.width}×${m.height})${m.control ? " · this screen" : ""}`;
}

function describe(o) {
  if (!o.open) return "closed";
  const where = o.monitor != null ? ` on monitor ${o.monitor + 1}` : "";
  return o.fullscreen ? `full screen${where}` : `window${where}`;
}

function draw() {
  const rows = status.outputs.map((o) => {
    const key = `outputs.monitor.${o.output}`;
    const saved = Number(recall(key));
    const initial = o.open && o.monitor != null ? o.monitor : Number.isInteger(saved) && recall(key) != null && saved < status.monitors.length ? saved : o.default_monitor;
    const monitor = el(
      "select",
      { onchange: (e) => remember(key, e.target.value) },
      status.monitors.map((m) => el("option", { value: m.index, selected: m.index === initial }, monitorName(m.index))),
    );
    const show = (fullscreen) => call("output_show", { output: o.output, fullscreen, monitor: Number(monitor.value) }).then(refresh);
    const single = status.monitors.length < 2;
    return el(
      "div.out-row",
      {},
      el("div.out-name", {}, el("strong", {}, o.title), el(`span.out-state${o.open ? ".on" : ""}`, {}, describe(o))),
      el("label.field", {}, el("span", {}, "Monitor"), monitor),
      el(
        "div.out-actions",
        {},
        el("button.primary", { title: single ? "Only one monitor: full screen covers this window (Esc in the output leaves full screen)" : "Full screen on the chosen monitor", onclick: () => show(true) }, "Full screen"),
        el("button", { title: "An ordinary window you can move and resize", onclick: () => show(false) }, "Window"),
        el("button", { disabled: !o.open, onclick: () => call("output_close", { output: o.output }).then(refresh) }, "Close"),
      ),
    );
  });
  panel.replaceChildren(
    el("div.out-head", {}, el("strong", {}, "Output screens"), el("span.spacer"), el("button.icon", { onclick: close, title: "Close" }, "✕")),
    ...rows,
    el(
      "p.muted.small",
      {},
      status.monitors.length < 2
        ? "Only one monitor is connected: open the outputs as windows, or connect the projector / TV and choose it here."
        : "Full screen goes on the second monitor unless you choose another. In an output: Esc leaves full screen, double-click toggles it.",
    ),
  );
}

function close() {
  panel?.remove();
  panel = null;
}

async function toggle() {
  if (panel) return close();
  panel = el("div.outputs-panel");
  document.body.append(panel);
  const r = button().getBoundingClientRect();
  panel.style.top = `${r.bottom + 6}px`;
  panel.style.left = `${Math.max(8, Math.min(r.left, innerWidth - 560))}px`;
  panel.append(el("p.muted", {}, "Looking for monitors…"));
  await refresh();
}

export function initOutputs() {
  const b = button();
  if (!b) return;
  b.addEventListener("click", (e) => {
    e.stopPropagation();
    toggle();
  });
  document.addEventListener("mousedown", (e) => {
    if (panel && !panel.contains(e.target) && e.target !== b) close();
  });
  document.addEventListener("keydown", (e) => e.key === "Escape" && close());
  listen("episco://outputs", () => refresh());
  refresh();
}
