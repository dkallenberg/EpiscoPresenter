// Editing a Library item on its own (Library → Edit, or ＋ New), without
// putting it in a service: the app opens it in the Editor as a service of
// one item (src-tauri/src/library_edit.rs). The top bar says "Library:" and
// has Save to Library and Done; Done goes back to the Library. After a save,
// the upcoming services that use the item can be given the new version.

import { app, call, el, toast } from "/js/control/util.js";
import { HYMNALS } from "/js/render.js";
import { flushEdits } from "/js/control/editor.js";

/** The open "service" is a Library item being edited. */
export const isLibraryEdit = (s = app.service) =>
  Boolean(s && !s.plan_path && s.cues?.length === 1 && s.cues[0].id === "library-item");

/** The Library item last saved from the editor, to show when Done. */
let lastSaved = null;

function showEditor() {
  document.getElementById("home")?.setAttribute("hidden", "");
  app.select("library-item");
  document.querySelector('.tabs button[data-tab="editor"]')?.click();
}

/** Opens one of the parish's Library items in the Editor. */
export async function editLibraryItem(id) {
  if ((await call("library_edit_open", { id })) === undefined) return;
  lastSaved = id;
  showEditor();
}

/** A Prayer Book item: a parish version of it, then that in the Editor. */
export async function editParishVersion(id) {
  const copy = await call("library_copy", { id });
  if (copy) await editLibraryItem(copy);
}

/** ＋ New in the Library: a hymn, a prayer or other text, a pasted text or
 *  hymns from files. */
export function newLibraryItem({ pasteText, importHymns }) {
  const close = () => overlay.remove();
  let kind = "hymn";
  const book = el("select", {}, HYMNALS.map(([k, n]) => el("option", { value: k }, n)));
  const number = el("input", { placeholder: "e.g. 436", inputMode: "numeric" });
  const title = el("input", { placeholder: "e.g. Lift Up Your Heads" });
  const types = (app.vocab?.categories ?? []).map((c) => c.key).filter((k) => !["hymn", "announcements", "media", "video"].includes(k));
  const type = el("select", {}, (types.length ? types : ["prayer"]).map((k) => el("option", { value: k, selected: k === "prayer" }, k.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase()))));
  const rite = el("select", {}, [["", "Both rites"], ["II", "Rite II"], ["I", "Rite I"]].map(([v, l]) => el("option", { value: v }, l)));
  const hymnFields = el("div.todo-hymn", {}, el("label.field", {}, el("span", {}, "Book"), book), el("label.field", {}, el("span", {}, "Hymn #"), number));
  const textFields = el("div.todo-two", {}, el("label.field", {}, el("span", {}, "Type"), type), el("label.field", {}, el("span", {}, "Rite"), rite));
  textFields.hidden = true;
  const tab = (key, label) =>
    el(`button${key === kind ? ".active" : ""}`, {
      dataset: { kind: key },
      onclick: () => {
        kind = key;
        seg.querySelectorAll("button").forEach((b) => b.classList.toggle("active", b.dataset.kind === key));
        hymnFields.hidden = key !== "hymn";
        textFields.hidden = key === "hymn";
        title.placeholder = key === "hymn" ? "e.g. Lift Up Your Heads" : "e.g. A Prayer for the Parish";
      },
    }, label);
  const seg = el("div.seg", {}, tab("hymn", "A hymn"), tab("text", "A prayer or other text"));
  const create = async () => {
    const t = title.value.trim();
    if (kind === "hymn" && !t && !number.value.trim()) return toast("Type the hymn's number or title.", "warn");
    if (kind !== "hymn" && !t) return toast("Give the text a title.", "warn");
    const item =
      kind === "hymn"
        ? { category: "hymn", title: t, hymn_number: number.value.trim() || null, hymnal: book.value }
        : { category: type.value, title: t, rite: rite.value || null };
    if ((await call("library_edit_open", { new: item })) === undefined) return;
    close();
    lastSaved = null;
    showEditor();
    toast(kind === "hymn" ? "Paste the hymn's words in Hymn words, then Save to Library." : "Type the text in the Editor, then Save to Library.");
  };
  [number, title].forEach((i) => i.addEventListener("keydown", (e) => e.key === "Enter" && create()));
  const overlay = el(
    "div.modal-overlay.todo-pop",
    { onmousedown: (e) => e.target === overlay && close() },
    el(
      "div.modal",
      {},
      el("h3", {}, "New Library item"),
      el("p.muted.small", {}, "Made here and saved in the Library, without putting it in a service."),
      seg,
      hymnFields,
      textFields,
      el("label.field", {}, el("span", {}, "Title"), title),
      el(
        "p.muted.small",
        {},
        "Or bring in texts you already have: ",
        el("button.link", { onclick: () => (close(), pasteText()) }, "Paste a text…"),
        " · ",
        el("button.link", { onclick: () => (close(), importHymns()) }, "Import hymns from files…"),
      ),
      el("div.modal-actions", {}, el("button", { onclick: close }, "Cancel"), el("button.primary", { onclick: create }, "Create and edit")),
    ),
  );
  document.body.append(overlay);
  number.focus();
}

const when = (s) => {
  if (!s.date) return "";
  const d = new Date(`${s.date}T12:00`);
  const day = d.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" });
  if (!s.time) return day;
  const [h, m] = s.time.split(":").map(Number);
  return `${day} · ${((h + 11) % 12) + 1}:${String(m).padStart(2, "0")} ${h < 12 ? "am" : "pm"}`;
};

/** After a save: offer the new version to the upcoming services using it. */
function offerUpdate(id, uses) {
  return new Promise((resolve) => {
    const boxes = [];
    const rows = uses.map((u) => {
      const box = el("input", { type: "checkbox", checked: !u.open_on, disabled: Boolean(u.open_on) });
      boxes.push([box, u]);
      return el(
        "label.todo-radio",
        {},
        box,
        el("span", {}, el("b", {}, u.title), ` · ${when(u)} · ${u.items.join(", ")}`, u.open_on ? el("span.warn-text", {}, ` · open on ${u.open_on}'s computer`) : ""),
      );
    });
    const close = (v) => (overlay.remove(), resolve(v));
    const update = async () => {
      const paths = boxes.filter(([b]) => b.checked).map(([, u]) => u.path);
      if (!paths.length) return close(0);
      const n = await call("library_update_services", { id, paths });
      if (n !== undefined) toast(`Updated in ${n} service${n === 1 ? "" : "s"}`);
      close(n ?? 0);
    };
    const overlay = el(
      "div.modal-overlay.todo-pop",
      {},
      el(
        "div.modal",
        {},
        el("h3", {}, `Update it in ${uses.length} upcoming service${uses.length === 1 ? "" : "s"}?`),
        el("p.muted.small", {}, "These services have their own copy of this item. Tick the ones that should have the version just saved; how each prints, and which verses are sung, stay as they are."),
        el("div.todo-radios", {}, ...rows),
        el("div.modal-actions", {}, el("button", { onclick: () => close(0) }, "Not now"), el("button.primary", { onclick: update }, "Update")),
      ),
    );
    document.body.append(overlay);
  });
}

/** Save to Library: the item's file, then the services that use it. */
export async function saveLibraryItem() {
  // Changes still being typed in the Editor go in first.
  await flushEdits();
  const r = await call("library_edit_save");
  if (!r) return false;
  lastSaved = r.id;
  toast("Saved in the Library");
  if (r.uses?.length) await offerUpdate(r.id, r.uses);
  return true;
}

/** Done: back to the Library (and the service set aside, if one was open). */
export async function doneLibraryItem() {
  await flushEdits();
  const s = app.service;
  if (s?.modified || s?.cues?.some((c) => c.modified)) {
    const save = await call("confirm", { title: "Save to the Library?", message: "This item has changes that aren't in the Library yet.", okLabel: "Save" });
    if (save && !(await saveLibraryItem())) return;
  }
  if ((await call("library_edit_done")) === undefined) return;
  document.getElementById("home")?.setAttribute("hidden", "");
  document.querySelector('.tabs button[data-tab="library"]')?.click();
  if (lastSaved) document.dispatchEvent(new CustomEvent("episco:library-show", { detail: lastSaved }));
}
