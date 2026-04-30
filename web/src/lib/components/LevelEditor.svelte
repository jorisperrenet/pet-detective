<script lang="ts">
  import { onMount } from 'svelte';
  import {
    ANIMALS, BOARD_BACKGROUND, CAR_SIZE_PX, CAR_SPRITES,
    HOUSE_SIZE_PX, HOUSE_Y_OFFSET, PET_Y_OFFSET, roadSpriteUrl,
  } from '../animals';
  import { loadBuiltinLevels, type BuiltinLevel } from '../levels';

  /** Pet/house pair the user has placed so far. */
  type Pair = { pet: [number, number]; house: [number, number] };

  /** Six valid grid sizes the bundle uses (rows × cols, portrait orientation). */
  const SIZES: { rows: number; cols: number }[] = [
    { rows: 3, cols: 3 },
    { rows: 4, cols: 3 },
    { rows: 5, cols: 3 },
    { rows: 6, cols: 3 },
    { rows: 5, cols: 4 },
    { rows: 6, cols: 4 },
  ];

  let chosenSize = $state<{ rows: number; cols: number } | null>(null);
  /** Hovered grid size button, used to preview that grid before the user clicks. */
  let hoverSize = $state<{ rows: number; cols: number } | null>(null);
  let pairs = $state<Pair[]>([]);
  let pendingPet = $state<[number, number] | null>(null); // pet placed but house not yet
  let allBuiltin = $state<BuiltinLevel[] | null>(null);
  let stoppedEarly = $state<boolean>(false);
  /** When the user picks one of several candidates, we pin to that level so the
   *  rest of the editor flow continues against it (and auto-fill / dots / car all
   *  use that level's data). The user still has to place the remaining animals. */
  let pickedLevel = $state<BuiltinLevel | null>(null);

  /** localStorage key for persisting editor state across reloads. */
  const STORAGE_KEY = 'pet-detective.editor.state.v1';
  /** Once restoreState has had its turn, the save effect is allowed to write.
   *  Without this gate the initial-empty-state effect run would clobber the
   *  saved snapshot before it gets read. */
  let restoredFromStorage = $state(false);

  type StoredState = {
    chosenSize: { rows: number; cols: number } | null;
    pairs: Pair[];
    pendingPet: [number, number] | null;
    stoppedEarly: boolean;
    pickedLevelIdx: number | null;
  };

  function saveState() {
    if (typeof window === 'undefined') return;
    try {
      const s: StoredState = {
        chosenSize,
        pairs,
        pendingPet,
        stoppedEarly,
        pickedLevelIdx: pickedLevel?.idx ?? null,
      };
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
    } catch { /* private mode / quota — ignore */ }
  }

  function restoreState(builtins: BuiltinLevel[]) {
    if (typeof window === 'undefined') { restoredFromStorage = true; return; }
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (!raw) { restoredFromStorage = true; return; }
      const s = JSON.parse(raw) as Partial<StoredState>;
      if (s.chosenSize && typeof s.chosenSize.rows === 'number' && typeof s.chosenSize.cols === 'number') {
        chosenSize = s.chosenSize;
      }
      if (Array.isArray(s.pairs)) pairs = s.pairs as Pair[];
      if (Array.isArray(s.pendingPet) && s.pendingPet.length === 2) pendingPet = s.pendingPet as [number, number];
      if (typeof s.stoppedEarly === 'boolean') stoppedEarly = s.stoppedEarly;
      if (typeof s.pickedLevelIdx === 'number') {
        pickedLevel = builtins.find((b) => b.idx === s.pickedLevelIdx) ?? null;
      }
    } catch { /* corrupt JSON — ignore and start fresh */ }
    finally { restoredFromStorage = true; }
  }

  onMount(async () => {
    try {
      allBuiltin = await loadBuiltinLevels();
      restoreState(allBuiltin);
    } catch (e) { console.error(e); }
  });

  // Persist any state change — but only AFTER restoreState has run, otherwise
  // the initial empty-state effect run would overwrite the saved snapshot.
  $effect(() => {
    void chosenSize; void pairs; void pendingPet; void stoppedEarly; void pickedLevel;
    if (!restoredFromStorage) return;
    saveState();
  });

  // ──────────────────────────────────────────────────────────────────────────
  // Matching: filter built-ins of the same dimensions whose pet/house cell
  // multisets are a SUPERSET of what the user has entered. Letter assignments
  // are randomised at runtime, so we only check unordered (pet_pos, house_pos)
  // pairs. The user's pair (Pa, Ha) is considered to match if there's some
  // pair (Pb, Hb) in the level with Pa==Pb && Ha==Hb. With multiple user
  // pairs, we need a one-to-one assignment — solved by a small DFS.
  // ──────────────────────────────────────────────────────────────────────────

  function posKey(p: [number, number]): number { return p[0] * 16 + p[1]; }

  /** Greedy / backtracking: can we assign each user pair to a distinct level pair? */
  function pairsMatch(user: Pair[], levelPairs: Pair[]): boolean {
    if (user.length === 0) return true;
    if (user.length > levelPairs.length) return false;
    const used = new Array(levelPairs.length).fill(false);
    const dfs = (k: number): boolean => {
      if (k === user.length) return true;
      const u = user[k];
      const upk = posKey(u.pet);
      const uhk = posKey(u.house);
      for (let i = 0; i < levelPairs.length; i++) {
        if (used[i]) continue;
        const lp = levelPairs[i];
        if (posKey(lp.pet) !== upk || posKey(lp.house) !== uhk) continue;
        used[i] = true;
        if (dfs(k + 1)) return true;
        used[i] = false;
      }
      return false;
    };
    return dfs(0);
  }

  /** Build a level's pairs once. */
  function levelPairs(lv: BuiltinLevel): Pair[] {
    const pets = lv.puzzle.animals;
    const hs = lv.puzzle.houses;
    const out: Pair[] = [];
    for (let i = 0; i < pets.length; i++) out.push({ pet: pets[i], house: hs[i] });
    return out;
  }

  /**
   * In-game the puzzle data is reused under up to four reflections (identity,
   * horizontal flip, vertical flip, and both) so that a single base level
   * shows up four ways. We try all four when matching.
   */
  type Variant = 0 | 1 | 2 | 3;
  function transform(p: [number, number], v: Variant, rows: number, cols: number): [number, number] {
    let [r, c] = p;
    if (v & 1) c = cols - 1 - c; // horizontal flip
    if (v & 2) r = rows - 1 - r; // vertical flip
    return [r, c];
  }
  function variantPairs(lv: BuiltinLevel, v: Variant): Pair[] {
    if (v === 0) return levelPairs(lv);
    const rows = lv.rows;
    const cols = lv.cols;
    return levelPairs(lv).map(({ pet, house }) => ({
      pet: transform(pet, v, rows, cols),
      house: transform(house, v, rows, cols),
    }));
  }

  /** True if there's some pet in the level placed at `pos` not already used by `usedSet`. */
  function petAvailableAt(pos: [number, number], allPairs: Pair[], usedIdx: Set<number>): boolean {
    for (let i = 0; i < allPairs.length; i++) {
      if (usedIdx.has(i)) continue;
      if (allPairs[i].pet[0] === pos[0] && allPairs[i].pet[1] === pos[1]) return true;
    }
    return false;
  }

  /**
   * Return the set of cells (encoded `r*32+c`) that still contain a pet across
   * the current candidate levels — used as a small dot-overlay so the user can
   * see at a glance where placements are still possible. Helps catch orientation
   * mistakes (clicking a cell with no pet in any matching level).
   */
  function possiblePetCells(cands: BuiltinLevel[], userPairs: Pair[]): Set<number> {
    const out = new Set<number>();
    for (const lv of cands) {
      // Same logic as candidate match — but for hints we want positions from
      // EVERY reflection that's compatible, so the user can see all options.
      for (const v of [0, 1, 2, 3] as Variant[]) {
        const lp = variantPairs(lv, v);
        const used = new Array(lp.length).fill(false);
        const dfs = (k: number): boolean => {
          if (k === userPairs.length) return true;
          const u = userPairs[k];
          for (let i = 0; i < lp.length; i++) {
            if (used[i]) continue;
            if (lp[i].pet[0] !== u.pet[0] || lp[i].pet[1] !== u.pet[1]) continue;
            if (lp[i].house[0] !== u.house[0] || lp[i].house[1] !== u.house[1]) continue;
            used[i] = true;
            if (dfs(k + 1)) return true;
            used[i] = false;
          }
          return false;
        };
        if (!dfs(0)) continue;
        for (let i = 0; i < lp.length; i++) {
          if (used[i]) continue;
          out.add(lp[i].pet[0] * 32 + lp[i].pet[1]);
        }
      }
    }
    return out;
  }

  /** Try each level under all four reflections (identity, H-flip, V-flip, HV-flip). */
  function levelMatchesAnyVariant(lv: BuiltinLevel, userPairs: Pair[], pending: [number, number] | null): boolean {
    for (const v of [0, 1, 2, 3] as Variant[]) {
      const lp = variantPairs(lv, v);
      if (!pairsMatch(userPairs, lp)) continue;
      if (!pending) return true;
      // pendingPet must also fit: re-run the inner DFS with this variant.
      const used = new Array(lp.length).fill(false);
      const dfs = (k: number): boolean => {
        if (k === userPairs.length) {
          return petAvailableAt(pending, lp, new Set(used.flatMap((u, i) => (u ? [i] : []))));
        }
        const u = userPairs[k];
        for (let i = 0; i < lp.length; i++) {
          if (used[i]) continue;
          if (lp[i].pet[0] !== u.pet[0] || lp[i].pet[1] !== u.pet[1]) continue;
          if (lp[i].house[0] !== u.house[0] || lp[i].house[1] !== u.house[1]) continue;
          used[i] = true;
          if (dfs(k + 1)) return true;
          used[i] = false;
        }
        return false;
      };
      if (dfs(0)) return true;
    }
    return false;
  }

  let candidates = $derived.by(() => {
    // While the user is just hovering size buttons (no chosenSize yet), still
    // surface the count of levels for the size they're hovering — gives them a
    // sense of how big each pool is before they commit.
    const size = chosenSize ?? hoverSize;
    if (!allBuiltin || !size) return [] as BuiltinLevel[];
    // If the user picked one of the multiple candidates, narrow to just it.
    if (pickedLevel) {
      if (pickedLevel.rows === size.rows && pickedLevel.cols === size.cols
        && levelMatchesAnyVariant(pickedLevel, pairs, pendingPet)) {
        return [pickedLevel];
      }
      return [];
    }
    const minPets = pairs.length + (pendingPet ? 1 : 0);
    return allBuiltin.filter((lv) => {
      if (lv.cols !== size.cols || lv.rows !== size.rows) return false;
      if (stoppedEarly && lv.petCount !== pairs.length) return false;
      if (!stoppedEarly && lv.petCount < minPets) return false;
      return levelMatchesAnyVariant(lv, pairs, pendingPet);
    });
  });

  // Step model
  let step = $derived.by<'pet' | 'house' | 'done'>(() => {
    if (!chosenSize) return 'pet';
    if (stoppedEarly) return 'done';
    if (pendingPet) return 'house';
    return 'pet';
  });
  let nextLetterIndex = $derived(pairs.length);
  let nextLetter = $derived(ANIMALS[Math.min(nextLetterIndex, 10)].letter);

  function chooseSize(s: { rows: number; cols: number }) {
    chosenSize = s;
    pairs = [];
    pendingPet = null;
    stoppedEarly = false;
    pickedLevel = null;
  }

  function clickCell(r: number, c: number) {
    if (!chosenSize) return;
    if (stoppedEarly) return;
    // Must not double-click an already-occupied cell
    for (const p of pairs) {
      if (p.pet[0] === r && p.pet[1] === c) return;
      if (p.house[0] === r && p.house[1] === c) return;
    }
    if (pendingPet && pendingPet[0] === r && pendingPet[1] === c) return;

    if (pendingPet === null) {
      pendingPet = [r, c];
    } else {
      pairs = [...pairs, { pet: pendingPet, house: [r, c] }];
      pendingPet = null;
      // Auto-stop if the level is uniquely identified.
      // (User can also click "no more animals" any time.)
    }
  }

  function undoLast() {
    // One atomic action at a time: a placed house first goes back to "pet placed,
    // house pending"; then a placed pet clears entirely. Never both at once.
    if (pendingPet) {
      pendingPet = null;
    } else if (pairs.length > 0) {
      const last = pairs[pairs.length - 1];
      pairs = pairs.slice(0, -1);
      pendingPet = last.pet;
    }
    stoppedEarly = false;
  }

  function declareDone() {
    if (pendingPet) return; // can't stop while a pet is half-placed
    stoppedEarly = true;
  }

  function clearAll() {
    chosenSize = null;
    pairs = [];
    pendingPet = null;
    stoppedEarly = false;
    pickedLevel = null;
  }

  /**
   * Returns `{ kind, idx }` where idx is which animal slot (0..10) is displayed
   * at this cell. Letters never appear in the UI — only the sprite for that idx.
   */
  /**
   * If the user's placements + pendingPet narrow to exactly one (level, variant,
   * letter-assignment) tuple, return it. We use this to (a) show the level's real
   * road network in the editor canvas and (b) hand off to the Solve page with the
   * right orientation + sprite-letter map.
   */
  type Lock = {
    level: BuiltinLevel;
    variant: Variant;
    /** assignment[i] = level-pair index that user-pair `i` maps to, in user-input order. */
    assignment: number[];
  };

  function tryLock(): Lock | null {
    if (candidates.length !== 1) return null;
    const lv = candidates[0];
    let chosen: Lock | null = null;
    for (const v of [0, 1, 2, 3] as Variant[]) {
      const lp = variantPairs(lv, v);
      const used = new Array(lp.length).fill(false);
      const assignment: number[] = [];
      const dfs = (k: number): boolean => {
        if (k === pairs.length) {
          if (pendingPet) {
            return petAvailableAt(pendingPet, lp, new Set(used.flatMap((u, i) => (u ? [i] : []))));
          }
          return true;
        }
        const u = pairs[k];
        for (let i = 0; i < lp.length; i++) {
          if (used[i]) continue;
          if (lp[i].pet[0] !== u.pet[0] || lp[i].pet[1] !== u.pet[1]) continue;
          if (lp[i].house[0] !== u.house[0] || lp[i].house[1] !== u.house[1]) continue;
          used[i] = true;
          assignment.push(i);
          if (dfs(k + 1)) return true;
          used[i] = false;
          assignment.pop();
        }
        return false;
      };
      if (!dfs(0)) continue;
      const candidateLock: Lock = { level: lv, variant: v, assignment: assignment.slice() };
      // If the user has manually picked this level, lock onto the first valid
      // variant — the user has confirmed they want this puzzle.
      if (pickedLevel) return candidateLock;
      if (chosen) return null; // More than one variant fits — keep blank canvas.
      chosen = candidateLock;
    }
    return chosen;
  }

  let lock = $derived(tryLock());

  /** Once we know the level + orientation, the car sits at the variant-mapped
   *  start cell of that level. */
  let lockedStart = $derived.by<[number, number] | null>(() => {
    if (!lock) return null;
    const p = lock.level.puzzle;
    return transform(p.start, lock.variant, p.rows, p.cols);
  });

  /**
   * Auto-complete: once we know the level/variant/assignment, every pet the user
   * places implies its matching house. Don't make them click twice.
   */
  $effect(() => {
    if (!lock || !pendingPet) return;
    const lp = variantPairs(lock.level, lock.variant);
    const usedIdx = new Set<number>(lock.assignment);
    for (let i = 0; i < lp.length; i++) {
      if (usedIdx.has(i)) continue;
      if (lp[i].pet[0] === pendingPet[0] && lp[i].pet[1] === pendingPet[1]) {
        pairs = [...pairs, { pet: pendingPet, house: lp[i].house }];
        pendingPet = null;
        return;
      }
    }
  });

  /**
   * When only one animal is left and the level is locked, the final pair is
   * forced — its (pet, house) is the only slot of `lock.assignment` not yet
   * used. Place it automatically so the user doesn't have to.
   */
  $effect(() => {
    if (!lock || pendingPet) return;
    if (pairs.length !== lock.level.petCount - 1) return;
    const lp = variantPairs(lock.level, lock.variant);
    const usedIdx = new Set<number>(lock.assignment);
    for (let i = 0; i < lp.length; i++) {
      if (usedIdx.has(i)) continue;
      pairs = [...pairs, { pet: lp[i].pet, house: lp[i].house }];
      return;
    }
  });

  /**
   * Auto-redirect: once every animal in the locked level is placed, jump to the
   * solve page in the matching orientation + sprite mapping. Also clear the
   * persisted editor state so a fresh visit to "Find a level" starts blank.
   */
  $effect(() => {
    if (!lock || pendingPet) return;
    if (pairs.length !== lock.level.petCount) return;
    const url = solveUrl(lock);
    queueMicrotask(() => {
      if (typeof window === 'undefined') return;
      try { window.localStorage.removeItem(STORAGE_KEY); } catch { /* ignore */ }
      window.history.pushState(null, '', url);
      window.dispatchEvent(new PopStateEvent('popstate'));
    });
  });

  /** Apply variant transform to a single (r, c). */
  function tx(p: [number, number], v: Variant, rows: number, cols: number): [number, number] {
    return transform(p, v, rows, cols);
  }

  /** Compute the connectivity mask of cell (r, c) under the locked level + variant.
   *  Reads the level's vRoads/hRoads, transforms cell coords back to the level's
   *  un-flipped frame, then re-applies the variant's mask flip. */
  function maskUnderLock(r: number, c: number, lk: Lock): number {
    const { level, variant } = lk;
    const p = level.puzzle;
    const [lr, lc] = tx([r, c], variant, p.rows, p.cols);
    let m = 0;
    if (lr > 0 && p.vRoads[lr - 1]?.[lc] === '|') m |= 8;
    if (lr < p.rows - 1 && p.vRoads[lr]?.[lc] === '|') m |= 4;
    if (lc > 0 && p.hRoads[lr]?.[lc - 1] === '-') m |= 2;
    if (lc < p.cols - 1 && p.hRoads[lr]?.[lc] === '-') m |= 1;
    // H-flip swaps L and R; V-flip swaps U and D.
    if (variant & 1) m = (m & 0b1100) | ((m & 1) << 1) | ((m & 2) >> 1);
    if (variant & 2) m = (m & 0b0011) | ((m & 8) >> 1) | ((m & 4) << 1);
    return m;
  }

  /** Build the ?level&flip&map URL for the locked level, applying the user's
   *  letter-to-level-letter assignment so the solve page shows their sprites
   *  in the right cells under the right orientation. */
  function solveUrl(lk: Lock): string {
    const lp = levelPairs(lk.level);
    // m[level_letter_idx] = user_letter_idx (which sprite to use)
    const m: (number | null)[] = new Array(lp.length).fill(null);
    for (let k = 0; k < pairs.length; k++) m[lk.assignment[k]] = k;
    const usedUser = new Set(m.filter((x): x is number => x !== null));
    let nextUser = 0;
    for (let i = 0; i < m.length; i++) {
      if (m[i] !== null) continue;
      while (usedUser.has(nextUser)) nextUser++;
      m[i] = nextUser;
      usedUser.add(nextUser);
    }
    const mapStr = m.map((x) => String.fromCharCode(65 + (x as number))).join('');
    const params = new URLSearchParams();
    params.set('level', String(lk.level.idx));
    if (lk.variant !== 0) params.set('flip', String(lk.variant));
    params.set('map', mapStr);
    return `?${params.toString()}`;
  }

  let pendingPetCellHints = $derived(
    step !== 'done' && pendingPet === null && chosenSize
      ? possiblePetCells(candidates, pairs)
      : new Set<number>(),
  );

  function cellInfo(r: number, c: number): { kind: 'empty' | 'pet' | 'house' | 'pending'; idx: number } {
    if (pendingPet && pendingPet[0] === r && pendingPet[1] === c) {
      return { kind: 'pending', idx: pairs.length };
    }
    for (let i = 0; i < pairs.length; i++) {
      if (pairs[i].pet[0] === r && pairs[i].pet[1] === c) return { kind: 'pet', idx: i };
      if (pairs[i].house[0] === r && pairs[i].house[1] === c) return { kind: 'house', idx: i };
    }
    return { kind: 'empty', idx: -1 };
  }
</script>

<div class="flex flex-col gap-3">
  <div class="panel p-3">
    {#if chosenSize}
      <div class="text-xs text-gray-400">Grid: <span class="font-mono">{chosenSize.cols}×{chosenSize.rows}</span></div>
    {:else}
      <div class="text-xs text-gray-500 mb-2">Grid sizes:</div>
      <div class="flex flex-wrap gap-1.5">
        {#each SIZES as s}
          <button
            class="px-3 py-1.5 rounded-md text-xs border bg-white/5 border-white/10 text-gray-300 hover:border-car/60 hover:text-white"
            onclick={() => chooseSize(s)}
            onmouseenter={() => (hoverSize = s)}
            onmouseleave={() => { if (hoverSize === s) hoverSize = null; }}
            onfocus={() => (hoverSize = s)}
            onblur={() => { if (hoverSize === s) hoverSize = null; }}
          >{s.cols}×{s.rows}</button>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Canvas first, then the matches panel directly below — works at every width. -->
  {#if true}
    <!-- Always show *some* grid as a starting point so the canvas isn't empty.
         Default to the largest size (4×6); change to whatever the user is
         hovering, then lock to whatever they click. -->
    {@const defaultSize = SIZES[SIZES.length - 1]}
    {@const previewSize = chosenSize ?? hoverSize ?? defaultSize}
    {@const isPreview = !chosenSize}
    {@const CELL = 250}
    {@const PAD = 20}
    {@const BLEED = 4}
    {@const cellX = (c: number) => PAD + c * CELL + CELL / 2}
    {@const cellY = (r: number) => PAD + r * CELL + CELL / 2}
    {@const ROWS = previewSize.rows}
    {@const COLS = previewSize.cols}
    <!-- Edge-aware connectivity mask: bit U=8, D=4, L=2, R=1 only when there's a neighbour. -->
    {@const maskAt = (r: number, c: number) => (
      (r > 0 ? 8 : 0)
      | (r < ROWS - 1 ? 4 : 0)
      | (c > 0 ? 2 : 0)
      | (c < COLS - 1 ? 1 : 0)
    )}
    <!-- Match Board.svelte: look up the L/R-swapped sprite, then mirror it, so shadows fall on the correct screen side. -->
    {@const lookupOf = (m: number) => (m & 0b1100) | ((m & 1) << 1) | ((m & 2) >> 1)}
    {@const ROAD_W = 86}
    {@const renderMaskAt = (r: number, c: number) => lock ? maskUnderLock(r, c, lock) : maskAt(r, c)}

    <div class="panel p-3">
      <div class="text-[11px] uppercase tracking-wider text-gray-500 mb-2">Matches</div>
      {#if !allBuiltin}
        <div class="text-sm text-gray-400">Loading level database…</div>
      {:else if pairs.length === 0}
        <div class="text-sm text-gray-400">
          {candidates.length} levels of size {previewSize.cols}×{previewSize.rows} in the database.
        </div>
      {:else if candidates.length === 0}
        <div class="text-sm text-red-300">
          No built-in level matches these placements. Did you mis-click? Press <em>Undo last</em>.
        </div>
      {:else if candidates.length === 1}
        {@const lv = candidates[0]}
        <div class="flex items-center justify-between gap-3 flex-wrap">
          <div class="text-sm">
            <span class="text-emerald-300">Uniquely identified:</span>
            <span class="font-mono text-emerald-200 ml-1">level #{lv.idx}</span>
            <span class="text-gray-500"> · in-game fuel <span class="text-gray-200">{lv.parMoves}</span></span>
            {#if lock}
              <span class="text-gray-500 ml-2">· orientation
                <span class="text-gray-300 font-mono">
                  {lock.variant === 0 ? 'as-is' : lock.variant === 1 ? 'H-flip' : lock.variant === 2 ? 'V-flip' : '180°'}
                </span></span>
            {/if}
          </div>
          {#if lock}
            <a class="btn-primary text-sm" href={solveUrl(lock)}>Solve this puzzle →</a>
          {/if}
        </div>
        {#if lock}
          <div class="mt-3">
            <div class="text-[11px] uppercase tracking-wider text-gray-500 mb-1.5">
              Letter mapping (in-game letter → animal you placed)
            </div>
            <div class="flex flex-wrap gap-1.5">
              {#each Array(lv.petCount) as _, levelLetter}
                {@const userIdx = lock.assignment.findIndex((a) => a === levelLetter)}
                <div class="flex items-center gap-1 px-2 py-1 rounded bg-white/5 border border-white/10 text-xs">
                  <span class="font-mono text-gray-400">{String.fromCharCode(65 + levelLetter)}</span>
                  <span class="text-gray-600">→</span>
                  {#if userIdx >= 0}
                    <img src={ANIMALS[userIdx].petSprite} alt="" class="h-5 w-auto" />
                  {:else}
                    <span class="text-gray-500">?</span>
                  {/if}
                </div>
              {/each}
            </div>
            {#if pairs.length < lv.petCount}
              <div class="mt-2 text-xs text-gray-500">
                {lv.petCount - pairs.length} animal{lv.petCount - pairs.length === 1 ? '' : 's'} left to place — orange dots show where they sit. (You can also just hit <em>Solve</em> now.)
              </div>
            {/if}
          </div>
        {/if}
      {:else}
        <div class="text-sm text-gray-300">
          <span class="font-mono text-gray-100">{candidates.length}</span>
          levels still match. Add another pet/house pair to narrow it down.
          {#if candidates.length <= 8}
            <ul class="mt-2 grid grid-cols-2 sm:grid-cols-4 gap-1.5 text-xs">
              {#each candidates as lv}
                <li>
                  <button
                    type="button"
                    class="block w-full text-left px-2 py-1 rounded bg-white/5 border border-white/10 text-gray-300 font-mono hover:border-car/60"
                    onclick={() => { pickedLevel = lv; }}
                    title="Pick this level — you'll need to fill in the remaining animals before solving"
                  >
                    #{lv.idx} · {lv.petCount} animals · fuel {lv.parMoves}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    </div>

    <div class="panel p-3">
      <div class="flex items-center gap-2 mb-3 flex-wrap">
        {#if !chosenSize}
          <span class="text-sm text-gray-400">Select the grid size first.</span>
        {:else if step === 'pet' && nextLetterIndex < 11}
          <span class="text-sm text-gray-300 inline-flex items-center gap-2">
            Click where this animal sits:
            <img src={ANIMALS[nextLetterIndex].petSprite} alt="next pet"
              class="h-7 w-auto inline-block align-middle" />
          </span>
        {:else if step === 'house'}
          <span class="text-sm text-gray-300 inline-flex items-center gap-2">
            Click the matching house:
            <img src={ANIMALS[pairs.length].houseSprite} alt="next house"
              class="h-7 w-auto inline-block align-middle" />
          </span>
        {:else if step === 'done'}
          <span class="text-sm text-gray-400">Done — {pairs.length} pet/house pair{pairs.length === 1 ? '' : 's'} placed.</span>
        {/if}
        {#if lock}
          <span class="text-xs text-gray-500 ml-2">
            <span class="font-mono text-emerald-300">{pairs.length}</span>
            <span class="text-gray-500"> / {lock.level.petCount}</span> placed — house fills in automatically
          </span>
        {/if}
        <span class="ml-auto flex gap-2">
          {#if !stoppedEarly && !lock && pairs.length >= 2 && !pendingPet}
            <button class="btn-ghost text-xs" onclick={declareDone}>No more animals</button>
          {/if}
          {#if pairs.length > 0 || pendingPet}
            <button class="btn-ghost text-xs" onclick={undoLast}>Undo last</button>
          {/if}
          <button class="btn-ghost text-xs" onclick={clearAll}>Reset</button>
        </span>
      </div>

      <!-- Clickable board: same background + full-cross road tile in every cell, then sprites on top -->
      <svg
        class="block w-full h-auto select-none mx-auto {isPreview ? 'opacity-70 pointer-events-none' : ''}"
        viewBox="0 0 {previewSize.cols * CELL + PAD * 2} {previewSize.rows * CELL + PAD * 2}"
        xmlns="http://www.w3.org/2000/svg"
        style="max-height: min(56vh, 480px)"
      >
        <defs>
          <clipPath id="editorBoardClip">
            <rect x="0" y="0" width={previewSize.cols * CELL + PAD * 2} height={previewSize.rows * CELL + PAD * 2} rx="36" />
          </clipPath>
        </defs>
        <g clip-path="url(#editorBoardClip)">
          <image href={BOARD_BACKGROUND} x="0" y="0"
            width={previewSize.cols * CELL + PAD * 2} height={previewSize.rows * CELL + PAD * 2}
            preserveAspectRatio="xMidYMid slice" />
        </g>

        <!--
          Asphalt backstop: black road network drawn first so any hairline seam
          between adjacent SVG tiles is filled.
        -->
        <g fill="#0c0c0e" stroke="none">
          {#each Array(ROWS) as _, r}
            {#each Array(COLS) as _, c}
              {@const m = renderMaskAt(r, c)}
              <rect x={cellX(c) - ROAD_W / 2} y={cellY(r) - ROAD_W / 2}
                width={ROAD_W} height={ROAD_W} rx={ROAD_W / 2} />
              {#if (m & 1) !== 0}<rect x={cellX(c)} y={cellY(r) - ROAD_W / 2} width={CELL / 2 + 1} height={ROAD_W} />{/if}
              {#if (m & 2) !== 0}<rect x={cellX(c) - CELL / 2 - 1} y={cellY(r) - ROAD_W / 2} width={CELL / 2 + 1} height={ROAD_W} />{/if}
              {#if (m & 4) !== 0}<rect x={cellX(c) - ROAD_W / 2} y={cellY(r)} width={ROAD_W} height={CELL / 2 + 1} />{/if}
              {#if (m & 8) !== 0}<rect x={cellX(c) - ROAD_W / 2} y={cellY(r) - CELL / 2 - 1} width={ROAD_W} height={CELL / 2 + 1} />{/if}
            {/each}
          {/each}
        </g>

        <!--
          Per-cell road tile sprite: corners are corner pieces, edges are T-pieces,
          interior cells are full crosses. L/R-swap + horizontal mirror so the
          baked-in shadows fall on the same side as the in-game render.
        -->
        {#each Array(ROWS) as _, r}
          {#each Array(COLS) as _, c}
            {@const m = renderMaskAt(r, c)}
            {@const url = roadSpriteUrl(lookupOf(m))}
            {#if url}
              {@const cx0 = cellX(c) - CELL / 2 - BLEED / 2}
              {@const cy0 = cellY(r) - CELL / 2 - BLEED / 2}
              {@const w = CELL + BLEED}
              {@const h = CELL + BLEED}
              <g transform="translate({cx0 + w} {cy0}) scale(-1 1)">
                <image href={url} x="0" y="0" width={w} height={h} preserveAspectRatio="none" />
              </g>
            {/if}
          {/each}
        {/each}

        <!-- Car sprite once we know the level + variant -->
        {#if lockedStart}
          <image
            href={CAR_SPRITES.down}
            x={cellX(lockedStart[1]) - CAR_SIZE_PX / 2}
            y={cellY(lockedStart[0]) - CAR_SIZE_PX / 2}
            width={CAR_SIZE_PX} height={CAR_SIZE_PX}
            preserveAspectRatio="xMidYMid meet"
          />
        {/if}

        <!-- Pet/house sprites + click targets per cell -->
        {#each Array(previewSize.rows) as _, r}
          {#each Array(previewSize.cols) as _, c}
            {@const info = cellInfo(r, c)}
            {@const cx = cellX(c)}
            {@const cy = cellY(r)}
            {#if info.kind === 'pet' || info.kind === 'pending'}
              {@const pw = ANIMALS[info.idx].petWidth}
              {@const ph = ANIMALS[info.idx].petHeight}
              <image
                href={ANIMALS[info.idx].petSprite}
                x={cx - pw / 2}
                y={cy - ph / 2 + PET_Y_OFFSET}
                width={pw} height={ph}
                preserveAspectRatio="xMidYMid meet"
                opacity={info.kind === 'pending' ? 0.7 : 1}
              />
              {#if info.kind === 'pending'}
                <circle cx={cx} cy={cy + PET_Y_OFFSET} r={CELL / 2 - 24} fill="none"
                  stroke="#f47049" stroke-width="6" stroke-dasharray="14 14" opacity="0.85" />
              {/if}
            {:else if info.kind === 'house'}
              <image
                href={ANIMALS[info.idx].houseSprite}
                x={cx - HOUSE_SIZE_PX / 2}
                y={cy - HOUSE_SIZE_PX / 2 + HOUSE_Y_OFFSET}
                width={HOUSE_SIZE_PX} height={HOUSE_SIZE_PX}
                preserveAspectRatio="xMidYMid meet"
              />
            {/if}
            <!-- Hint dot when this cell could still hold a pet across current candidate levels -->
            {#if info.kind === 'empty' && pendingPetCellHints.has(r * 32 + c)}
              <circle cx={cx} cy={cy} r={14} fill="#f47049" opacity="0.55" />
            {/if}
            <!-- Transparent click target for the cell -->
            <rect
              class="cell-click"
              x={cx - CELL / 2} y={cy - CELL / 2} width={CELL} height={CELL}
              fill="transparent"
              style="cursor: {stoppedEarly ? 'default' : 'pointer'}"
              onclick={() => clickCell(r, c)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); clickCell(r, c); } }}
              role="button"
              tabindex="0"
              aria-label={`row ${r} column ${c}`}
            />
          {/each}
        {/each}
      </svg>
    </div>
  {/if}
</div>

<style>
  /* Tailwind doesn't generate these dynamic-looking utilities reliably; keep them inline. */
  :global(.bg-emerald-500-25) { background-color: rgb(16 185 129 / 0.25); }
  /* Suppress the browser's default focus ring on cell click targets — we render
     a circle around the pending pet ourselves and the cell-sized box ignored
     PET_Y_OFFSET, so the outline looked like a stray square in cells with a pet. */
  .cell-click:focus,
  .cell-click:focus-visible { outline: none; }
</style>
