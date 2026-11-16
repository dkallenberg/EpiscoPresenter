// The media library (Library tab → Media): images and videos kept in the
// app's media folder. Add one to the service as a full-screen item, make it
// the background of the selected item, or set it as a psalm's musical
// setting. `pickMedia()` is the chooser used by the editor's ♪ Music blocks.

import { app, call, el, toast } from "/js/control/util.js";

const media = { files: [], filter: "all", query: "", tag: null, selected: null };

/** Every tag in the library, A to Z. */
const allTags = () => [...new Set(media.files.flatMap((f) => f.tags ?? []))].sort((a, b) => a.localeCompare(b));

/** A file matches words typed in its name or its tags. */
const matchesQuery = (f, q) => {
  q = q.trim().toLowerCase();
  return !q || f.name.toLowerCase().includes(q) || (f.tags ?? []).some((t) => t.toLowerCase().includes(q));
};

/** Tag chips to narrow a list; `onPick(tag|null)`. */
function tagChips(current, onPick) {
  const tags = allTags();
  if (!tags.length) return null;
  return el(
    "div.chips.media-tags",
    {},
    el("span.muted.small", {}, "Tags:"),
    tags.map((t) => el(`button.chip${current === t ? ".active" : ""}`, { onclick: () => onPick(current === t ? null : t) }, `#${t}`)),
  );
}

const url = (path) => (window.__TAURI__?.core?.convertFileSrc ? window.__TAURI__.core.convertFileSrc(path) : path);
const isVideo = (f) => f.kind === "video";
const size = (n) => (n > 1e6 ? `${(n / 1e6).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1e3))} KB`);

function thumb(f, still = true) {
  if (isVideo(f)) {
    const v = el("video", { muted: true, preload: "metadata", playsInline: true });
    v.src = still ? `${url(f.path)}#t=0.5` : url(f.path);
    return v;
  }
  return el("img", { src: url(f.path), alt: f.name, loading: "lazy" });
}

async function load() {
  media.files = (await call("media_list")) ?? [];
}

async function importFiles() {
  const added = await call("media_import");
  if (!added?.length) return null;
  await load();
  toast(`Added ${added.length} file${added.length === 1 ? "" : "s"} to the media library`);
  return added;
}

const shown = () =>
  media.files.filter(
    (f) =>
      (media.filter === "all" || f.kind === media.filter) &&
      (!media.tag || (f.tags ?? []).includes(media.tag)) &&
      matchesQuery(f, media.query),
  );

/** The Media view inside the Library tab. `toolbar` holds the Items/Media switch. */
export async function renderMedia(root, toolbar) {
  await load();
  draw(root, toolbar);
}

function draw(root, toolbar) {
  const files = shown();
  if (!files.some((f) => f.path === media.selected)) media.selected = files[0]?.path ?? null;
  const chips = el(
    "div.chips",
    {},
    [["all", "All"], ["image", "Images"], ["video", "Videos"]].map(([k, l]) =>
      el(`button.chip${media.filter === k ? ".active" : ""}`, { onclick: () => { media.filter = k; draw(root, toolbar); } }, l),
    ),
  );
  const search = el("input.lib-search", {
    type: "search",
    placeholder: "Search names and tags…",
    value: media.query,
    oninput: (e) => {
      media.query = e.target.value;
      draw(root, toolbar);
      const s = root.querySelector(".lib-search");
      s.focus();
      s.setSelectionRange(s.value.length, s.value.length);
    },
  });
  const grid = el(
    "div.media-grid",
    {},
    files.length
      ? files.map((f) =>
          el(
            `div.media-tile${f.path === media.selected ? ".selected" : ""}`,
            {
              title: f.name,
              onclick: () => { media.selected = f.path; draw(root, toolbar); },
              ondblclick: () => addToService([f]),
            },
            el("div.media-thumb", {}, thumb(f), isVideo(f) ? el("span.media-kind", {}, "▶") : null),
            el("div.media-name", {}, f.name),
            f.tags?.length ? el("div.media-tile-tags", {}, f.tags.map((t) => `#${t}`).join(" ")) : null,
          ),
        )
      : el(
          "p.muted",
          {},
          media.files.length ? "Nothing matches." : "No media yet. Use Import… to add images (music lines, slides, photos) and videos.",
        ),
  );
  root.replaceChildren(
    el(
      "div.lib-side",
      {},
      el(
        "div.lib-toolbar",
        {},
        toolbar,
        search,
        el("button.small", { onclick: async () => { const a = await importFiles(); if (a) { media.selected = a[0].path; draw(root, toolbar); } } }, "Import…"),
      ),
      chips,
      tagChips(media.tag, (t) => { media.tag = t; draw(root, toolbar); }),
      grid,
      el("p.hint.muted", {}, "Double-click to add as its own item (full screen). Files are copied into the app’s media folder."),
    ),
    preview(root, toolbar),
  );
}

function preview(root, toolbar) {
  const f = media.files.find((x) => x.path === media.selected);
  if (!f) return el("div.lib-preview");
  const cue = app.selectedCue();
  const big = thumb(f, false);
  if (isVideo(f)) Object.assign(big, { controls: true });
  const setBg = async (outputs) => {
    if (!cue) return toast("Select an item in the timeline first.", "warn");
    for (const output of outputs) {
      const look = { ...(cue.overrides?.looks?.[output] ?? {}), background: f.path };
      await call("set_cue_look", { id: cue.id, output, look });
    }
    toast(`Background of “${cue.title}” set`);
  };
  const name = el("input", { value: f.name, disabled: app.readonly });
  const rename = async () => {
    const v = name.value.trim();
    if (!v || v === f.name) return;
    const p = await call("media_rename", { path: f.path, name: v });
    if (p) {
      media.selected = p;
      await load();
      draw(root, toolbar);
    }
  };
  name.addEventListener("change", rename);
  return el(
    "div.lib-preview.media-preview",
    {},
    el("div.media-big", {}, big),
    el("div.media-info", {}, name, el("span.muted.small", {}, `${isVideo(f) ? "Video" : "Image"} · ${size(f.size)}`)),
    tagEditor(f, root, toolbar),
    el(
      "div.media-actions",
      {},
      el("button.primary", { disabled: app.readonly || !app.service, title: "Full screen on the sanctuary and the stream", onclick: () => addToService([f]) }, "Add to service"),
      el("button", { disabled: app.readonly || !app.service, title: "Full screen on the sanctuary only; the stream shows the camera", onclick: () => addToService([f], true) }, "Add (sanctuary only)"),
      el("span.muted.small", {}, cue ? `Background of “${cue.title}”:` : "Select an item to use this as its background."),
      el("button.small", { disabled: app.readonly || !cue, onclick: () => setBg(["sanctuary"]) }, "Sanctuary"),
      el("button.small", { disabled: app.readonly || !cue, onclick: () => setBg(["stream"]) }, "Live Stream"),
      el("button.small", { disabled: app.readonly || !cue, onclick: () => setBg(["sanctuary", "stream"]) }, "Both"),
      cue?.category === "psalm" && !isVideo(f)
        ? el("button.small", { disabled: app.readonly, onclick: () => setPsalmSetting(cue, f.path) }, "Psalm’s musical setting")
        : null,
      el("span.spacer"),
      el("button.small.danger-btn", { disabled: app.readonly, onclick: () => remove(f, root, toolbar) }, "Delete"),
    ),
    isVideo(f) ? el("p.muted.small", {}, "Videos play on the screens when their item is live, with sound from the sanctuary computer.") : null,
  );
}

/** The selected file's tags: remove one with ✕, add with Enter (several at
 *  once with commas). */
function tagEditor(f, root, toolbar) {
  const save = async (tags) => {
    const saved = await call("media_set_tags", { path: f.path, tags });
    if (saved === undefined) return;
    f.tags = saved;
    draw(root, toolbar);
    root.querySelector(".tag-input")?.focus();
  };
  const input = el("input.tag-input", {
    placeholder: f.tags?.length ? "Add a tag" : "Add tags, e.g. Advent, music, Christmas pageant",
    disabled: app.readonly,
    list: "media-tag-list",
    onkeydown: (e) => {
      if (e.key !== "Enter" || !input.value.trim()) return;
      e.preventDefault();
      save([...(f.tags ?? []), ...input.value.split(",").map((t) => t.trim()).filter(Boolean)]);
    },
  });
  return el(
    "div.media-tag-editor",
    {},
    (f.tags ?? []).map((t) =>
      el(
        "span.media-tag",
        {},
        `#${t}`,
        app.readonly ? null : el("button.tag-x", { title: `Remove #${t}`, onclick: () => save(f.tags.filter((x) => x !== t)) }, "✕"),
      ),
    ),
    input,
    el("datalist", { id: "media-tag-list" }, allTags().map((t) => el("option", { value: t }))),
  );
}

async function addToService(files, sanctuaryOnly = false) {
  if (!app.service) return toast("Open a service first (☰ Services).", "warn");
  const i = app.selectedIndex();
  const ids = await call("media_add", { paths: files.map((f) => f.path), after: i >= 0 ? i : null, sanctuaryOnly });
  if (ids?.length) {
    app.select(ids[0]);
    toast(`Added “${files[0].name}” to the service${sanctuaryOnly ? " (sanctuary only)" : ""}`);
  }
}

/** Sets a psalm's musical-setting image (shown on the stream between the
 *  gold title box and the text bar). */
export async function setPsalmSetting(cue, path) {
  const doc = structuredClone(cue.content.document);
  doc.meta.extra = (doc.meta.extra ?? []).filter(([k]) => k !== "setting_image");
  if (path) doc.meta.extra.push(["setting_image", path]);
  await call("update_cue_document", { id: cue.id, document: doc });
  toast(path ? "Musical setting set" : "Musical setting removed");
}

async function remove(f, root, toolbar) {
  const ok = await call("confirm", { title: "Delete media", message: `Delete “${f.name}” from the media library? Items that use it will show nothing in its place.`, okLabel: "Delete" });
  if (!ok) return;
  await call("media_delete", { path: f.path });
  await load();
  draw(root, toolbar);
}

/**
 * A chooser over the media library. Resolves to a file path, or null.
 * `kind`: "image" | "video" | undefined (both).
 */
export function pickMedia({ kind, relativeTo = null, title = "Choose from the media library" } = {}) {
  return new Promise((resolve) => {
    let done = false;
    const finish = (v) => {
      if (done) return;
      done = true;
      overlay.remove();
      resolve(v);
    };
    const grid = el("div.media-grid.picker");
    let tag = null;
    const search = el("input.lib-search", { type: "search", placeholder: "Search names and tags…", oninput: () => fill() });
    const chipsHost = el("div");
    const fill = () => {
      chipsHost.replaceChildren(tagChips(tag, (t) => { tag = t; fill(); }) ?? "");
      const files = media.files.filter((f) => (!kind || f.kind === kind) && (!tag || (f.tags ?? []).includes(tag)) && matchesQuery(f, search.value));
      grid.replaceChildren(
        ...(files.length
          ? files.map((f) =>
              el(
                "div.media-tile",
                { title: f.name, onclick: () => finish(f.path) },
                el("div.media-thumb", {}, thumb(f)),
                el("div.media-name", {}, f.name),
                f.tags?.length ? el("div.media-tile-tags", {}, f.tags.map((t) => `#${t}`).join(" ")) : null,
              ),
            )
          : [el("p.muted", {}, media.files.length ? "Nothing matches." : "The media library is empty. Import an image first.")]),
      );
    };
    const overlay = el(
      "div.modal-overlay",
      { onmousedown: (e) => e.target === overlay && finish(null) },
      el(
        "div.modal.wide",
        {},
        el("h3", {}, title),
        search,
        chipsHost,
        grid,
        el(
          "div.modal-actions",
          {},
          el("button", { onclick: async () => { const a = await importFiles(); if (a) fill(); } }, "Import…"),
          el("button", { onclick: async () => finish(await call("pick_image", { relativeTo })) }, "Other file…"),
          el("span.spacer"),
          el("button", { onclick: () => finish(null) }, "Cancel"),
        ),
      ),
    );
    document.body.append(overlay);
    load().then(fill);
  });
}

/** The music folders in the parish library: pictures of music lines. */
const MUSIC = {
  antiphon: { folder: "Antiphon", what: "antiphon music" },
  setting: { folder: "Setting", what: "musical setting" },
  gospel: { folder: "Gospel Acclamation", what: "Gospel Acclamation" },
  hymn: { folder: "Hymn", what: "hymn" },
};

/**
 * A chooser over one music folder (`antiphon`, `setting` or `gospel`):
 * pick a picture there, add new ones to it, or take one from the media
 * library. Resolves to a file path, or null.
 */
export function pickMusic(kind, { relativeTo = null } = {}) {
  const m = MUSIC[kind];
  return new Promise((resolve) => {
    let done = false;
    let files = [];
    const finish = (v) => {
      if (done) return;
      done = true;
      overlay.remove();
      resolve(v);
    };
    const grid = el("div.media-grid.picker");
    const search = el("input.lib-search", { type: "search", placeholder: "Search names…", oninput: () => fill() });
    const fill = () => {
      const shown = files.filter((f) => matchesQuery(f, search.value));
      grid.replaceChildren(
        ...(shown.length
          ? shown.map((f) =>
              el(
                "div.media-tile",
                { title: f.name, onclick: () => finish(f.path) },
                el("div.media-thumb", {}, thumb(f)),
                el("div.media-name", {}, f.name),
              ),
            )
          : [el("p.muted", {}, files.length ? "Nothing matches." : `The ${m.folder} folder is empty. Add the ${m.what} with Import….`)]),
      );
    };
    const load = async () => {
      files = (await call("music_list", { kind })) ?? [];
      fill();
    };
    const overlay = el(
      "div.modal-overlay",
      { onmousedown: (e) => e.target === overlay && finish(null) },
      el(
        "div.modal.wide",
        {},
        el("h3", {}, `Choose the ${m.what}`),
        el("p.muted.small", {}, `From the parish library's music › ${m.folder} folder.`),
        search,
        grid,
        el(
          "div.modal-actions",
          {},
          el("button", { onclick: async () => { const a = await call("music_import", { kind }); if (a?.length) { toast(`Added to ${m.folder}`); await load(); } } }, "Import…"),
          el("button", { title: "Pictures added to the media library before these folders", onclick: async () => { overlay.hidden = true; const p = await pickMedia({ kind: "image", relativeTo, title: "Choose from the media library" }); overlay.hidden = false; if (p) finish(p); } }, "From the media library…"),
          el("span.spacer"),
          el("button", { onclick: () => finish(null) }, "Cancel"),
        ),
      ),
    );
    document.body.append(overlay);
    load();
  });
}

