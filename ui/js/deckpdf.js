// The announcements deck as slide images. The app downloads the Google
// Slides deck as a PDF (src-tauri/src/deck.rs); here pdf.js turns its pages
// into images, cached per size, for the timeline, the slide grid, the
// previews and the output screens.

import { invoke, listen } from "/js/bridge.js";

const T = window.__TAURI__;
const decks = new Map(); // deck url -> { doc: Promise<PDFDocumentProxy|null>, pages, images: Map }
const listeners = new Set();
let pdfjs = null;

/** Seconds per slide when the deck loops. */
export const LOOP_SECONDS = 9;

async function lib() {
  if (!pdfjs) {
    pdfjs = await import("/vendor/pdfjs/pdf.min.mjs");
    pdfjs.GlobalWorkerOptions.workerSrc = "/vendor/pdfjs/pdf.worker.min.mjs";
  }
  return pdfjs;
}

/** Calls `fn(url)` whenever a deck's slides become available or change. */
export function onDeckChange(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}
const changed = (url) => listeners.forEach((fn) => fn(url));

listen("episco://deck", () => {
  // A deck was (re)downloaded: forget every cached copy and reload.
  const urls = [...decks.keys()];
  decks.clear();
  urls.forEach((u) => load(u).then(() => changed(u)));
});

async function open(url, fetchIfMissing) {
  if (!T) return null;
  let file = await invoke("deck_local", { url }).catch(() => null);
  if (!file && fetchIfMissing) file = await invoke("deck_fetch", { url, refresh: false }).catch(() => null);
  if (!file) return null;
  const bytes = await fetch(T.core.convertFileSrc(file.path)).then((r) => (r.ok ? r.arrayBuffer() : null));
  if (!bytes) return null;
  const pdf = await lib();
  return pdf.getDocument({ data: new Uint8Array(bytes), isEvalSupported: false }).promise;
}

/** Loads a deck (downloading it if `fetchIfMissing`); resolves to its page count or 0. */
export function load(url, fetchIfMissing = false) {
  if (!url) return Promise.resolve(0);
  let d = decks.get(url);
  if (!d || (fetchIfMissing && d.failed)) {
    d = { pages: null, failed: false, images: new Map() };
    d.doc = open(url, fetchIfMissing).catch((e) => {
      console.warn("announcements deck:", e);
      return null;
    });
    d.doc.then((doc) => {
      d.pages = doc?.numPages ?? 0;
      d.failed = !doc;
      changed(url);
    });
    decks.set(url, d);
  }
  return d.doc.then((doc) => doc?.numPages ?? 0);
}

/** Page count if the deck is loaded (0 if it could not be), else null. */
export function pageCount(url) {
  return decks.get(url)?.pages ?? null;
}

/** An image URL of slide `n` (1-based) about `width` pixels wide, or null. */
export function slideImage(url, n, width = 1920) {
  const d = decks.get(url) ?? (load(url), decks.get(url));
  const key = `${n}@${width}`;
  if (!d.images.has(key)) {
    d.images.set(
      key,
      d.doc.then(async (doc) => {
        if (!doc || n < 1 || n > doc.numPages) return null;
        const page = await doc.getPage(n);
        const base = page.getViewport({ scale: 1 });
        const viewport = page.getViewport({ scale: width / base.width });
        const canvas = document.createElement("canvas");
        canvas.width = Math.round(viewport.width);
        canvas.height = Math.round(viewport.height);
        await page.render({ canvasContext: canvas.getContext("2d"), viewport }).promise;
        const blob = await new Promise((r) => canvas.toBlob(r, "image/png"));
        return blob ? URL.createObjectURL(blob) : null;
      }),
    );
  }
  return d.images.get(key);
}

/** The slide a looping deck shows now (1-based), the same on every screen. */
export function loopSlide(pages, now = Date.now()) {
  return pages > 0 ? (Math.floor(now / (LOOP_SECONDS * 1000)) % pages) + 1 : 1;
}
