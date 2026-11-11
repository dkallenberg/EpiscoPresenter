// The Google Slides announcements deck: any link to it (edit, share,
// present, or "Publish to web") becomes Google's embed address.
//
// The deck must be viewable by the output computers: share it as "Anyone
// with the link can view", or use File → Share → Publish to web.

const DELAY_MS = 9000;

/** { kind: "doc" | "pub", id } for a Google Slides link or bare id, else null. */
export function parseDeck(value) {
  const v = (value ?? "").trim();
  if (!v) return null;
  let m = v.match(/docs\.google\.com\/presentation\/d\/e\/([\w-]+)/);
  if (m) return { kind: "pub", id: m[1] };
  m = v.match(/docs\.google\.com\/presentation\/(?:u\/\d+\/)?d\/([\w-]+)/);
  if (m) return { kind: "doc", id: m[1] };
  if (/^[\w-]{25,}$/.test(v)) return { kind: "doc", id: v };
  return null;
}

/**
 * The embed URL. `slide` (1-based) shows that slide, still; otherwise the
 * deck plays itself every 9 seconds, once or in a `loop`.
 */
export function deckEmbedUrl(value, { slide = null, loop = false } = {}) {
  const deck = parseDeck(value);
  if (!deck) return null;
  const base =
    deck.kind === "pub"
      ? `https://docs.google.com/presentation/d/e/${deck.id}/embed`
      : `https://docs.google.com/presentation/d/${deck.id}/embed`;
  const play = slide == null;
  const params = new URLSearchParams({
    start: String(play),
    loop: String(play && loop),
    delayms: String(DELAY_MS),
    rm: "minimal",
  });
  if (slide != null) params.set("slide", String(slide));
  return `${base}?${params}`;
}
