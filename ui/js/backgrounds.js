// Built-in backgrounds, drawn as SVG in the cue's liturgical colour.
//
// They reproduce the geometry of the parish's slide designs (stained-glass
// bands and panels, brick borders) so that the text boxes in render.js line up
// whether a cue uses a built-in or the parish's own PNG with the same layout.
// All coordinates are on the 1920x1080 stage.

const W = 1920;
const H = 1080;

// Base glass colour per liturgical colour; tints are mixed toward white.
const BASE = {
  Green: { glass: "#5f8f3e", panel: "#638c45", light: "#dfe3d9" },
  Violet: { glass: "#6a3f8f", panel: "#5a3a78", light: "#e4dfe9" },
  White: { glass: "#c3a24a", panel: "#9c8443", light: "#efebdf" },
  Red: { glass: "#a8352d", panel: "#8e3a33", light: "#efe0dd" },
  Blue: { glass: "#3561a8", panel: "#3a5f8f", light: "#dfe5ee" },
  Black: { glass: "#6b6b6b", panel: "#3e3e3e", light: "#e4e4e4" },
};
const BRICK = "#b5803a";
const LEAD = "#1b1b1b"; // the "lead" lines between glass panes

function mix(hex, toward, t) {
  const p = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
  const [a, b] = [p(hex), p(toward)];
  return `#${a.map((v, i) => Math.round(v + (b[i] - v) * t).toString(16).padStart(2, "0")).join("")}`;
}

// Deterministic RNG so a background never changes between renders.
function rng(seed) {
  let h = 2166136261;
  for (const c of seed) h = Math.imul(h ^ c.charCodeAt(0), 16777619);
  return () => {
    h += 0x6d2b79f5;
    let t = h;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Stained glass filling a rectangle: recursive guillotine cuts. */
function glass(out, rand, base, x, y, w, h, cell = 150) {
  const tints = [0.05, 0.25, 0.42, 0.58, 0.7, 0.8];
  const split = (x, y, w, h, depth) => {
    const big = w > cell * 1.4 || h > cell * 1.1;
    if ((big || (depth < 2 && rand() < 0.5)) && (w > 40 && h > 30)) {
      if (w / h > 1.3 || (w / h > 0.7 && rand() < 0.5)) {
        const cut = Math.round(w * (0.3 + rand() * 0.4));
        split(x, y, cut, h, depth + 1);
        split(x + cut, y, w - cut, h, depth + 1);
      } else {
        const cut = Math.round(h * (0.3 + rand() * 0.4));
        split(x, y, w, cut, depth + 1);
        split(x, y + cut, w, h - cut, depth + 1);
      }
      return;
    }
    const fill = mix(base, "#ffffff", tints[Math.floor(rand() * tints.length)]);
    out.push(`<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${fill}" stroke="${LEAD}" stroke-width="4"/>`);
  };
  split(x, y, w, h, 0);
}

/** A row or column of bricks (the brown borders in the parish designs). */
function bricks(out, x, y, w, h, vertical = false) {
  out.push(`<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${BRICK}" stroke="${LEAD}" stroke-width="3"/>`);
  const len = 120;
  if (vertical) for (let yy = y + len; yy < y + h; yy += len) out.push(`<line x1="${x}" y1="${yy}" x2="${x + w}" y2="${yy}" stroke="${LEAD}" stroke-width="3"/>`);
  else for (let xx = x + len; xx < x + w; xx += len) out.push(`<line x1="${xx}" y1="${y}" x2="${xx}" y2="${y + h}" stroke="${LEAD}" stroke-width="3"/>`);
}

function panel(out, id, x, y, w, h, color) {
  out.push(
    `<defs><linearGradient id="${id}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${mix(color, "#000000", 0.08)}"/><stop offset="1" stop-color="${mix(color, "#ffffff", 0.12)}"/></linearGradient></defs>`,
    `<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="url(#${id})" stroke="${LEAD}" stroke-width="4"/>`,
  );
}

const DESIGNS = {
  // Sanctuary: glass bands top and bottom, light centre (prayers).
  "glass-light": (o, r, c) => {
    glass(o, r, c.glass, 0, 0, W, 92, 170);
    glass(o, r, c.glass, 0, 988, W, 92, 170);
    panel(o, "g", 0, 92, W, 896, c.light);
  },
  // Sanctuary: glass bands, panel in the liturgical colour (readings).
  "glass-dark": (o, r, c) => {
    glass(o, r, c.glass, 0, 0, W, 92, 170);
    glass(o, r, c.glass, 0, 988, W, 92, 170);
    panel(o, "g", 0, 92, W, 896, c.panel);
  },
  // Sanctuary: glass bands, charcoal centre (psalms).
  "glass-charcoal": (o, r, c) => {
    glass(o, r, c.glass, 0, 0, W, 92, 170);
    glass(o, r, c.glass, 0, 988, W, 92, 170);
    panel(o, "g", 0, 92, W, 896, "#353535");
  },
  // Stream: dark panel on the left between brick borders, glass on the right.
  "panel-dark": (o, r, c) => {
    glass(o, r, c.glass, 0, 0, 34, H, 90);
    bricks(o, 32, 0, 26, H, true);
    panel(o, "g", 58, 0, 835, H, c.panel);
    bricks(o, 893, 0, 36, H, true);
    glass(o, r, c.glass, 929, 0, W - 929, H, 160);
  },
  // Stream: light panel on the left (hymns), glass on the right.
  "panel-light": (o, r, c) => {
    glass(o, r, c.glass, 0, 0, 58, H, 90);
    panel(o, "g", 58, 0, 990, H, c.light);
    glass(o, r, c.glass, 1048, 0, W - 1048, H, 160);
  },
  // Welcome / postlude: camera window framed in brick, info bar beneath,
  // glass down both sides (measured from the parish template).
  window: (o, r, c) => {
    glass(o, r, c.glass, 0, 0, 182, H, 90);
    glass(o, r, c.glass, 1738, 0, W - 1738, H, 90);
    bricks(o, 182, 0, 26, 842, true);
    bricks(o, 1712, 0, 26, 842, true);
    panel(o, "w", 208, 0, 1504, 842, "#1f1f1f");
    bricks(o, 182, 842, 1556, 26);
    panel(o, "g", 182, 868, 1556, 212, c.panel);
  },
  // Stream: glass above, brick-framed bar across the bottom, label tag.
  "bar-bottom": (o, r, c) => {
    // Measured from the parish template: bricks 58.6–60.7% and 94.8–96.9%,
    // label tag x 4.9–24.8%, y 28.1–35.7%.
    glass(o, r, c.glass, 0, 0, W, 633, 170);
    bricks(o, 0, 633, W, 23);
    panel(o, "g", 0, 656, W, 368, c.panel);
    bricks(o, 0, 1024, W, 23);
    glass(o, r, c.glass, 0, 1047, W, 33, 170);
    o.push(`<rect x="94" y="303" width="382" height="83" fill="${BRICK}" stroke="${LEAD}" stroke-width="4"/>`);
  },
};

export const BUILTIN_LABELS = {
  "builtin:glass-light": "Glass · light",
  "builtin:glass-dark": "Glass · colour",
  "builtin:glass-charcoal": "Glass · charcoal",
  "builtin:panel-dark": "Stream panel · colour",
  "builtin:panel-light": "Stream panel · light",
  "builtin:bar-bottom": "Stream bottom bar",
  "builtin:window": "Welcome window",
  key: "Key colour (lower third)",
  "#000000": "Black",
};

const cache = new Map();

// The parish's own artwork, used in place of the drawn design where it
// exists (files in ui/backgrounds/<colour>/, bundled with the app). Add a
// colour by dropping files with these names into a new folder and listing
// it here.
const ARTWORK = {
  Green: {
    "panel-dark": "/backgrounds/green/panel-dark.webp",
    "panel-light": "/backgrounds/green/panel-light.webp",
    "bar-bottom": "/backgrounds/green/bar-bottom.webp",
    "lower-third": "/backgrounds/green/lower-third.webp",
    "glass-light": "/backgrounds/green/glass-light.webp",
    "glass-dark": "/backgrounds/green/glass-dark.webp",
    "glass-charcoal": "/backgrounds/green/glass-charcoal.webp",
    window: "/backgrounds/green/window.webp",
  },
};

/** Parish artwork for a design in a colour, if there is any. */
export function artworkUrl(design, color) {
  return ARTWORK[color]?.[design] ?? null;
}

/** URL for a built-in background in a liturgical colour, or null. */
export function builtinUrl(name, color) {
  const id = name.replace(/^builtin:/, "");
  const art = artworkUrl(id, color);
  if (art) return art;
  const design = DESIGNS[id];
  if (!design) return null;
  const c = BASE[color] ?? BASE.Green;
  const key = `${name}|${color}`;
  if (!cache.has(key)) {
    const out = [];
    design(out, rng(key), c);
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" preserveAspectRatio="none">${out.join("")}</svg>`;
    cache.set(key, `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`);
  }
  return cache.get(key);
}

/** The framed bar drawn over the key colour for built-in lower thirds. */
export function lowerThirdFrameUrl(color) {
  const key = `lt|${color}`;
  if (!cache.has(key)) {
    const c = BASE[color] ?? BASE.Green;
    const o = [];
    // Bar spans 1.5–70.4% x 74.8–97.5% y, as in the parish template.
    const x = 29, y = 810, w = 1325, h = 244;
    bricks(o, x, y, w, 32);
    bricks(o, x, y + h - 32, w, 32);
    bricks(o, x, y + 32, 32, h - 64, true);
    bricks(o, x + w - 32, y + 32, 32, h - 64, true);
    panel(o, "lt", x + 32, y + 32, w - 64, h - 64, c.panel);
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}">${o.join("")}</svg>`;
    cache.set(key, `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`);
  }
  return cache.get(key);
}
