// Undo and Redo for the open service (undo.rs): ↶ ↷ in the Editor (and the
// bulletin's toolbar), ⌘Z and ⇧⌘Z (Ctrl on Windows; Ctrl+Y redoes too).
// Typing in a text box (a title, a search) keeps the box's own undo; in the
// Editor's text rows, which are saved as you type, ⌘Z undoes the service.

import { listen } from "/js/bridge.js";
import { flushEdits, reloadEditor } from "/js/control/editor.js";
import { app, call, el, toast } from "/js/control/util.js";

let status = { undo: 0, redo: 0 };

function render() {
  for (const b of document.querySelectorAll(".undo-btn")) b.disabled = app.readonly || !status.undo;
  for (const b of document.querySelectorAll(".redo-btn")) b.disabled = app.readonly || !status.redo;
}

/** Undoes (or with `again`, redoes) the last change to the open service. */
export async function undo(again = false) {
  if (app.readonly || !app.service) return;
  if (!(again ? status.redo : status.undo)) return toast(again ? "Nothing to redo" : "Nothing to undo");
  // What is still being typed counts as the last change.
  await flushEdits();
  const r = await call(again ? "redo" : "undo");
  if (!r) return;
  status = r;
  render();
  reloadEditor();
}

/** The two buttons, for a toolbar. */
export function undoButtons() {
  const b = [
    el("button.small.undo-btn", { title: "Undo the last change (⌘Z)", onclick: () => undo(false) }, "↶ Undo"),
    el("button.small.redo-btn", { title: "Redo (⇧⌘Z)", onclick: () => undo(true) }, "↷ Redo"),
  ];
  queueMicrotask(render);
  return b;
}

export function initUndo() {
  listen("episco://undo", (s) => {
    status = s ?? status;
    render();
  });
  app.on((what) => what === "service" && render());
  call("undo_status").then((s) => {
    if (s) status = s;
    render();
  });
  document.addEventListener(
    "keydown",
    (e) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      const key = e.key.toLowerCase();
      const redo = (key === "z" && e.shiftKey) || (key === "y" && e.ctrlKey && !e.metaKey);
      if (key !== "z" && !redo) return;
      // A text box keeps its own undo; so does a bulletin line being typed.
      if (e.target.closest?.("input, textarea, select, .doc-view [contenteditable='true']")) return;
      if (document.querySelector(".modal-overlay, .layout-tool")) return;
      e.preventDefault();
      undo(redo);
    },
    true,
  );
}
