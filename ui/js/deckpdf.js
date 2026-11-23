// The announcements deck (and imported presentations) as slide images. The app downloads the Google
// Slides deck as a PDF (src-tauri/src/deck.rs); here pdf.js turns its pages
// into images, cached per size, for the timeline, the slide grid, the
// previews and the output screens.

import { invoke, listen } from "/js/bridge.js";

const T = window.__TAURI__;
const decks = new Map(); // deck url -> { doc: Promise<PDFDocumentProxy|null>, pages, images: Map }
const errors = new Map(); // deck url -> why its slides couldn't be shown

/** Why a deck's slides couldn't be shown on this computer, if they couldn't. */
export const deckError = (url) => errors.get(url) ?? null;
const listeners = new Set();
let pdfjs = null;

/** Seconds per slide when the deck loops. */
export const LOOP_SECONDS = 9;

async function lib() {
  if (!pdfjs) {
    // The PDF is read here in the page, not in a Web Worker: a Mac release
    // copy doesn't start workers from the app's own pages, which left the
    // slides waiting for good. pdf.js uses the reader it finds here instead
    // of starting one; decks are small.
    globalThis.pdfjsWorker ??= await import("/vendor/pdfjs/pdf.worker.min.mjs");
    pdfjs = await import("/vendor/pdfjs/pdf.min.mjs");
    pdfjs.GlobalWorkerOptions.workerSrc = "/vendor/pdfjs/pdf.worker.min.mjs";
  }
  return pdfjs;
}

/** `promise`, or an error after `ms` (so a stuck step says so). */
const within = (promise, ms, why) =>
  Promise.race([promise, new Promise((_, reject) => setTimeout(() => reject(new Error(why)), ms))]);

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

/** A PDF that is already a file the page can read (one in the media
 *  library), rather than a Google Slides link to download. */
const isFileUrl = (url) => /^(asset:|https?:\/\/asset\.localhost|blob:|\/)/.test(url);

async function open(url, fetchIfMissing) {
  let src;
  // Each step says what went wrong, for deckError().
  const fail = (why) => {
    throw new Error(why);
  };
  if (isFileUrl(url)) src = url;
  else {
    if (!T) return null;
    let file = await invoke("deck_local", { url }).catch((e) => fail(`couldn't look for the downloaded slides (${e})`));
    if (!file && fetchIfMissing) file = await invoke("deck_fetch", { url, refresh: false }).catch((e) => fail(String(e)));
    if (!file) fail("the slides haven't been downloaded to this computer yet");
    src = T.core.convertFileSrc(file.path);
  }
  const response = await fetch(src).catch((e) => fail(`couldn't read the downloaded slides from ${src} (${e})`));
  if (!response.ok) fail(`couldn't read the downloaded slides from ${src} (${response.status})`);
  const bytes = await response.arrayBuffer();
  const pdf = await lib().catch((e) => fail(`couldn't load the PDF reader (${e})`));
  return within(pdf.getDocument({ data: new Uint8Array(bytes), isEvalSupported: false }).promise, 30000, "the PDF reader didn't answer").catch((e) =>
    fail(`couldn't read the PDF (${e.message ?? e})`),
  );
}

/** Loads a deck (downloading it if `fetchIfMissing`); resolves to its page count or 0. */
export function load(url, fetchIfMissing = false) {
  if (!url) return Promise.resolve(0);
  let d = decks.get(url);
  if (!d || (fetchIfMissing && d.failed)) {
    d = { pages: null, failed: false, images: new Map() };
    d.doc = open(url, fetchIfMissing)
      .then((doc) => (errors.delete(url), doc))
      .catch((e) => {
        console.warn("announcements deck:", e);
        errors.set(url, e?.message ?? String(e));
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
