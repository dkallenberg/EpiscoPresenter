// To do: each line beside the bulletin opens a small pop-up in the middle of
// the window with only what that line needs (the hymn, the preacher, the
// Google Slides link …). Save fills it in and closes; the line then leaves
// the list. Everything else stays where it was, behind an "all …" link.

import { app, call, el, toast, HONORIFICS } from "/js/control/util.js";
import { assetUrl, invoke } from "/js/bridge.js";
import { HYMNALS } from "/js/render.js";
import { parseDeck } from "/js/deck.js";
import { load as loadDeck, slideImage } from "/js/deckpdf.js";
import { pickMusic } from "/js/control/media.js";
import { openParish, openReadings } from "/js/control/bulletin.js";

/**
 * A pop-up: `title`, a line saying what it is for, `body` (nodes), and
 * Cancel / Save. `save` resolves true to close. `more` is [label, action]
 * for the full screen behind it.
 */
function popup({ title, about, body, save, saveLabel = "Save", more = null, wide = false }) {
  const close = () => {
    overlay.remove();
    document.removeEventListener("keydown", keys, true);
  };
  const run = async () => {
    saveBtn.disabled = true;
    try {
      if ((await save()) !== false) close();
    } finally {
      saveBtn.disabled = false;
    }
  };
  const keys = (e) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      close();
    } else if (e.key === "Enter" && e.target.tagName === "INPUT" && e.target.type !== "checkbox" && !e.target.dataset.noEnter) {
      e.preventDefault();
      run();
    }
  };
  document.addEventListener("keydown", keys, true);
  const saveBtn = el("button.primary", { onclick: run }, saveLabel);
  const overlay = el(
    "div.modal-overlay.todo-pop",
    { onmousedown: (e) => e.target === overlay && close() },
    el(
      `div.modal${wide ? ".wide" : ""}`,
      {},
      el("h3", {}, title),
      about ? el("p.muted.small", {}, about) : null,
      ...body,
      el(
        "div.modal-actions",
        {},
        more ? el("button.link.todo-more", { onclick: () => (close(), more[1]()) }, more[0]) : null,
        el("button", { onclick: close }, "Cancel"),
        saveBtn,
      ),
    ),
  );
  document.body.append(overlay);
  overlay.querySelector("input:not([type=checkbox]):not([type=radio]), textarea, select")?.focus();
  return { close, overlay };
}

const field = (label, control) => el("label.field", {}, el("span", {}, label), control);

/** Opens the Editor on an item. */
function openEditor(id) {
  app.select(id);
  document.querySelector('.tabs button[data-tab="editor"]')?.click();
}

// ---------- the parish, the service time, the Google Slides ----------

const PAPERS = [
  ["tabloid", "11 × 17, folded (8½ × 11 pages)"],
  ["legal", "8½ × 14, folded (7 × 8½ pages)"],
  ["letter", "8½ × 11, folded (5½ × 8½ pages)"],
];

/** The parish's name, place, logo and paper: what the cover needs. */
export async function todoParish() {
  const p = (await call("parish_get")) ?? {};
  let logo = p.logo ?? null;
  const name = el("input", { value: p.name ?? "", placeholder: "e.g. St. James Episcopal Church" });
  const place = el("input", { value: p.place ?? "", placeholder: "e.g. Riverton" });
  const logoName = el("span.muted.small", {}, logo ? String(logo).split(/[\\/]/).pop() : "None yet");
  const paper = el("select", {}, PAPERS.map(([v, l]) => el("option", { value: v, selected: v === (p.paper ?? "tabloid") }, l)));
  popup({
    title: "Parish details",
    about: "What every bulletin's cover needs. Everyone in the parish shares these.",
    body: [
      el("div.todo-two", {}, field("Parish name", name), field("Place", place)),
      el("div.todo-two", {},
        el("div.field", {}, el("span", {}, "Logo (PNG or JPEG)"), el("div.todo-pick", {}, logoName, el("button.small", {
          onclick: async () => {
            const path = await call("parish_pick_logo");
            if (path) ((logo = path), (logoName.textContent = String(path).split(/[\\/]/).pop()));
          },
        }, "Choose…"))),
        field("Bulletin paper", paper),
      ),
    ],
    more: ["All parish details", () => openParish()],
    save: async () => {
      if (!name.value.trim()) return toast("Type the parish's name.", "warn"), false;
      const info = { ...p, name: name.value.trim(), place: place.value.trim(), paper: paper.value };
      if (logo) info.logo = logo;
      return (await call("parish_set", { info })) !== undefined;
    },
  });
}

/** The service time on the cover: typed, or one of the regular services. */
export async function todoTime() {
  const parish = (await call("parish_get")) ?? {};
  const time = el("input", { type: "time", value: app.service?.time ?? "" });
  const regular = (parish.services ?? []).filter((r) => r.time);
  const label = (r) => {
    const [h, m] = r.time.split(":").map(Number);
    const t = `${((h + 11) % 12) + 1}:${String(m).padStart(2, "0")} ${h < 12 ? "am" : "pm"}`;
    return [`${r.day[0].toUpperCase()}${r.day.slice(1)} ${t}`, r.name].filter(Boolean).join(" · ");
  };
  const pick = regular.length
    ? field("Or a regular service", el("select", { onchange: (e) => e.target.value && (time.value = e.target.value) },
        el("option", { value: "" }, "Choose…"),
        regular.map((r) => el("option", { value: r.time }, label(r)))))
    : null;
  popup({
    title: "Service time",
    about: "Printed on the bulletin's cover.",
    body: [el("div.todo-two", {}, field("Time", time), pick)],
    save: async () => {
      if (!time.value) return toast("Choose the time.", "warn"), false;
      return (await call("set_service_time", { time: time.value })) !== undefined;
    },
  });
}

/** The announcements' Google Slides link: checked by downloading it. */
export function todoDeck() {
  const link = el("input", { value: app.service?.announcements_deck ?? "", placeholder: "https://docs.google.com/presentation/d/…" });
  const thumb = el("img.todo-deck-thumb", { alt: "" });
  thumb.hidden = true;
  const status = el("p.small", {}, "Paste the deck's link (in Google Slides: Share → Copy link).");
  let checked = null;
  let seq = 0;
  const check = async () => {
    const url = link.value.trim();
    const n = ++seq;
    thumb.hidden = true;
    checked = null;
    const deck = parseDeck(url);
    if (!url) return void (status.textContent = "Paste the deck's link (in Google Slides: Share → Copy link).", (status.className = "small"));
    if (!deck) return void ((status.textContent = "This isn't a Google Slides link (docs.google.com/presentation/…)."), (status.className = "small warn-text"));
    if (deck.kind === "pub") {
      status.className = "small warn-text";
      status.textContent = "A “Publish to web” link can't be downloaded. In Google Slides choose Share → Copy link, and paste that.";
      return;
    }
    status.className = "small muted";
    status.textContent = "Checking the deck…";
    try {
      await invoke("deck_fetch", { url, refresh: true });
      const pages = await loadDeck(url, true);
      if (n !== seq) return;
      if (!pages) throw new Error("no slides");
      checked = url;
      status.className = "small ok-text";
      status.textContent = `✓ ${pages} slide${pages === 1 ? "" : "s"}. Both computers will download it.`;
      const src = await slideImage(url, 1, 320);
      if (n === seq && src) ((thumb.src = src), (thumb.hidden = false));
    } catch (e) {
      if (n !== seq) return;
      status.className = "small warn-text";
      status.textContent = /did not send|Share/.test(String(e))
        ? "Google didn't send the slides. In Google Slides choose Share → General access → “Anyone with the link” (Viewer), then paste the link again."
        : /could not reach/.test(String(e))
          ? "Google Slides couldn't be reached: check this computer's internet connection. The link can still be saved; the deck downloads when it can."
          : `The deck couldn't be checked: ${e}`;
    }
  };
  let timer = 0;
  link.addEventListener("input", () => (clearTimeout(timer), (timer = setTimeout(check, 400))));
  popup({
    title: "Announcements · Google Slides",
    about: "The deck shown before and after the service.",
    body: [field("Link to the deck", link), el("div.todo-deck", {}, thumb, status)],
    save: async () => {
      const url = link.value.trim();
      if (!parseDeck(url)) return toast("Paste a Google Slides link.", "warn"), false;
      const ok = (await call("set_announcements_deck", { url })) !== undefined;
      if (ok && checked !== url) toast("Saved. The deck couldn't be checked yet: it will download when it can.", "warn");
      return ok;
    },
  });
  if (link.value) check();
}

// ---------- items in the service ----------

/** The pop-up for an item's To do line. */
export function openTodo(c, info) {
  if (info?.needs_text) return todoReading(c);
  switch (c.category) {
    case "hymn":
      return todoHymn(c);
    case "homily":
      return todoPreacher(c);
    case "anthem":
      return todoAnthem(c);
    case "psalm":
      return todoPsalm(c);
    default:
      return openEditor(c.id);
  }
}

/** The hymn: found in the Library by number or title, with its verses
 *  sung; or a new one, in the Editor. */
async function todoHymn(c) {
  const place = c.overrides?.title ?? c.title;
  const hymns = ((await call("library_list")) ?? []).filter((e) => e.category === "hymn");
  const book = el("select", {}, HYMNALS.map(([k, name]) => el("option", { value: k }, name)));
  const number = el("input", { placeholder: "e.g. 436", inputMode: "numeric" });
  const title = el("input", { placeholder: "e.g. Lift up your heads" });
  const results = el("div.todo-results", { role: "listbox" });
  const versesRow = el("div.todo-verses");
  let chosen = null;
  let verses = [];
  const sung = new Set();

  const words = (t) => (t ?? "").toLowerCase().replace(/[^\p{L}\p{N}\s]/gu, " ").split(/\s+/).filter(Boolean);
  const find = () => {
    const n = number.value.trim();
    const q = words(title.value);
    const word = HYMNALS.find(([k]) => k === book.value)?.[2] ?? "Hymn";
    if (!n && !q.length) return [];
    return hymns
      .map((e) => {
        const [w, num] = (e.citation ?? "").split(" ");
        let score = 0;
        if (n) {
          if (num !== n) return null;
          score += w === word ? 2 : 1;
        }
        if (q.length) {
          const hay = words(`${e.title} ${e.summary ?? ""}`);
          if (!q.every((x) => hay.some((h) => h.startsWith(x)))) return null;
          score += 1;
        }
        return { e, score };
      })
      .filter(Boolean)
      .sort((a, b) => b.score - a.score || a.e.title.localeCompare(b.e.title))
      .slice(0, 8)
      .map((r) => r.e);
  };
  const choose = async (e) => {
    chosen = e;
    results.querySelectorAll(".todo-res").forEach((r) => r.classList.toggle("on", r.dataset.id === e?.id));
    versesRow.replaceChildren();
    verses = [];
    sung.clear();
    if (!e) return;
    const cue = await call("library_preview", { id: e.id });
    if (chosen !== e) return;
    const blocks = cue?.content?.document?.blocks ?? [];
    verses = [...new Set(blocks.filter((b) => b.kind === "paragraph" && /^\d+$/.test((b.role ?? "").trim())).map((b) => Number(b.role)))].sort((a, b) => a - b);
    verses.forEach((v) => sung.add(v));
    if (verses.length < 2) return;
    versesRow.append(
      el("span.image-field-label", {}, "Verses sung"),
      ...verses.map((v) =>
        el("label.check", {}, el("input", { type: "checkbox", checked: true, onchange: (ev) => (ev.target.checked ? sung.add(v) : sung.delete(v)) }), el("span", {}, String(v))),
      ),
    );
  };
  const addNew = async () => {
    const n = number.value.trim();
    const t = title.value.trim();
    // The place keeps its name; the hymn takes the number and title typed.
    await call("set_cue_overrides", { id: c.id, overrides: { ...c.overrides, title: place } });
    const doc = structuredClone(c.content.document);
    if (t) doc.meta.title = t;
    if (n) doc.meta.hymn_number = n;
    doc.meta.extra = (doc.meta.extra ?? []).filter(([k]) => k !== "hymnal");
    if (book.value !== "h82") doc.meta.extra.push(["hymnal", book.value]);
    await call("update_cue_document", { id: c.id, document: doc });
    pop.close();
    openEditor(c.id);
    toast("Paste the hymn's words in Hymn words, then Apply.");
  };
  const render = () => {
    const found = find();
    const typed = number.value.trim() || title.value.trim();
    const label = [number.value.trim() ? `${HYMNALS.find(([k]) => k === book.value)?.[2] ?? "Hymn"} ${number.value.trim()}` : "", title.value.trim()].filter(Boolean).join(" · ");
    results.replaceChildren(
      ...found.map((e) =>
        el("button.todo-res", { dataset: { id: e.id }, role: "option", onclick: () => choose(e), ondblclick: () => use() },
          el("span.todo-num", {}, e.citation ?? ""),
          el("span.todo-t", {}, e.title, el("i", {}, e.summary ?? "")))),
      typed
        ? el("button.todo-res.new", { onclick: addNew }, found.length ? "＋ None of these: add a new hymn…" : `＋ Add a new hymn${label ? `: ${label}` : ""}, not in the Library yet…`)
        : el("p.muted.small", {}, "Type the hymn's number, or words from its title or first line."),
    );
    // An exact number is picked for you.
    const exact = number.value.trim() && found.length && found[0].citation?.split(" ")[1] === number.value.trim() ? found[0] : null;
    if (exact && exact !== chosen) choose(exact);
    else if (chosen && !found.includes(chosen)) choose(null);
    else if (chosen) choose(chosen);
  };
  [number, title].forEach((i) => i.addEventListener("input", render));
  book.addEventListener("change", render);

  const use = async () => {
    if (!chosen) {
      if (number.value.trim() || title.value.trim()) toast("Choose a hymn from the list, or Add a new hymn.", "warn");
      else toast("Type the hymn's number or title.", "warn");
      return false;
    }
    const list = verses.filter((v) => sung.has(v));
    if (verses.length > 1 && !list.length) return toast("At least one verse is sung.", "warn"), false;
    const ok = await call("library_fill_cue", { id: chosen.id, cue: c.id, verses: list.length && list.length < verses.length ? list : null });
    if (ok === undefined) return false;
    toast(`${place}: ${chosen.citation ? `${chosen.citation} · ` : ""}${chosen.title}`);
    return true;
  };
  const pop = popup({
    title: `${place} · choose the hymn`,
    about: "Find it in the Library by its number, or by words from its title or first line.",
    body: [el("div.todo-hymn", {}, field("Book", book), field("Hymn #", number), field("Or its title", title)), results, versesRow],
    saveLabel: `Use for ${place}`,
    save: use,
    more: ["Open the full Editor", () => openEditor(c.id)],
    wide: true,
  });
  render();
}

/** A reading's text, pasted from Bible Gateway (or the parish's copy). */
function todoReading(c) {
  const citation = c.content?.document?.meta?.citation ?? "";
  const version = app.service?.bible ?? "NRSVue";
  const paste = el("textarea.reading-paste", { rows: 8, placeholder: `Paste ${citation} here. Verse numbers, headings, footnotes and the copyright line are taken out.` });
  popup({
    title: `${c.title} · add the text`,
    about: `${citation} · ${version}`,
    body: [
      el("div.todo-open", {}, el("button", { onclick: () => call("open_passage", { citation, version }) }, "Open on Bible Gateway ↗")),
      field("Paste the passage", paste),
      el("p.muted.small", {}, "With the parish's Bible file every reading fills itself in: All reading texts → Import Bible file…"),
    ],
    saveLabel: "Use pasted",
    more: ["All reading texts", () => openReadings(c.id)],
    save: async () => {
      if (!paste.value.trim()) return toast("Paste the reading first.", "warn"), false;
      return (await call("reading_set_text", { id: c.id, text: paste.value, version, words: 40 })) != null;
    },
  });
}

/** The preacher: one of the parish's preachers, or someone else. */
async function todoPreacher(c) {
  const parish = (await call("parish_get")) ?? {};
  const preachers = (parish.staff ?? []).filter((m) => m.preaches !== false && m.name?.trim());
  const extra = (k) => c.content.document.meta.extra?.find(([x]) => x === k)?.[1] ?? "";
  const honorific = el("input", { value: extra("preacher_honorific"), placeholder: "The Rev.", list: "todo-honorifics" });
  const name = el("input", { value: extra("preacher"), placeholder: "e.g. Jane Doe" });
  const job = el("input", { value: extra("preacher_title"), placeholder: "e.g. Bishop of Riverton" });
  const full = (m) => [m.honorific, m.name].filter((x) => x?.trim()).join(" ");
  const now = preachers.findIndex((m) => m.name.trim() === name.value.trim());
  let pick = now >= 0 ? String(now) : name.value ? "other" : preachers.length ? "" : "other";
  const other = el("div.todo-other", {}, el("div.todo-three", {}, field("Title", honorific), field("Name", name), field("Position", job)));
  const shows = el("p.muted.small");
  const person = () => (pick === "other" ? { honorific: honorific.value, name: name.value, title: job.value } : pick === "" ? null : preachers[Number(pick)] ?? null);
  const update = () => {
    other.hidden = pick !== "other";
    const p = person();
    shows.textContent = p?.name?.trim() ? `The bulletin prints: The Sermon · ${[full(p), p.title].filter(Boolean).join(", ")}` : "";
  };
  [honorific, name, job].forEach((i) => i.addEventListener("input", update));
  const radios = el(
    "div.todo-radios",
    { role: "radiogroup" },
    ...preachers.map((m, i) =>
      el("label.todo-radio", {}, el("input", { type: "radio", name: "todo-preacher", checked: pick === String(i), onchange: () => ((pick = String(i)), update()), ondblclick: () => pop.overlay.querySelector("button.primary").click() }), full(m), m.title ? el("span.muted", {}, ` · ${m.title}`) : null),
    ),
    el("label.todo-radio", {}, el("input", { type: "radio", name: "todo-preacher", checked: pick === "other", onchange: () => ((pick = "other"), update(), name.focus()) }), "Someone else"),
  );
  const pop = popup({
    title: "The Sermon · the preacher",
    about: preachers.length ? "The parish's preachers come from Parish details → Staff & preachers." : "Add the parish's preachers once in Parish details → Staff & preachers; they are then offered here.",
    body: [radios, other, shows, el("datalist", { id: "todo-honorifics" }, HONORIFICS.map((h) => el("option", { value: h })))],
    more: ["Open the full Editor", () => openEditor(c.id)],
    save: async () => {
      const p = person();
      if (!p?.name?.trim()) return toast("Choose the preacher, or type a name.", "warn"), false;
      return (await call("sermon_set_preacher", { id: c.id, honorific: p.honorific ?? "", name: p.name, title: p.title ?? "" })) !== undefined;
    },
  });
  update();
}

/** Past music, offered as a title is typed: anthems, preludes, postludes. */
let pastMusic = null;
export async function suggestions(kind) {
  pastMusic ??= call("music_suggestions").then((m) => m ?? {});
  return (await pastMusic)?.[kind] ?? [];
}
export const forgetSuggestions = () => (pastMusic = null);

/** Fills `composer` when `title` matches a piece used before, and offers
 *  those pieces as the title is typed. */
export async function suggestFor(kind, title, composer) {
  const list = await suggestions(kind);
  if (!list.length) return null;
  const id = `todo-past-${kind}`;
  title.setAttribute("list", id);
  title.addEventListener("change", () => {
    const m = list.find((p) => p.title.toLowerCase() === title.value.trim().toLowerCase());
    if (m && !composer.value.trim()) composer.value = m.composer;
  });
  return el("datalist", { id }, list.map((p) => el("option", { value: p.title, label: [p.composer, p.date].filter(Boolean).join(" · ") })));
}

/** A choir anthem's title and composer. */
async function todoAnthem(c) {
  const doc = c.content.document;
  const title = el("input", { value: doc.meta.title ?? "", placeholder: "e.g. Ave verum corpus" });
  const composer = el("input", { value: doc.meta.extra?.find(([k]) => k === "composer")?.[1] ?? "", placeholder: "e.g. W. A. Mozart, arr. by …" });
  const list = await suggestFor("anthems", title, composer);
  popup({
    title: `${c.title} · name the anthem`,
    about: "Printed in the bulletin and shown on the live stream. Anthems sung before are offered as you type.",
    body: [field("Title", title), field("Composer (and arranger)", composer), list],
    more: ["Open the full Editor", () => openEditor(c.id)],
    save: async () => {
      if (!title.value.trim() && !composer.value.trim()) return toast("Type the anthem's title.", "warn"), false;
      const ok = (await call("anthem_set", { id: c.id, title: title.value.trim(), composer: composer.value.trim() })) !== undefined;
      if (ok) forgetSuggestions();
      return ok;
    },
  });
}

/** The psalm's music: its antiphon and setting, or said with none. */
function todoPsalm(c) {
  const doc = c.content.document;
  const hasAntiphon = doc.blocks.some((b) => b.kind === "heading" && /^antiphon$/i.test(b.text ?? ""));
  let antiphonMusic = doc.blocks.find((b) => b.kind === "image" && b.src)?.src ?? null;
  let setting = doc.meta.extra?.find(([k]) => k === "setting_image")?.[1] ?? null;
  let said = false;
  const relativeTo = c.content.source ?? null;
  const words = hasAntiphon ? null : el("input", { placeholder: "e.g. Show us your mercy, O Lord, and grant us your salvation." });
  const picture = (label, kind, get, set) => {
    const img = el("img.score-thumb", { alt: "" });
    const show = () => {
      img.hidden = !get();
      if (get()) img.src = assetUrl(c, get());
      btn.textContent = get() ? "Change…" : "Choose…";
    };
    const btn = el("button.small", {
      onclick: async () => {
        const path = await pickMusic(kind, { relativeTo });
        if (path) (set(path), show());
      },
    });
    img.onerror = () => (img.hidden = true);
    const row = el("div.image-field", {}, el("span.image-field-label", {}, label), img, btn);
    show();
    return row;
  };
  const sung = el(
    "div.todo-sung",
    {},
    words ? field("Antiphon words (sung by all; may be left empty)", words) : null,
    picture("Antiphon music", "antiphon", () => antiphonMusic, (p) => (antiphonMusic = p)),
    picture("Musical setting", "setting", () => setting, (p) => (setting = p)),
    el("p.muted.small", {}, "The pictures come from the parish's music › Antiphon and music › Setting folders; Choose… can import new ones."),
  );
  const how = el(
    "div.todo-radios",
    { role: "radiogroup" },
    el("label.todo-radio", {}, el("input", { type: "radio", name: "todo-psalm", checked: true, onchange: () => ((said = false), (sung.hidden = false)) }), "Sung, with an antiphon or setting"),
    el("label.todo-radio", {}, el("input", { type: "radio", name: "todo-psalm", onchange: () => ((said = true), (sung.hidden = true)) }), "Said responsively, with no music"),
  );
  popup({
    title: `${c.title} · its music`,
    about: "How the psalm is sung, for the bulletin, the script and the screens.",
    body: [how, sung],
    more: ["Open the full Psalm layout", () => openEditor(c.id)],
    save: async () => {
      const antiphon = words?.value.trim() || null;
      if (!said && !antiphonMusic && !setting) return toast("Choose the antiphon's music or the setting, or choose Said responsively.", "warn"), false;
      return (await call("psalm_set_music", { id: c.id, music: { said, antiphon, antiphon_music: antiphonMusic, setting } })) !== undefined;
    },
  });
}
