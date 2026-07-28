<script lang="ts">
  import { onMount } from 'svelte';
  import Board from './Board.svelte';
  import SolutionPlayer from './SolutionPlayer.svelte';
  import AnimalKey from './AnimalKey.svelte';
  import type { ParsedPuzzle, SolveResult } from '../types';
  import { ANIMALS, CAR_SPRITES } from '../animals';
  import {
    loadBuiltinLevels,
    groupLevels,
    type BuiltinLevel,
    type LevelGroup,
  } from '../levels';
  import { loadPrecomputed, lumositytoApp, type PrecomputedEntry } from '../precomputed';
  import { solveFromHist } from '../solver';

  let groups = $state<LevelGroup[] | null>(null);
  let allLevels = $state<BuiltinLevel[] | null>(null);
  let precomputed = $state<PrecomputedEntry[] | null>(null);
  let groupIdx = $state<number>(0);
  let selectedLevel = $state<BuiltinLevel | null>(null);
  /** Optional reflection of the displayed puzzle (0..3, bit0=H, bit1=V), passed
   *  through ?flip=N from the level editor. */
  let flipVariant = $state<number>(0);
  /** Optional letter-index map: spriteFor[level_letter_idx] = user_letter_idx so
   *  the user's chosen sprites land on the right cells when they arrive from the
   *  editor. Default identity. Passed as ?map=ABCDEFGHIJK. */
  let spriteMap = $state<number[] | null>(null);
  let solution = $state<SolveResult | null>(null);
  let solving = $state<boolean>(false);
  let loadError = $state<string | null>(null);

  /** Write current state to the URL (?group=…&level=…&flip=…&map=…). */
  function writeUrl() {
    if (typeof window === 'undefined') return;
    const params = new URLSearchParams();
    if (selectedLevel) {
      params.set('level', String(selectedLevel.idx));
      if (flipVariant !== 0) params.set('flip', String(flipVariant));
      if (spriteMap) {
        params.set('map', spriteMap.map((i) => String.fromCharCode(65 + i)).join(''));
      }
    } else if (groupIdx > 0) {
      params.set('group', String(groupIdx));
    }
    const qs = params.toString();
    const url = qs ? `?${qs}` : window.location.pathname;
    window.history.pushState(null, '', url);
  }

  /** Restore state from the URL. Called once after the levels list is loaded
   *  and on every popstate. */
  function readUrl() {
    if (typeof window === 'undefined' || !groups || !allLevels) return;
    const params = new URLSearchParams(window.location.search);
    const lvl = params.get('level');
    if (lvl !== null) {
      const idx = Number(lvl);
      const target = allLevels[idx];
      if (target) {
        const gi = groups.findIndex((g) => g.levels.some((x) => x.idx === idx));
        if (gi >= 0) groupIdx = gi;
        const f = params.get('flip');
        flipVariant = f !== null ? (Math.max(0, Math.min(3, Number(f))) as number) : 0;
        const m = params.get('map');
        spriteMap = m && /^[A-K]+$/.test(m) ? Array.from(m).map((ch) => ch.charCodeAt(0) - 65) : null;
        openLevel(target, /*pushHistory*/ false);
        return;
      }
    }
    flipVariant = 0;
    spriteMap = null;
    selectedLevel = null;
    solution = null;
    const g = params.get('group');
    if (g !== null) {
      const gi = Number(g);
      groupIdx = gi >= 0 && gi < groups.length ? gi : 0;
    } else {
      groupIdx = 0;
    }
  }

  onMount(async () => {
    try {
      const [all, pre] = await Promise.all([loadBuiltinLevels(), loadPrecomputed()]);
      allLevels = all;
      groups = groupLevels(all);
      precomputed = pre;
      readUrl();
      window.addEventListener('popstate', readUrl);
    } catch (e) {
      console.error(e);
      loadError = (e as Error).message ?? 'Failed to load level data.';
    }
  });

  $effect(() => () => {
    if (typeof window !== 'undefined') window.removeEventListener('popstate', readUrl);
  });

  let currentGroup = $derived(groups?.[groupIdx]);

  function selectGroup(i: number) {
    groupIdx = i;
    selectedLevel = null;
    solution = null;
    writeUrl();
  }

  function openLevel(lv: BuiltinLevel, pushHistory = true) {
    if (pushHistory) {
      // User clicked a tile directly — no editor remap, default orientation.
      flipVariant = 0;
      spriteMap = null;
    }
    selectedLevel = lv;
    solution = null;
    solving = true;
    try {
      const entry = precomputed?.[lv.idx];
      if (!entry || !entry.hist) {
        solving = false;
        if (pushHistory) writeUrl();
        return;
      }
      // entry.hist is in Lumosity convention; the in-app solver convention swaps case.
      const histApp = lumositytoApp(entry.hist);
      const sourcePuzzle = applyFlip(lv.puzzle, flipVariant);
      solution = solveFromHist(sourcePuzzle, histApp);
    } catch (e) {
      console.error(e);
    }
    solving = false;
    if (pushHistory) writeUrl();
  }

  /** Build a Sprites object that points at /sprites/pets/<letter>.png etc. */
  function spritesFor(puzzle: ParsedPuzzle, map: number[] | null) {
    const idxOf = (i: number) => (map ? map[i] ?? i : i);
    return {
      car: CAR_SPRITES.right,
      animals: puzzle.animals.map((_, i) => ANIMALS[idxOf(i)].petSprite),
      houses: puzzle.houses.map((_, i) => ANIMALS[idxOf(i)].houseSprite),
      // Per-slot pet sizes follow the remap too — otherwise a slot showing
      // a different species still uses its original-letter dimensions and
      // appears clipped or off-centre.
      animalSizes: puzzle.animals.map((_, i) => ({
        width: ANIMALS[idxOf(i)].petWidth,
        height: ANIMALS[idxOf(i)].petHeight,
      })),
    };
  }

  /** Apply a 0..3 H/V flip to a parsed puzzle: positions, road indicators, start
   *  cell, and the grid string array. Used when the user arrives via the editor
   *  with `?flip=N`, so what they see matches what they entered. */
  function applyFlip(p: ParsedPuzzle, v: number): ParsedPuzzle {
    if (v === 0) return p;
    const tx = (r: number, c: number): [number, number] => {
      let nr = r, nc = c;
      if (v & 1) nc = p.cols - 1 - c;
      if (v & 2) nr = p.rows - 1 - r;
      return [nr, nc];
    };
    const grid: string[][] = Array.from({ length: p.rows }, () => Array(p.cols).fill(''));
    for (let r = 0; r < p.rows; r++) {
      for (let c = 0; c < p.cols; c++) {
        const [nr, nc] = tx(r, c);
        grid[nr][nc] = p.grid[r][c];
      }
    }
    // hRoads is rows x (cols-1). After H-flip cols reverse and h-road between
    // (r, c) and (r, c+1) maps to between (r, cols-1-c-1) and (r, cols-1-c).
    const hRoads: string[][] = Array.from({ length: p.rows }, () => Array(Math.max(0, p.cols - 1)).fill('x'));
    for (let r = 0; r < p.rows; r++) {
      for (let c = 0; c < p.cols - 1; c++) {
        const nr = (v & 2) ? p.rows - 1 - r : r;
        const nc = (v & 1) ? p.cols - 2 - c : c;
        hRoads[nr][nc] = p.hRoads[r][c];
      }
    }
    // vRoads is (rows-1) x cols.
    const vRoads: string[][] = Array.from({ length: Math.max(0, p.rows - 1) }, () => Array(p.cols).fill('x'));
    for (let r = 0; r < p.rows - 1; r++) {
      for (let c = 0; c < p.cols; c++) {
        const nr = (v & 2) ? p.rows - 2 - r : r;
        const nc = (v & 1) ? p.cols - 1 - c : c;
        vRoads[nr][nc] = p.vRoads[r][c];
      }
    }
    return {
      ...p,
      grid,
      hRoads,
      vRoads,
      start: tx(p.start[0], p.start[1]),
      animals: p.animals.map(([r, c]) => tx(r, c)),
      houses: p.houses.map(([r, c]) => tx(r, c)),
    };
  }

  let displayedPuzzle = $derived(selectedLevel ? applyFlip(selectedLevel.puzzle, flipVariant) : null);

  function backToList() {
    selectedLevel = null;
    solution = null;
    writeUrl();
  }

  /** Levels in the current group, used to compute coordinates of canned solution etc. */
</script>

<div class="flex flex-col gap-2">
  {#if loadError}
    <div class="panel border-red-500/40 p-4 text-red-700 dark:text-red-300">
      <div class="font-semibold mb-1">Couldn't load levels</div>
      <p class="text-sm">{loadError}</p>
    </div>
  {:else if !groups}
    <div class="panel p-6 text-center text-gray-600 dark:text-gray-400">Loading 2,558 levels…</div>
  {:else if selectedLevel}
    <!-- Detail view -->
    <div class="panel p-4">
      <div class="flex items-center justify-between mb-2 flex-wrap gap-2">
        <div class="flex items-center gap-2">
          <button class="btn-ghost text-xs px-2 py-1" onclick={backToList}>← back to list</button>
          <span class="text-xs text-gray-600 dark:text-gray-400">
            Level <span class="font-mono text-gray-800 dark:text-gray-200">#{selectedLevel.idx}</span>
            <span class="mx-1.5 text-gray-600">·</span>
            {selectedLevel.cols}×{selectedLevel.rows}
          </span>
        </div>
        {#if solution}
          {@const beats = solution.fuel < selectedLevel.parMoves}
          <div class="flex items-center gap-1.5 text-xs">
            <span class="rounded-md border border-gray-200 bg-gray-50 px-1.5 py-0.5 dark:border-white/10 dark:bg-white/5">
              <span class="text-gray-500">in-game fuel</span>
              <span class="ml-1 font-mono text-gray-900 dark:text-gray-100">{selectedLevel.parMoves}</span>
            </span>
            {#if beats}
              <span class="rounded-md border border-yellow-500/40 bg-yellow-500/15 px-1.5 py-0.5 text-yellow-800 dark:text-yellow-300">
                <span class="text-yellow-700 dark:text-yellow-400/80">optimal</span>
                <span class="font-mono ml-1">{solution.fuel}</span>
                <span class="ml-1">⚠ saves {selectedLevel.parMoves - solution.fuel}</span>
              </span>
            {:else}
              <span class="rounded-md border border-emerald-500/40 bg-emerald-500/15 px-1.5 py-0.5 text-emerald-800 dark:text-emerald-300">
                <span class="text-emerald-700 dark:text-emerald-400/80">optimal</span>
                <span class="font-mono ml-1">{solution.fuel}</span>
              </span>
            {/if}
          </div>
        {/if}
      </div>

      {#if solving}
        <div class="text-sm text-gray-600 dark:text-gray-400">Solving…</div>
      {:else if solution && displayedPuzzle}
        {@const ourHist = precomputed?.[selectedLevel.idx]?.hist ?? ''}
        <SolutionPlayer
          puzzle={displayedPuzzle}
          sprites={spritesFor(displayedPuzzle, spriteMap)}
          {solution}
          lumosityHist={selectedLevel.rawSolution ?? undefined}
          ourHist={ourHist || undefined}
        />
      {/if}

      <div class="mt-4 border-t border-gray-200 pt-3 dark:border-white/5">
        <div class="text-[11px] uppercase tracking-wider text-gray-500 mb-2">Animals in this level</div>
        <AnimalKey layout="compact" highlight={(1 << selectedLevel.petCount) - 1} />
      </div>
    </div>
  {:else}
    <!-- Group selector -->
    <div class="panel px-3 py-2">
      <div class="text-[11px] uppercase tracking-wider text-gray-500 mb-2">Browse by difficulty</div>
      <div class="flex flex-wrap gap-1.5">
        {#each groups as g, i}
          <button
            class="rounded-md border px-2.5 py-1 text-xs transition-colors {groupIdx === i
              ? 'border-car bg-car text-gray-950'
              : 'border-gray-300 bg-gray-50 text-gray-700 hover:border-car dark:border-white/10 dark:bg-white/5 dark:text-gray-300'}"
            onclick={() => selectGroup(i)}
          >
            <span class="font-mono">{g.petCount}</span>
            <span class="opacity-70">animals · {g.cols}×{g.rows}</span>
            <span class="opacity-50 ml-1">({g.levels.length})</span>
          </button>
        {/each}
      </div>
    </div>

    <!-- Level grid -->
    {#if currentGroup}
      <div class="panel px-3 py-2">
        <div class="flex items-center justify-between mb-2 text-xs text-gray-500">
          <span><span class="font-mono text-gray-700 dark:text-gray-300">{currentGroup.levels.length}</span> levels</span>
          <span>fuel range {currentGroup.levels.reduce((m, l) => Math.min(m, l.parMoves), Infinity)}–{currentGroup.levels.reduce((m, l) => Math.max(m, l.parMoves), 0)}</span>
        </div>
        <div class="grid gap-0.5" style="grid-template-columns: repeat(auto-fill, minmax(2.5rem, 1fr));">
          {#each currentGroup.levels as lv}
            <!--
              Real <a href> so right-click "Open in new tab" / middle-click / ⌘-click work.
              On a plain left click we preventDefault and update state in place so it stays
              SPA-style; modifier-clicks fall through to the browser as expected.

              Tile colour-codes whether the in-game route is sub-optimal and by how much.
            -->
            {@const pre = precomputed?.[lv.idx]}
            {@const cf = pre?.cf ?? null}
            {@const bfs = pre?.fuel ?? null}
            {@const wasted = cf != null && bfs != null ? cf - bfs : 0}
            <a
              class="flex aspect-square flex-col items-center justify-center rounded-[3px] border font-mono leading-none no-underline transition-colors {wasted === 0
                ? 'border-gray-300 bg-gray-50 text-gray-700 hover:border-car hover:bg-orange-50 dark:border-white/10 dark:bg-white/5 dark:text-gray-300 dark:hover:bg-car/15'
                : wasted === 1
                  ? 'border-yellow-500/50 bg-yellow-100 text-yellow-900 dark:bg-yellow-500/15 dark:text-yellow-100'
                  : 'border-red-500 bg-red-100 text-red-900 dark:bg-red-500 dark:text-red-100'}"
              href={`?level=${lv.idx}`}
              onclick={(e) => {
                if (e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || e.button !== 0) return;
                e.preventDefault();
                openLevel(lv);
              }}
              title={`#${lv.idx} · fuel ${lv.parMoves}${wasted > 0 ? ` (BFS ${bfs}, wastes ${wasted})` : ''} · ${lv.rawSolution}`}
            >
              <span class="text-[10px]">{lv.idx}</span>
              <span class="text-[7px] opacity-70 leading-tight">fuel {lv.parMoves}</span>
            </a>
          {/each}
        </div>
      </div>
    {/if}

    <div class="panel p-3">
      <div class="text-[11px] uppercase tracking-wider text-gray-500 mb-2">Animal key</div>
      <AnimalKey />
    </div>
  {/if}
</div>
