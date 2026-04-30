/**
 * Loader for precomputed.json — every built-in level's BFS-optimal solution,
 * pre-computed by the Rust solver. The web app uses these directly so it never
 * needs to run the JS solver for built-in levels (orders of magnitude faster).
 */

export interface PrecomputedEntry {
  /** level index in the file (0..2557) */
  i: number;
  /** pet count */
  p: number;
  w: number;
  h: number;
  par: number;
  t: number;
  /** BFS-optimum fuel */
  fuel: number | null;
  /** Visit-order history string in **Lumosity convention**: uppercase = pet pickup, lowercase = house drop. */
  hist: string | null;
  /** Original canned solution from the Lumosity solutions file, same convention. */
  canned: string;
  /** Cost of the canned solution if replayable, else null. */
  cf: number | null;
}

let cache: PrecomputedEntry[] | null = null;
let inflight: Promise<PrecomputedEntry[]> | null = null;

export async function loadPrecomputed(): Promise<PrecomputedEntry[]> {
  if (cache) return cache;
  if (inflight) return inflight;
  inflight = (async () => {
    const r = await fetch(`${import.meta.env.BASE_URL}data/precomputed.json`);
    if (!r.ok) throw new Error(`precomputed.json: HTTP ${r.status}`);
    const j = (await r.json()) as PrecomputedEntry[];
    cache = j;
    return j;
  })();
  return inflight;
}

/** Convert a Lumosity-convention hist string to the in-app convention (case swap). */
export function lumositytoApp(hist: string): string {
  let out = '';
  for (const c of hist) {
    if (c >= 'A' && c <= 'Z') out += c.toLowerCase();
    else if (c >= 'a' && c <= 'z') out += c.toUpperCase();
    else out += c;
  }
  return out;
}
