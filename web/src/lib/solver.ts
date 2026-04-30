import type { ParsedPuzzle, SolveResult, SolveStep } from './types';

/**
 * Path-builder for built-in levels. Given a precomputed visit-order string
 * (in this app's case-swapped convention: lowercase = animal pickup,
 * uppercase = house drop), walk it event-by-event and reconstruct the
 * cell-level path between each pair of events with a single grid BFS.
 *
 * The state-space search itself was moved to the Rust solver and shipped as
 * `precomputed.json` — this is just animation glue.
 */

interface BfsResult {
  dist: Int16Array;   // length rows*cols, -1 if unreachable
  parent: Int16Array; // for path reconstruction
}

function bfs(start: number, rows: number, cols: number, vR: Uint8Array, hR: Uint8Array): BfsResult {
  const N = rows * cols;
  const dist = new Int16Array(N).fill(-1);
  const parent = new Int16Array(N).fill(-1);
  const queue = new Int32Array(N);
  let head = 0, tail = 0;
  dist[start] = 0;
  queue[tail++] = start;
  const colsM1 = cols - 1;
  while (head < tail) {
    const cur = queue[head++];
    const x = (cur / cols) | 0;
    const y = cur - x * cols;
    const d = dist[cur] + 1;
    if (x > 0 && vR[(x - 1) * cols + y]) {
      const n = cur - cols;
      if (dist[n] < 0) { dist[n] = d; parent[n] = cur; queue[tail++] = n; }
    }
    if (x < rows - 1 && vR[x * cols + y]) {
      const n = cur + cols;
      if (dist[n] < 0) { dist[n] = d; parent[n] = cur; queue[tail++] = n; }
    }
    if (y > 0 && hR[x * colsM1 + (y - 1)]) {
      const n = cur - 1;
      if (dist[n] < 0) { dist[n] = d; parent[n] = cur; queue[tail++] = n; }
    }
    if (y < colsM1 && hR[x * colsM1 + y]) {
      const n = cur + 1;
      if (dist[n] < 0) { dist[n] = d; parent[n] = cur; queue[tail++] = n; }
    }
  }
  return { dist, parent };
}

function reconstructCellPath(from: number, to: number, parent: Int16Array, cols: number): [number, number][] {
  const out: [number, number][] = [];
  let cur = to;
  while (cur !== -1 && cur !== from) {
    out.push([(cur / cols) | 0, cur % cols]);
    cur = parent[cur];
  }
  if (cur === from) out.push([(from / cols) | 0, from % cols]);
  out.reverse();
  return out;
}

/**
 * Build a SolveResult from a visit-order history string. Total fuel = sum of
 * BFS distances between consecutive events.
 */
export function solveFromHist(puzzle: ParsedPuzzle, hist: string): SolveResult {
  const { rows, cols, vRoads, hRoads, animals, houses } = puzzle;
  const [sx, sy] = puzzle.start;
  const startPos = sx * cols + sy;

  const vR = new Uint8Array(Math.max(0, rows - 1) * cols);
  for (let i = 0; i < rows - 1; i++)
    for (let j = 0; j < cols; j++)
      vR[i * cols + j] = vRoads[i][j] === '|' ? 1 : 0;
  const hR = new Uint8Array(rows * Math.max(0, cols - 1));
  for (let i = 0; i < rows; i++)
    for (let j = 0; j < cols - 1; j++)
      hR[i * (cols - 1) + j] = hRoads[i][j] === '-' ? 1 : 0;

  const steps: SolveStep[] = [];
  let prevPos = startPos;
  let trunk = 0;
  let totalFuel = 0;

  for (const ch of hist) {
    let target: number;
    let type: 'animal' | 'house';
    let idx: number;
    if (ch >= 'a' && ch <= 'z') {
      idx = ch.charCodeAt(0) - 97;
      target = animals[idx][0] * cols + animals[idx][1];
      type = 'animal';
      trunk |= 1 << idx;
    } else if (ch >= 'A' && ch <= 'Z') {
      idx = ch.charCodeAt(0) - 65;
      target = houses[idx][0] * cols + houses[idx][1];
      type = 'house';
      trunk &= ~(1 << idx);
    } else {
      continue;
    }
    const { dist, parent } = bfs(prevPos, rows, cols, vR, hR);
    const d = dist[target];
    if (d < 0) throw new Error(`solveFromHist: target unreachable for char '${ch}'`);
    totalFuel += d;
    const path = reconstructCellPath(prevPos, target, parent, cols);
    steps.push({ type, idx, pos: [(target / cols) | 0, target % cols], path, trunkAfter: trunk });
    prevPos = target;
  }

  return { fuel: totalFuel, steps };
}

