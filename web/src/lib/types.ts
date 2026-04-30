export type Cell = '.' | 'X' | string; // 'a'..'z' = animal, 'A'..'Z' = house, '.' = empty road, '' = no cell

export interface ParsedPuzzle {
  rows: number;
  cols: number;
  /** rows x cols, each cell is one char: '.', 'X', 'a'-'z', 'A'-'Z', or '' */
  grid: string[][];
  /** (rows-1) x cols, '|' or 'x' indicating presence of vertical road between (i,y) and (i+1,y) */
  vRoads: string[][];
  /** rows x (cols-1), '-' or 'x' indicating presence of horizontal road between (x,i) and (x,i+1) */
  hRoads: string[][];
  start: [number, number];
  /** index → [row, col] */
  animals: [number, number][];
  houses: [number, number][];
  /** mean hue in degrees [0,360) for each pair index, used for hue-based fallbacks. */
  hues: number[];
}

export interface SolveStep {
  type: 'animal' | 'house';
  /** animal/house pair index */
  idx: number;
  /** target cell */
  pos: [number, number];
  /** path of cells from previous position to this target, inclusive of both endpoints */
  path: [number, number][];
  /** remaining trunk after this step (bitmask of pair indices currently in trunk) */
  trunkAfter: number;
}

export interface SolveResult {
  fuel: number;
  steps: SolveStep[];
}
