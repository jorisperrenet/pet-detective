<script lang="ts">
  import type { ParsedPuzzle, SolveResult } from '../types';
  import Board from './Board.svelte';

  interface Sprites {
    car: string;
    animals: string[];
    houses: string[];
    full?: string;
  }

  interface Props {
    puzzle: ParsedPuzzle;
    solution: SolveResult;
    sprites?: Sprites;
    /** Compact layout (no big stat tiles) for the home-page example. */
    compact?: boolean;
    /** Lumosity's shipped solution string (history) — rendered at the bottom of the right column. */
    lumosityHist?: string;
    /** Our BFS-optimal solution string — rendered alongside the Lumosity one. */
    ourHist?: string;
  }
  let { puzzle, solution, sprites, compact = false, lumosityHist, ourHist }: Props = $props();

  /** Quintic smootherstep — zero 1st & 2nd derivatives at endpoints; gives the
   *  car a natural ease-in/cruise/ease-out per click destination. */
  function sigmoid(t: number): number {
    if (t <= 0) return 0;
    if (t >= 1) return 1;
    return t * t * t * (t * (t * 6 - 15) + 10);
  }

  function stepFuel(k: number): number {
    return Math.max(0, solution.steps[k].path.length - 1);
  }

  let totalSteps = $derived(solution.steps.length);
  let cumFuel = $derived.by(() => {
    const c = new Array<number>(totalSteps + 1).fill(0);
    for (let i = 0; i < totalSteps; i++) c[i + 1] = c[i] + stepFuel(i);
    return c;
  });

  // Continuous progress in step units. progress = k + t means "at fraction t inside step k".
  let progress = $state(0);
  let playing = $state(false);
  let speedMul = $state(1);
  /** Base step duration is now 3× the original — so the slider's 1× default
   *  corresponds to what used to be 1/3×, and the user can still scale up. */
  const SPEED_BASE_MULT = 3;
  function stepDurationMs(k: number): number {
    return SPEED_BASE_MULT * Math.max(280, 220 * Math.max(1, stepFuel(k)));
  }

  // Heading convention: 0 = up, π = down, ±π/2 = right/left.
  // Default is "down" so the car faces the player at the very start, like in-game.
  const HEADING_DOWN = Math.PI;

  function carAtStep(k: number, t: number, prevHeading: number): { row: number; col: number; heading: number } {
    const path = solution.steps[k].path;
    if (path.length === 0) return { row: puzzle.start[0], col: puzzle.start[1], heading: prevHeading };
    if (path.length === 1) return { row: path[0][0], col: path[0][1], heading: prevHeading };
    const eased = sigmoid(t);
    const n = path.length - 1;
    const u = Math.min(n - 1e-9, eased * n);
    const i = Math.min(n - 1, Math.floor(u));
    const f = u - i;
    const row = path[i][0] + (path[i + 1][0] - path[i][0]) * f;
    const col = path[i][1] + (path[i + 1][1] - path[i][1]) * f;
    const dr = path[i + 1][0] - path[i][0];
    const dc = path[i + 1][1] - path[i][1];
    const heading = (dr === 0 && dc === 0) ? prevHeading : Math.atan2(dc, -dr);
    return { row, col, heading };
  }

  let car = $derived.by<{ row: number; col: number; heading: number }>(() => {
    if (totalSteps === 0) return { row: puzzle.start[0], col: puzzle.start[1], heading: HEADING_DOWN };
    // Before the player has pressed play, sit at the start cell facing down — the in-game default.
    if (progress === 0) return { row: puzzle.start[0], col: puzzle.start[1], heading: HEADING_DOWN };
    if (progress >= totalSteps) {
      // Keep the last move's direction so the car settles facing that way.
      const last = solution.steps[totalSteps - 1];
      const lastPath = last.path;
      let h = HEADING_DOWN;
      if (lastPath.length >= 2) {
        const a = lastPath[lastPath.length - 2];
        const b = lastPath[lastPath.length - 1];
        h = Math.atan2(b[1] - a[1], -(b[0] - a[0]));
      }
      return { row: last.pos[0], col: last.pos[1], heading: h };
    }
    const k = Math.floor(progress);
    const t = progress - k;
    // Carry the heading from the previous step's tail when the car is momentarily still.
    let prev = HEADING_DOWN;
    if (k > 0) {
      const ppath = solution.steps[k - 1].path;
      if (ppath.length >= 2) {
        const a = ppath[ppath.length - 2];
        const b = ppath[ppath.length - 1];
        prev = Math.atan2(b[1] - a[1], -(b[0] - a[0]));
      }
    }
    return carAtStep(k, t, prev);
  });

  let stateAtProgress = $derived.by(() => {
    let trunk = 0, picked = 0, delivered = 0;
    const completed = Math.floor(progress);
    for (let s = 0; s < completed; s++) {
      const step = solution.steps[s];
      if (step.type === 'animal') { trunk |= 1 << step.idx; picked |= 1 << step.idx; }
      else { trunk &= ~(1 << step.idx); delivered |= 1 << step.idx; }
    }
    return { trunk, picked, delivered };
  });

  let currentStepIdx = $derived(progress >= totalSteps ? -1 : Math.floor(progress));
  let highlight = $derived.by(() => {
    if (currentStepIdx < 0) return null;
    const step = solution.steps[currentStepIdx];
    return { row: step.pos[0], col: step.pos[1] };
  });

  let fuelUsed = $derived.by(() => {
    if (totalSteps === 0) return 0;
    if (progress >= totalSteps) return solution.fuel;
    const k = Math.floor(progress);
    const t = progress - k;
    return Math.round(cumFuel[k] + sigmoid(t) * stepFuel(k));
  });

  // Currently-in-trunk indices, ordered (smallest bit first).
  let trunkSlots = $derived.by(() => {
    const out: number[] = [];
    let m = stateAtProgress.trunk;
    let i = 0;
    while (m) { if (m & 1) out.push(i); m >>>= 1; i++; }
    return out;
  });

  // RAF loop — advance progress in step-units according to per-step durations.
  let raf = 0;
  let lastTime = 0;
  function loop(now: number) {
    if (!playing) { raf = 0; return; }
    if (lastTime === 0) lastTime = now;
    const dt = (now - lastTime);
    lastTime = now;
    if (progress < totalSteps) {
      const k = Math.floor(progress);
      const dur = stepDurationMs(k) / speedMul;
      progress = Math.min(totalSteps, progress + dt / dur);
    }
    if (progress >= totalSteps) {
      progress = totalSteps;
      playing = false;
    }
    if (playing) {
      raf = requestAnimationFrame(loop);
    } else {
      raf = 0; // make sure play() doesn't see a stale id
    }
  }
  function play() {
    // Always cancel any stale frame first so Replay reliably restarts after the
    // previous run ended (the old raf id sticks around even when playing flips
    // to false, so without this guard the next frame is never requested).
    if (raf) { cancelAnimationFrame(raf); raf = 0; }
    if (progress >= totalSteps) progress = 0;
    playing = true;
    lastTime = 0;
    raf = requestAnimationFrame(loop);
  }
  function pause() { playing = false; }
  function reset() {
    if (raf) { cancelAnimationFrame(raf); raf = 0; }
    progress = 0; playing = false; lastTime = 0;
  }
  function stepForward() { pause(); progress = Math.min(totalSteps, Math.floor(progress + 0.0001) + 1); }
  function stepBack()    { pause(); progress = Math.max(0, Math.ceil(progress - 0.0001) - 1); }
  function jumpTo(k: number) { pause(); progress = Math.max(0, Math.min(totalSteps, k + 1)); }

  function onKey(e: KeyboardEvent) {
    if ((e.target as HTMLElement)?.tagName === 'INPUT' || compact) return;
    if (e.key === ' ') { e.preventDefault(); playing ? pause() : play(); }
    else if (e.key === 'ArrowRight') stepForward();
    else if (e.key === 'ArrowLeft') stepBack();
    else if (e.key === 'r' || e.key === 'R') reset();
  }

  let route = $derived(
    solution.steps.map((s, i) => ({
      idx: s.idx,
      type: s.type,
      ch: s.type === 'animal' ? String.fromCharCode(97 + s.idx) : String.fromCharCode(65 + s.idx),
      hue: puzzle.hues[s.idx],
      stepIndex: i,
      sprite: s.type === 'animal' ? sprites?.animals[s.idx] : sprites?.houses[s.idx],
    })),
  );

  let nextStep = $derived(currentStepIdx >= 0 ? solution.steps[currentStepIdx] : null);
  let nextSprite = $derived(
    nextStep
      ? (nextStep.type === 'animal' ? sprites?.animals[nextStep.idx] : sprites?.houses[nextStep.idx])
      : null,
  );

  let progressPct = $derived(((fuelUsed) / Math.max(1, solution.fuel)) * 100);
</script>

<svelte:window onkeydown={onKey} />

<!-- On large screens: board + drive panel on the left, route timeline on the right
     (only when there's enough horizontal room). Mobile: natural single-column stack. -->
<div class="grid gap-3 {compact ? '' : 'sm:grid-cols-[minmax(0,1fr)_minmax(260px,340px)] md:grid-cols-[minmax(0,1fr)_minmax(300px,380px)] lg:grid-cols-[minmax(0,1fr)_minmax(340px,440px)] xl:grid-cols-[minmax(0,1fr)_minmax(380px,480px)] sm:items-start'}">
  <div class="flex flex-col gap-3 min-w-0">
  <Board
    {puzzle}
    {sprites}
    carPos={{ row: car.row, col: car.col }}
    carHeading={car.heading}
    trunk={stateAtProgress.trunk}
    pickedAnimals={stateAtProgress.picked}
    deliveredHouses={stateAtProgress.delivered}
    {highlight}
  />

  {#if !compact}
    <!-- Trunk strip — directly under the board so it visually belongs to it. -->
    <div class="flex items-center justify-center gap-2 mt-2">
      <span class="text-[11px] uppercase tracking-[0.18em] text-gray-500 mr-1">Trunk</span>
      {#each [0, 1, 2, 3] as i}
        {@const animalIdx = trunkSlots[i] ?? -1}
        {@const sprite = animalIdx >= 0 ? sprites?.animals[animalIdx] : null}
        <div class="w-12 h-12 rounded-lg bg-bg-deep ring-1 ring-white/5 flex items-center justify-center overflow-hidden"
          class:ring-2={animalIdx >= 0}>
          {#if sprite}
            <img src={sprite} alt="" class="w-10 h-10 object-contain" />
          {:else}
            <span class="text-gray-700 text-xs">·</span>
          {/if}
        </div>
      {/each}
    </div>

  {:else}
    <!-- Compact controls for the home-page example -->
    <div class="flex items-center justify-between gap-3 text-sm">
      <button class="btn-primary text-xs px-2.5 py-1.5" onclick={() => (playing ? pause() : play())}>
        {playing ? 'Pause' : (progress >= totalSteps ? 'Replay' : 'Play')}
      </button>
      <div class="flex-1 h-1.5 bg-bg-deep rounded-full overflow-hidden ring-1 ring-white/5">
        <div class="h-full bg-gradient-to-r from-car to-orange-300 rounded-full transition-[width] duration-100"
          style:width="{Math.max(0, Math.min(100, progressPct))}%"></div>
      </div>
      <div class="text-gray-300 text-xs tabular-nums shrink-0">
        <span class="text-car">{fuelUsed}</span> / {solution.fuel}
      </div>
    </div>
  {/if}
  </div>

  {#if !compact}
    <!-- Right column on lg+: drive panel, route timeline, and stats line. Mobile stacks below the board. -->
    <div class="flex flex-col gap-3 sm:sticky sm:top-3 self-start">

    <!-- Drive panel — moved here so the play controls sit alongside the route. -->
    <section class="panel p-3 sm:p-4 flex flex-col gap-3 ring-1 ring-white/[0.04]">
      <div class="flex items-center justify-between gap-3 min-h-[36px]">
        <div class="flex items-center gap-2.5 min-w-0">
          {#if nextSprite}
            <div class="w-9 h-9 rounded-lg bg-bg-deep/80 ring-1 ring-white/5 flex items-center justify-center shrink-0 overflow-hidden">
              <img src={nextSprite} alt="" class="w-8 h-8 object-contain" />
            </div>
          {:else}
            <div class="w-9 h-9 rounded-lg bg-green-500/10 ring-1 ring-green-400/30 flex items-center justify-center shrink-0">
              <svg class="w-4 h-4 text-green-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            </div>
          {/if}
          <div class="min-w-0">
            <div class="text-[10px] uppercase tracking-[0.18em] text-gray-500 mb-0.5">
              {progress >= totalSteps ? 'Complete' : `Stop ${Math.min(totalSteps, Math.floor(progress) + 1)} / ${totalSteps}`}
            </div>
            <div class="text-xs sm:text-sm text-gray-100 leading-tight truncate">
              {#if progress >= totalSteps}
                Delivered {puzzle.animals.length} pets in {solution.fuel} fuel.
              {:else if nextStep?.type === 'animal'}
                Drive {stepFuel(currentStepIdx)} cell{stepFuel(currentStepIdx) === 1 ? '' : 's'} — pick up <span style:color={`hsl(${puzzle.hues[nextStep.idx]}, 70%, 65%)`}>pet&nbsp;{String.fromCharCode(97 + nextStep.idx)}</span>.
              {:else if nextStep}
                Drive {stepFuel(currentStepIdx)} cell{stepFuel(currentStepIdx) === 1 ? '' : 's'} — drop off at <span style:color={`hsl(${puzzle.hues[nextStep.idx]}, 70%, 65%)`}>house&nbsp;{String.fromCharCode(65 + nextStep.idx)}</span>.
              {/if}
            </div>
          </div>
        </div>
        <div class="text-right tabular-nums shrink-0">
          <div class="text-[10px] uppercase tracking-[0.18em] text-gray-500">Fuel</div>
          <div class="text-lg font-semibold leading-none">
            <span class="text-car">{fuelUsed}</span>
            <span class="text-gray-600 text-sm"> / {solution.fuel}</span>
          </div>
        </div>
      </div>

      <div class="relative h-2 bg-bg-deep rounded-full overflow-hidden ring-1 ring-white/[0.04]">
        <div class="absolute inset-y-0 left-0 bg-gradient-to-r from-car to-orange-300 rounded-full transition-[width] duration-100"
          style:width="{Math.max(0, Math.min(100, progressPct))}%"></div>
      </div>

      <div class="flex items-center flex-wrap gap-2 pt-0.5">
        <button class="btn-primary text-xs px-3 py-1.5" onclick={() => (playing ? pause() : play())} aria-label={playing ? 'Pause' : 'Play'}>
          {#if playing}
            <svg class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="5" width="4" height="14" rx="1"/><rect x="14" y="5" width="4" height="14" rx="1"/></svg>
            Pause
          {:else}
            <svg class="w-3 h-3" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
            {progress >= totalSteps ? 'Replay' : 'Play'}
          {/if}
        </button>
        <button class="btn-ghost h-7 w-7 p-0" onclick={stepBack} aria-label="Step back" title="Previous (←)">
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h2v12H6zM21 6v12L11 12z"/></svg>
        </button>
        <button class="btn-ghost h-7 w-7 p-0" onclick={stepForward} aria-label="Step forward" title="Next (→)">
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor"><path d="M16 6h2v12h-2zM3 18V6l10 6z"/></svg>
        </button>
        <button class="btn-ghost h-7 w-7 p-0" onclick={reset} aria-label="Reset" title="Reset (R)">
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/>
          </svg>
        </button>
        <div class="flex items-center gap-1.5 text-[11px] text-gray-400 w-full sm:w-auto ml-auto">
          <span>Speed</span>
          <input type="range" min="0.25" max="3" step="0.25" bind:value={speedMul} class="accent-car w-20" />
          <span class="tabular-nums w-8 text-right text-gray-200">{speedMul.toFixed(2)}×</span>
        </div>
      </div>
    </section>

    <!-- Route timeline. -->
    <section class="panel p-3">
      <div class="text-[10px] uppercase tracking-[0.18em] text-gray-500 mb-2">Route</div>
      <div class="grid grid-cols-[repeat(auto-fill,minmax(2rem,1fr))] gap-1 justify-items-center">
        {#each route as r}
          {@const isCurrent = r.stepIndex === currentStepIdx}
          {@const isPast = r.stepIndex < currentStepIdx || currentStepIdx === -1}
          {@const isHouse = r.type === 'house'}
          <button
            type="button"
            class="relative flex items-center justify-center transition-all"
            class:opacity-60={!isCurrent && !isPast && currentStepIdx !== -1}
            class:scale-110={isCurrent}
            onclick={() => jumpTo(r.stepIndex)}
            title={`Stop ${r.stepIndex + 1}: ${r.type === 'animal' ? 'pick up' : 'drop off'} ${r.ch}`}
          >
            <div class="w-8 h-8 rounded-lg flex items-center justify-center overflow-hidden"
              class:ring-2={isCurrent}
              class:ring-offset-2={isCurrent}
              class:ring-offset-bg-panel={isCurrent}
              style:background-color={`hsla(${r.hue}, 70%, 50%, ${isHouse ? 0.10 : 0.18})`}
              style:--ring-color={`hsla(${r.hue}, 75%, 60%, 0.9)`}
              style:box-shadow={isCurrent ? `0 0 0 2px hsla(${r.hue}, 80%, 60%, 0.85)` : `inset 0 0 0 1px hsla(${r.hue}, 70%, 60%, 0.35)`}
            >
              {#if r.sprite}
                <img src={r.sprite} alt="" class="w-7 h-7 object-contain" class:grayscale={isPast} class:opacity-70={isPast} />
              {:else}
                <span class="font-mono text-xs font-semibold" style:color={`hsl(${r.hue}, 70%, 65%)`}>{r.ch}</span>
              {/if}
            </div>
            {#if isHouse}
              <!-- Tiny corner flag distinguishes "house" from "pet" at a glance. -->
              <span class="absolute -top-1 -right-1 w-3 h-3 rounded-full bg-bg-panel ring-1 ring-white/10 flex items-center justify-center">
                <svg viewBox="0 0 24 24" class="w-2 h-2 text-gray-300" fill="currentColor">
                  <path d="M12 3 4 9v12h6v-7h4v7h6V9z"/>
                </svg>
              </span>
            {/if}
          </button>
        {/each}
      </div>
    </section>

    <!-- Inline stats line — small, secondary. -->
    <div class="flex items-center justify-center text-xs text-gray-500 gap-3 flex-wrap">
      <span><span class="text-car font-medium">{solution.fuel}</span> fuel optimal</span>
      <span class="text-gray-700">·</span>
      <span><span class="text-gray-300 font-medium">{puzzle.animals.length}</span> pets</span>
      <span class="text-gray-700">·</span>
      <span><span class="text-gray-300 font-medium">{totalSteps}</span> stops</span>
    </div>

    {#if lumosityHist || ourHist}
      <div class="text-[11px] text-gray-500 flex flex-col items-center gap-1 pt-1 break-all text-center">
        {#if lumosityHist}
          <span>
            Lumosity solution:
            <span class="font-mono text-gray-300">{lumosityHist}</span>
          </span>
        {/if}
        {#if ourHist}
          <span>
            Our optimal solution:
            <span class="font-mono text-gray-300">{ourHist}</span>
          </span>
        {/if}
      </div>
    {/if}

    </div>
  {/if}
</div>
