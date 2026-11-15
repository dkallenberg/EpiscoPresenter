// The prelude: its title and composer, from the planner's service list, the
// bulletin, or Service settings. It heads the bulletin and script and shows
// on the live stream's welcome before the service.

import { ask } from "/js/control/made.js";
import { app, call, el, toast } from "/js/control/util.js";

/** "Adagio, Johann Sebastian Bach" (or nothing yet). */
export function preludeText(p = app.service?.prelude) {
  if (!p || (!p.title && !p.composer)) return "";
  return [p.title, p.composer].filter(Boolean).join(", ");
}

/** Asks for the prelude's title and composer. */
export async function editPrelude() {
  if (!app.service || app.readonly) return;
  const p = app.service.prelude ?? {};
  const r = await ask(
    "Prelude",
    [
      ["Title", "e.g. Adagio", p.title ?? ""],
      ["Composer (and arranger)", "e.g. Johann Sebastian Bach, arr. by Homer Whitfield", p.composer ?? ""],
    ],
    "Save",
    "At the top of the bulletin and script, and on the live stream's welcome before the service. Leave both empty for no prelude.",
  );
  if (!r) return;
  const ok = await call("set_prelude", { title: r[0], composer: r[1] });
  if (ok !== undefined) toast(r[0] || r[1] ? "Prelude saved" : "No prelude");
}

/** The planner's service list: the prelude, above the first item. */
export function preludeRow() {
  const text = preludeText();
  return el(
    "li.cue.prelude-row",
    { title: "The prelude: at the top of the bulletin, and on the live stream's welcome. Click to change it.", onclick: editPrelude },
    el("span.cue-num", {}, "–"),
    el(
      "div.cue-body",
      {},
      el("div.cue-title", {}, el("span.cue-icon", {}, "♪"), "Prelude"),
      el("div.cue-meta.muted.small", {}, text || "Add the title and composer"),
    ),
  );
}
