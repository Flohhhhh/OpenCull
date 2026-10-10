// Keeps pictures loaded and decoded inside the view before they are asked for.
//
// The backend can hand over a picture in a millisecond, but the view still
// has to decode it before it can be drawn, and an <img> only starts on that
// once it exists. Hold an arrow key and the next photo arrives before the
// decoding is done, so each one was drawn from the filmstrip's small
// thumbnail, with the larger picture popping in a moment later or not at
// all. A picture held here has been through all of that already: an <img>
// given the same address draws it in the frame it appears.

/** Screen-sized pictures held at once. Decoded, each is about seven megabytes. */
const LIMIT = 16;

// Oldest first: a Map keeps the order things were put in.
const held = new Map<string, HTMLImageElement>();
const ready = new Set<string>();

function load(url: string, onReady: () => void): HTMLImageElement {
  const img = new Image();
  img.decoding = "async";
  img.src = url;
  // A file with no picture of this kind fails here, and is simply never ready.
  img.decode().then(onReady, () => {});
  return img;
}

function release(img: HTMLImageElement): void {
  img.src = "";
}

/** Gets the picture at `url` ready, and keeps it ready while it is among the most recent. */
export function hold(url: string): void {
  const had = held.get(url);
  if (had) {
    held.delete(url);
    held.set(url, had);
    return;
  }
  const img = load(url, () => {
    if (held.get(url) === img) ready.add(url);
  });
  held.set(url, img);
  for (const [old, el] of held) {
    if (held.size <= LIMIT) break;
    held.delete(old);
    ready.delete(old);
    release(el);
  }
}

/** Whether the picture at `url` can be drawn this instant. */
export function isHeld(url: string): boolean {
  return ready.has(url);
}

// The full-size picture is a hundred megabytes and more once decoded, so only
// one is ever held: the photo being looked at.
let full: { url: string; img: HTMLImageElement } | null = null;

/** Gets the full-size picture at `url` ready in place of the last one; `null` lets it go. */
export function holdFull(url: string | null): void {
  if (full?.url === url) return;
  if (full) release(full.img);
  full = url ? { url, img: load(url, () => {}) } : null;
}
