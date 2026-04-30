/**
 * Loader for the Lumosity Pet Detective level + solution text files.
 *
 * On-disk convention: UPPERCASE = pet, lowercase = house, '4' = car start.
 * The existing solver in this app expects: lowercase = animal, UPPERCASE = house, 'X' = start.
 * So this loader **swaps case** as it parses, and translates '4' → 'X'.
 *
 * Solutions in the file are visit-order strings using the on-disk convention;
 * after case-swap they slot into the existing `SolveStep` model directly.
 */
import type { ParsedPuzzle } from './types';
import { ANIMAL_HUES } from './animals';

export interface BuiltinLevel {
  /** zero-based index in the file (0..2557). */
  idx: number;
  petCount: number;
  /** grid cell dimensions. */
  cols: number;
  rows: number;
  /** par_moves field from the file. */
  parMoves: number;
  parTime: number;
  /** raw on-disk solution string (UPPERCASE = pet pickup, lowercase = drop). */
  rawSolution: string;
  /** parsed puzzle compatible with the existing Board / solver. */
  puzzle: ParsedPuzzle;
}

export interface LevelGroup {
  petCount: number;
  cols: number;
  rows: number;
  levels: BuiltinLevel[];
}

function swapCase(c: string): string {
  if (c >= 'A' && c <= 'Z') return c.toLowerCase();
  if (c >= 'a' && c <= 'z') return c.toUpperCase();
  return c;
}

function parseOne(textBlock: string[], idx: number): {
  puzzle: ParsedPuzzle;
  petCount: number;
} {
  const textH = textBlock.length;
  const textW = Math.max(...textBlock.map((l) => l.length));
  if (textH % 2 !== 1 || textW % 2 !== 1) {
    throw new Error(`level ${idx}: text dims must be odd, got ${textW}×${textH}`);
  }
  // Lumosity stores levels rotated 90°: each text-row corresponds to a game COLUMN
  // (so the level renders portrait on phones). text-cols → game rows; text-rows → game cols.
  const rows = (textW + 1) >> 1;
  const cols = (textH + 1) >> 1;

  const lines = textBlock.map((l) => l.padEnd(textW, ' '));

  const grid: string[][] = Array.from({ length: rows }, () => Array(cols).fill(''));
  const hRoads: string[][] = Array.from({ length: rows }, () => Array(Math.max(0, cols - 1)).fill('x'));
  const vRoads: string[][] = Array.from({ length: Math.max(0, rows - 1) }, () => Array(cols).fill('x'));
  let start: [number, number] | null = null;
  const animalCells: Record<string, [number, number]> = {};
  const houseCells: Record<string, [number, number]> = {};

  // Lumosity stores levels rotated 90° AND mirrored horizontally relative to the
  // text layout: top-of-text is the right column of the in-game board. Game (r, c)
  // therefore lives at file text position (row = 2*(cols-1-c), col = 2*r).
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const ch = lines[2 * (cols - 1 - c)][2 * r];
      let cell = '';
      if (ch === '.') {
        cell = '.';
      } else if (ch === ' ') {
        cell = '';
      } else if (ch === '4') {
        cell = 'X';
        start = [r, c];
      } else if ((ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z')) {
        cell = swapCase(ch);
        if (cell >= 'a' && cell <= 'z') animalCells[cell] = [r, c];
        else houseCells[cell] = [r, c];
      }
      grid[r][c] = cell;
    }
  }

  // Horizontal road between game (r, c) and (r, c+1) — after the same horizontal flip,
  // sits between text-rows 2*(cols-1-c) and 2*(cols-1-(c+1)) at text-col 2r, i.e. the
  // text-row in between is 2*(cols-1-c) - 1 = 2*(cols-2-c) + 1.
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols - 1; c++) {
      const hi = lines[2 * (cols - 2 - c) + 1][2 * r];
      hRoads[r][c] = hi === '.' || hi === '|' ? '-' : 'x';
    }
  }

  // Vertical road between game (r, c) and (r+1, c) sits at text position (2*(cols-1-c), 2r+1).
  for (let r = 0; r < rows - 1; r++) {
    for (let c = 0; c < cols; c++) {
      const vi = lines[2 * (cols - 1 - c)][2 * r + 1];
      vRoads[r][c] = vi === '.' || vi === '-' ? '|' : 'x';
    }
  }

  if (!start) throw new Error(`level ${idx}: no '4' (start) cell`);

  // Build animals[] / houses[] by index 0..pairCount-1, ordered a..z / A..Z.
  const animals: [number, number][] = [];
  const houses: [number, number][] = [];
  for (let i = 0; i < 26; i++) {
    const a = animalCells[String.fromCharCode(97 + i)];
    const h = houseCells[String.fromCharCode(65 + i)];
    if (a && h) {
      animals.push(a);
      houses.push(h);
    } else if (a || h) {
      throw new Error(`level ${idx}: pet/house ${i} (${String.fromCharCode(65 + i)}) is unpaired`);
    }
  }

  const hues = animals.map((_, i) => ANIMAL_HUES[i % ANIMAL_HUES.length]);

  const puzzle: ParsedPuzzle = {
    rows,
    cols,
    grid,
    vRoads,
    hRoads,
    start,
    animals,
    houses,
    hues,
  };

  return { puzzle, petCount: animals.length };
}

/**
 * Parse the levels file. Each level block is followed by `<int>` (par_moves) then
 * `<float>` (par_time), then a blank line.
 */
export function parseLevelsText(text: string): { puzzle: ParsedPuzzle; petCount: number; parMoves: number; parTime: number }[] {
  const lines = text.split('\n');
  const out: { puzzle: ParsedPuzzle; petCount: number; parMoves: number; parTime: number }[] = [];
  let buf: string[] = [];
  let i = 0;
  while (i < lines.length) {
    const l = lines[i];
    if (l === '') { i++; continue; }
    const trimmed = l.endsWith('.') ? l.slice(0, -1) : l;
    const isInt = trimmed.length > 0 && /^\d+$/.test(trimmed);
    if (isInt && buf.length > 0) {
      const parMoves = parseInt(trimmed, 10);
      i++;
      const parTime = parseFloat(lines[i]);
      i++;
      const idx = out.length;
      const { puzzle, petCount } = parseOne(buf, idx);
      out.push({ puzzle, petCount, parMoves, parTime });
      buf = [];
    } else {
      buf.push(l);
      i++;
    }
  }
  return out;
}

/** Strip the blank lines between groups; aligns 1:1 with parsed levels. */
export function parseSolutionsText(text: string): string[] {
  return text.split('\n').filter((l) => l !== '');
}

/** Convert an on-disk solution string to the in-app convention (swap case). */
export function swapSolutionCase(rawSol: string): string {
  let out = '';
  for (const c of rawSol) out += swapCase(c);
  return out;
}

/** Group built-in levels by (petCount, grid size). */
export function groupLevels(levels: BuiltinLevel[]): LevelGroup[] {
  const map = new Map<string, LevelGroup>();
  for (const lv of levels) {
    const key = `${lv.petCount}/${lv.cols}x${lv.rows}`;
    let g = map.get(key);
    if (!g) {
      g = { petCount: lv.petCount, cols: lv.cols, rows: lv.rows, levels: [] };
      map.set(key, g);
    }
    g.levels.push(lv);
  }
  return Array.from(map.values()).sort((a, b) =>
    a.petCount - b.petCount ||
    a.cols * a.rows - b.cols * b.rows ||
    a.cols - b.cols,
  );
}

/**
 * Fetch and parse the levels + solutions files from /data/.
 * Returns the parsed levels, in file order.
 */
export async function loadBuiltinLevels(): Promise<BuiltinLevel[]> {
  const base = import.meta.env.BASE_URL;
  const [levelsText, solutionsText] = await Promise.all([
    fetch(`${base}data/levels.txt`).then((r) => r.text()),
    fetch(`${base}data/solutions.txt`).then((r) => r.text()),
  ]);
  const parsed = parseLevelsText(levelsText);
  const sols = parseSolutionsText(solutionsText);
  return parsed.map((p, i) => ({
    idx: i,
    petCount: p.petCount,
    cols: p.puzzle.cols,
    rows: p.puzzle.rows,
    parMoves: p.parMoves,
    parTime: p.parTime,
    rawSolution: sols[i] ?? '',
    puzzle: p.puzzle,
  }));
}
