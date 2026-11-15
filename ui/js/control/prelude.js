// The prelude and postlude: their title and composer, and how the bulletin
// shows them, from the planner's service list, the bulletin, or Service
// settings. The prelude heads the bulletin and script and shows on the live
// stream's welcome before the service; the postlude is the bulletin's and
// script's last line (not on the stream).

import { app, call, el, toast } from "/js/control/util.js";

const LABEL = { prelude: "Prelude", postlude: "Postlude" };

/** The bulletin's choices for the piece: [key, label, tip]. */
const CHOICES = [
  ["reference", "Title and composer", "“Prelude  Adagio … Johann Sebastian Bach”"],
  ["title", "Name only", "Just “Prelude”, without the piece"],
  ["hidden", "Left out", "Not in the bulletin (the script still names it)"],
];

const music = (which) => app.service?.[which] ?? null;

/** How the bulletin shows it: "reference", "title" or "hidden". */
export function musicStyle(which) {
  return music(which)?.bulletin ?? "reference";
}

/** "Adagio, Johann Sebastian Bach" (or nothing yet). */
export function musicText(which) {
  const p = music(which);
  if (!p || (!p.title && !p.composer)) return "";
  return [p.title, p.composer].filter(Boolean).join(", ");
}

/** Still to name: no title or composer, and the bulletin would show it. */
export function needsNaming(which) {
  return !musicText(which) && musicStyle(which) === "reference";
}

export const preludeText = () => musicText("prelude");

/** Asks for the piece's title and composer and how the bulletin shows it. */
export function editMusic(which) {
  if (!app.service || app.readonly) return;
  const p = music(which) ?? {};
  const label = LABEL[which];
  let style = musicStyle(which);
  const title = el("input", { value: p.title ?? "", placeholder: which === "prelude" ? "e.g. Adagio" : "e.g. Toccata (Symphony No. 5)" });
  const composer = el("input", {
    value: p.composer ?? "",
    placeholder: which === "prelude" ? "e.g. Johann Sebastian Bach, arr. by Homer Whitfield" : "e.g. Charles-Marie Widor",
  });
  const seg = el(
    "div.seg.music-bulletin",
    {},
    CHOICES.map(([key, text, tip]) =>
      el(`button${key === style ? ".active" : ""}`, {
        dataset: { style: key },
        title: tip,
        onclick: () => {
          style = key;
          seg.querySelectorAll("button").forEach((b) => b.classList.toggle("active", b.dataset.style === key));
        },
      }, text),
    ),
  );
  const close = () => overlay.remove();
  const save = async () => {
    close();
    const ok = await call(`set_${which}`, { title: title.value.trim(), composer: composer.value.trim(), bulletin: style });
    if (ok === undefined) return;
    toast(style === "hidden" ? `${label} left out of the bulletin` : title.value.trim() || composer.value.trim() ? `${label} saved` : `No ${label.toLowerCase()} named`);
  };
  [title, composer].forEach((i) => i.addEventListener("keydown", (e) => e.key === "Enter" && save()));
  const where =
    which === "prelude"
      ? "At the top of the bulletin and script, and on the live stream's welcome before the service."
      : "The last line of the bulletin and script (not on the live stream).";
  const overlay = el(
    "div.modal-overlay",
    { onmousedown: (e) => e.target === overlay && close() },
    el(
      "div.modal",
      {},
      el("h3", {}, label),
      el("p.muted.small", {}, `${where} Leave both empty for none.`),
      el("label.field", {}, el("span", {}, "Title"), title),
      el("label.field", {}, el("span", {}, "Composer (and arranger)"), composer),
      el("div.field", {}, el("span", {}, "In the bulletin"), seg),
      el("div.modal-actions", {}, el("button", { onclick: close }, "Cancel"), el("button.primary", { onclick: save }, "Save")),
    ),
  );
  document.body.append(overlay);
  title.focus();
}

export const editPrelude = () => editMusic("prelude");

/** The planner's service list: the prelude above the first item, the
 *  postlude below the last. */
export function musicRow(which) {
  const text = musicText(which);
  const label = LABEL[which];
  const style = musicStyle(which);
  const where = which === "prelude" ? "at the top of the bulletin, and on the live stream's welcome" : "the bulletin's last line";
  return el(
    `li.cue.prelude-row.${which}-row`,
    { title: `The ${label.toLowerCase()}: ${where}. Click to change it.`, onclick: () => editMusic(which) },
    el("span.cue-num", {}, "–"),
    el(
      "div.cue-body",
      {},
      el("div.cue-title", {}, el("span.cue-icon", {}, "♪"), label),
      el(
        "div.cue-meta.muted.small",
        {},
        text || (style === "reference" ? "Add the title and composer" : ""),
        style === "hidden" ? `${text ? " · " : ""}left out of the bulletin` : style === "title" && text ? " · bulletin: name only" : "",
      ),
    ),
  );
}

export const preludeRow = () => musicRow("prelude");
