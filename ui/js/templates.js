// Slide layouts: where each piece of a slide goes and how its text looks.
//
// Every slide type has named elements, each a rectangle on the 1920x1080
// stage (x, y, w, h) with a font, size, colour and alignment. The defaults
// below match the parish slide designs (docs/backgrounds.md); the operator's
// changes from the Layout tool arrive in `service.templates` and are laid
// over them. Colours left empty follow the look's text tone (light / dark).

export const SANCTUARY_FONT = 'Tahoma, Verdana, "DejaVu Sans", sans-serif';
export const STREAM_FONT = 'Calibri, Carlito, "Segoe UI", "Open Sans", "Helvetica Neue", Arial, sans-serif';

/** Font choices offered in the Layout tool (any CSS family can be typed). */
export const FONTS = [
  ["", "Default"],
  [SANCTUARY_FONT, "Tahoma"],
  [STREAM_FONT, "Calibri"],
  ['Arial, Helvetica, "Liberation Sans", sans-serif', "Arial"],
  ['"Gill Sans", "Gill Sans MT", Calibri, sans-serif', "Gill Sans"],
  ['Optima, Candara, "Segoe UI", sans-serif', "Optima"],
  ['Georgia, "DejaVu Serif", serif', "Georgia"],
  ['Garamond, "EB Garamond", "Times New Roman", serif', "Garamond"],
  ['"Times New Roman", Times, "Liberation Serif", serif', "Times New Roman"],
  ['Palatino, "Palatino Linotype", "Book Antiqua", serif', "Palatino"],
];

const labels = (size, extra = {}) => ({ label: "Speaker labels & verse numbers", kind: "font", size, italic: true, ...extra });
const people = { label: "People (bold) lines", kind: "font" };

/** Built-in slide types, in Layout-tool order. */
export const SLIDE_TYPES = [
  {
    key: "stream.left_panel",
    output: "stream",
    label: "Left side",
    hint: "Prayers and dialogues: text in the panel on the left.",
    layout: "left_panel",
    background: "builtin:panel-light",
    sample: "prayer",
    elements: {
      title: { label: "Title", x: 74, y: 22, w: 800, h: 72, size: 60, bold: true, valign: "center" },
      reference: { label: "Reference (BCP page, citation)", x: 74, y: 22, w: 800, h: 72, size: 30, bold: true, align: "right", valign: "center" },
      subtitle: { label: "Subtitle / section", x: 74, y: 94, w: 800, h: 52, size: 43, bold: true, italic: true, valign: "center" },
      text: { label: "Text", x: 74, y: 150, w: 800, h: 908, size: 52, valign: "top" },
      labels: labels(30),
      people,
    },
  },
  {
    key: "stream.bottom_bar",
    output: "stream",
    label: "Bottom Bar",
    hint: "Psalms and the Eucharistic Prayer: the title in the gold box, the musical setting under it, the text in the bar.",
    layout: "bottom_bar",
    background: "builtin:bar-bottom",
    sample: "psalm",
    elements: {
      tag: { label: "Title (gold box)", x: 94, y: 303, w: 382, h: 83, size: 50, bold: true, color: "#ffffff", align: "center", valign: "center" },
      music: { label: "Musical setting image", kind: "image", x: 90, y: 398, w: 1740, h: 226 },
      title: { label: "Title in the bar", x: 56, y: 672, w: 1808, h: 62, size: 57, bold: true, valign: "center", hidden: true },
      subtitle: { label: "Section heading in the bar", x: 56, y: 672, w: 1808, h: 62, size: 43, bold: true, italic: true, align: "right", valign: "center", hidden: true },
      text: { label: "Text", x: 56, y: 672, w: 1808, h: 344, size: 50, valign: "center" },
      labels: labels(30),
      people,
    },
  },
  {
    key: "stream.lower_third",
    output: "stream",
    label: "Lower Third",
    hint: "Readings and short responses over the camera: everything else is keyed out.",
    layout: "lower_third",
    background: "key",
    sample: "reading",
    elements: {
      title: { label: "Title", x: 78, y: 850, w: 1228, h: 44, size: 32, bold: true, color: "#ffffff", valign: "center" },
      reference: { label: "Reference", x: 78, y: 850, w: 1228, h: 44, size: 30, bold: true, color: "#ffffff", align: "right", valign: "center" },
      text: { label: "Text", x: 78, y: 894, w: 1228, h: 116, size: 50, color: "#ffffff", align: "center", valign: "center" },
      labels: labels(30, { color: "#ffffff" }),
      people,
    },
  },
  {
    key: "stream.hymn",
    output: "stream",
    label: "Hymn",
    hint: "Hymns: the wide panel on the left, with the words and music credits.",
    layout: "left_panel_wide",
    background: "builtin:panel-light",
    sample: "hymn",
    elements: {
      title: { label: "Title (Entrance Hymn)", x: 74, y: 22, w: 960, h: 72, size: 60, bold: true, valign: "center" },
      reference: { label: "Hymn number", x: 74, y: 22, w: 960, h: 72, size: 30, bold: true, align: "right", valign: "center" },
      subtitle: { label: "Hymn title", x: 74, y: 94, w: 960, h: 52, size: 43, bold: true, italic: true, valign: "center" },
      text: { label: "Verses", x: 74, y: 150, w: 960, h: 830, size: 52, bold: true, valign: "top" },
      credits: { label: "Words / music credits", x: 74, y: 990, w: 960, h: 80, size: 22, bold: true, valign: "bottom" },
      labels: labels(30, { italic: false }),
    },
  },
  {
    key: "sanctuary.prayer",
    output: "sanctuary",
    label: "Prayer",
    hint: "Prayers, collects and dialogues between the glass bands.",
    layout: "centered",
    background: "builtin:glass-light",
    sample: "prayer",
    elements: {
      title: { label: "Title", x: 70, y: 135, w: 1780, h: 92, size: 69, valign: "bottom" },
      subtitle: { label: "Section heading", x: 70, y: 135, w: 1780, h: 92, size: 69, align: "right", valign: "bottom" },
      text: { label: "Text", x: 70, y: 227, w: 1780, h: 718, size: 73, valign: "center" },
      labels: labels(64),
      people,
    },
  },
  {
    key: "sanctuary.title",
    output: "sanctuary",
    label: "Title",
    hint: "Title slides: “The First Reading / Joshua 24:1-3a”.",
    layout: "centered",
    background: "builtin:glass-light",
    sample: "title",
    elements: {
      subtitle: { label: "Heading", x: 70, y: 300, w: 1780, h: 130, size: 73, align: "center", valign: "bottom" },
      title: { label: "Main title", x: 70, y: 440, w: 1780, h: 340, size: 128, align: "center", valign: "top" },
    },
  },
  {
    key: "sanctuary.sermon",
    output: "sanctuary",
    label: "Sermon",
    hint: "The sermon: “The Sermon”, the preacher and their position, laid out like a reading's title slide.",
    layout: "centered",
    background: "builtin:glass-light",
    sample: "sermon",
    elements: {
      subtitle: { label: "Heading (The Sermon)", x: 70, y: 300, w: 1780, h: 130, size: 73, align: "center", valign: "bottom" },
      title: { label: "Preacher", x: 70, y: 460, w: 1780, h: 170, size: 104, align: "center", valign: "top" },
      position: { label: "Position", x: 70, y: 640, w: 1780, h: 120, size: 64, italic: true, align: "center", valign: "top" },
    },
  },
  {
    key: "sanctuary.psalm",
    output: "sanctuary",
    label: "Psalm",
    hint: "Psalm verses, and antiphons: the words alone, with the psalm at the top left and “Antiphon” at the right (the music line before the Gospel goes almost full width with the words under it).",
    layout: "centered",
    background: "builtin:glass-light",
    sample: "psalm",
    elements: {
      title: { label: "Psalm title", x: 70, y: 135, w: 1780, h: 92, size: 69, valign: "bottom" },
      tag: { label: "“Antiphon” (right of the title)", x: 70, y: 135, w: 1780, h: 92, size: 69, align: "right", valign: "bottom" },
      text: { label: "Verses", x: 70, y: 227, w: 1780, h: 718, size: 73, valign: "center" },
      music: { label: "Antiphon music", kind: "image", x: 70, y: 150, w: 1780, h: 520 },
      antiphon: { label: "Antiphon words", x: 70, y: 690, w: 1780, h: 250, size: 73, bold: true, align: "center", valign: "top" },
      labels: labels(64, { italic: false }),
      people,
    },
  },
];

export const slideType = (key) => SLIDE_TYPES.find((t) => t.key === key);

/** How a slide type lays out lines, with the operator's changes:
 *  `join` runs a paragraph's lines together (the stream's hymn verses);
 *  `hanging` carries a too-long line on indented under itself; `alternate`
 *  indents every other psalm verse (the stream's psalms). `lines` is the
 *  number of lines a slide holds (null: as the text box holds, or on the
 *  sanctuary as written). */
export function typeOptions(templates, key) {
  const t = templates?.[key] ?? {};
  const stream = key?.startsWith("stream.");
  return {
    join: t.line_breaks ? t.line_breaks === "join" : key === "stream.hymn",
    hanging: t.hanging ?? true,
    alternate: t.alternate_verses ?? stream,
    lines: t.max_lines ?? null,
  };
}

/** The slide type's elements with the operator's changes applied. */
export function resolveTemplate(templates, key) {
  const type = slideType(key);
  if (!type) return null;
  const custom = templates?.[key]?.elements ?? {};
  const out = {};
  for (const [name, def] of Object.entries(type.elements)) {
    const c = custom[name] ?? {};
    out[name] = { ...def };
    for (const [k, v] of Object.entries(c)) if (v != null) out[name][k] = v;
  }
  return out;
}

const isAntiphonSlide = (slide) =>
  /^(antiphon|gospel acclamation)$/i.test(slide?.heading ?? "") || (slide?.blocks ?? []).some((b) => /^antiphon$/i.test(b.role ?? ""));
export { isAntiphonSlide };

/** Which slide type renders this slide, or null for the fixed layouts
 *  (sanctuary hymn lyrics, the welcome bar, full-width stream text). */
export function slideTypeFor(cue, slide, output, look) {
  if (output === "sanctuary") {
    if (look.layout === "lyrics" || look.layout === "info_bar" || look.layout === "lower_third") return null;
    if (cue.category === "homily") return "sanctuary.sermon";
    if (!slide || cue.content.kind === "announcements" || !slide.blocks.length) return "sanctuary.title";
    // The music line before the Gospel is laid out like the psalm's.
    if (cue.category === "psalm" || (cue.category === "gospel" && isAntiphonSlide(slide))) return "sanctuary.psalm";
    return "sanctuary.prayer";
  }
  return (
    {
      left_panel: "stream.left_panel",
      left_panel_wide: "stream.hymn",
      bottom_bar: "stream.bottom_bar",
      lower_third: "stream.lower_third",
    }[look.layout] ?? null
  );
}
