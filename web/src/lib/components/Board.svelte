<script lang="ts">
  import type { ParsedPuzzle } from '../types';
  import {
    ANIMALS,
    BOARD_BACKGROUND,
    CAR_SIZE_PX,
    CAR_SPRITES,
    GAME_CELL_PX,
    HOUSE_SIZE_PX,
    HOUSE_Y_OFFSET,
    PET_Y_OFFSET,
    roadSpriteUrl,
  } from '../animals';

  interface Sprites {
    car?: string;       // ignored — we always use directional sprites from CAR_SPRITES
    animals: string[];  // index 0..N-1 → URL
    houses: string[];
    /** Per-slot pet bounding box. Falls back to the slot's default ANIMALS entry. */
    animalSizes?: { width: number; height: number }[];
  }

  interface Props {
    puzzle: ParsedPuzzle;
    sprites?: Sprites;
    /** Car position in cell coordinates (row, col); floats allowed for interpolation. */
    carPos?: { row: number; col: number };
    /** Bitmask of pair indices currently in the trunk. */
    trunk?: number;
    /** Highlight a target cell. */
    highlight?: { row: number; col: number } | null;
    /** Show the start "X" marker (only useful before solving). */
    showStart?: boolean;
    /** Fade picked-up animals / dim delivered houses. */
    pickedAnimals?: number;
    deliveredHouses?: number;
    /**
     * Direction the car is currently facing, expressed as one of 'up'/'down'/'left'/'right'
     * — we sprite-swap rather than rotate so the artwork is always upright.
     * If `carHeading` is given (legacy callers), we map angle→direction.
     */
    carDirection?: 'up' | 'down' | 'left' | 'right';
    carHeading?: number;
  }

  let {
    puzzle,
    sprites,
    carPos,
    trunk = 0,
    highlight = null,
    showStart = false,
    pickedAnimals = 0,
    deliveredHouses = 0,
    carDirection,
    carHeading = Math.PI, // default to facing the player (down), like the in-game start state
  }: Props = $props();

  // Cell pixel size matches the bundled GridLayoutGroup (250 px).
  const CELL = GAME_CELL_PX;
  const PAD = 20;
  /**
   * Width of the asphalt strip drawn under the road tiles. The road sprites have
   * the manholes / rounded ends; this backstop just guarantees a continuous
   * black strip across cell boundaries, so any 1-px seam between adjacent SVG
   * tiles is filled. ~80 px is what the in-game road graphics show.
   */
  const ROAD_W = 86;
  /** Slight bleed for the road tile sprites, on top of the backstop. */
  const ROAD_BLEED = 4;

  $effect(() => { void trunk; });

  let rowIndices = $derived(Array.from({ length: puzzle.rows }, (_, i) => i));
  let colIndices = $derived(Array.from({ length: puzzle.cols }, (_, i) => i));
  let car = $derived(carPos ?? { row: puzzle.start[0], col: puzzle.start[1] });

  let viewW = $derived(puzzle.cols * CELL + PAD * 2);
  let viewH = $derived(puzzle.rows * CELL + PAD * 2);

  function cellX(c: number) { return PAD + c * CELL + CELL / 2; }
  function cellY(r: number) { return PAD + r * CELL + CELL / 2; }

  function hueFill(h: number, l = 55, s = 70) {
    return `hsl(${Math.round(h)}, ${s}%, ${l}%)`;
  }

  /**
   * UDLR connectivity bitmask for cell (r, c). Considers a cell to have a road
   * in a direction iff the matching road indicator is present in the puzzle.
   * U=8 D=4 L=2 R=1.
   */
  function roadMask(r: number, c: number): number {
    let m = 0;
    if (r > 0 && puzzle.vRoads[r - 1]?.[c] === '|') m |= 8;
    if (r < puzzle.rows - 1 && puzzle.vRoads[r]?.[c] === '|') m |= 4;
    if (c > 0 && puzzle.hRoads[r]?.[c - 1] === '-') m |= 2;
    if (c < puzzle.cols - 1 && puzzle.hRoads[r]?.[c] === '-') m |= 1;
    return m;
  }

  /** Choose a directional car sprite from the heading angle (radians, 0=up). */
  function dirFromHeading(rad: number): 'up' | 'down' | 'left' | 'right' {
    // Normalise to (-π, π]
    let a = ((rad + Math.PI) % (Math.PI * 2) + Math.PI * 2) % (Math.PI * 2) - Math.PI;
    const ax = Math.abs(a);
    if (ax < Math.PI / 4) return 'up';
    if (ax > 3 * Math.PI / 4) return 'down';
    return a > 0 ? 'right' : 'left';
  }

  let carDir = $derived(carDirection ?? dirFromHeading(carHeading));
  let carUrl = $derived(CAR_SPRITES[carDir]);
</script>

<svg
  class="block w-full h-auto select-none"
  viewBox="0 0 {viewW} {viewH}"
  xmlns="http://www.w3.org/2000/svg"
  style="max-height: min(64vh, 560px)"
>
  <defs>
    <filter id="targetGlow">
      <feGaussianBlur stdDeviation="6" />
      <feComponentTransfer><feFuncA type="linear" slope="2"/></feComponentTransfer>
    </filter>
    <filter id="grayscale">
      <feColorMatrix type="matrix" values="0.33 0.33 0.33 0 0  0.33 0.33 0.33 0 0  0.33 0.33 0.33 0 0  0 0 0 0.7 0" />
    </filter>
    <clipPath id="boardClip">
      <rect x="0" y="0" width={viewW} height={viewH} rx="36" />
    </clipPath>
  </defs>

  <!-- Bundled background texture, clipped to a rounded card. -->
  <g clip-path="url(#boardClip)">
    <image
      href={BOARD_BACKGROUND}
      x="0" y="0" width={viewW} height={viewH}
      preserveAspectRatio="xMidYMid slice"
    />
  </g>

  <!--
    Asphalt backstop. We draw black strips across every open road segment first,
    so adjacent cells share a continuous black surface. The road tile sprites that
    follow add the manholes / rounded ends, but any 1-px seam between SVG tiles
    is now filled by this layer.
  -->
  <g fill="#0c0c0e" stroke="none">
    {#each rowIndices as r}
      {#each colIndices as c}
        {@const mask = roadMask(r, c)}
        {#if mask !== 0}
          <!-- Cell-centre square so junctions are fully covered. -->
          <rect
            x={cellX(c) - ROAD_W / 2} y={cellY(r) - ROAD_W / 2}
            width={ROAD_W} height={ROAD_W}
            rx={ROAD_W / 2}
          />
          {#if (mask & 1) !== 0}
            <!-- right -->
            <rect x={cellX(c)} y={cellY(r) - ROAD_W / 2} width={CELL / 2 + 1} height={ROAD_W} />
          {/if}
          {#if (mask & 2) !== 0}
            <!-- left -->
            <rect x={cellX(c) - CELL / 2 - 1} y={cellY(r) - ROAD_W / 2} width={CELL / 2 + 1} height={ROAD_W} />
          {/if}
          {#if (mask & 4) !== 0}
            <!-- down -->
            <rect x={cellX(c) - ROAD_W / 2} y={cellY(r)} width={ROAD_W} height={CELL / 2 + 1} />
          {/if}
          {#if (mask & 8) !== 0}
            <!-- up -->
            <rect x={cellX(c) - ROAD_W / 2} y={cellY(r) - CELL / 2 - 1} width={ROAD_W} height={CELL / 2 + 1} />
          {/if}
        {/if}
      {/each}
    {/each}
  </g>

  <!-- Road tile sprites layered on top of the backstop for the manholes / rounded caps. -->
  {#each rowIndices as r}
    {#each colIndices as c}
      {@const mask = roadMask(r, c)}
      {#if mask !== 0}
        <!--
          Levels are stored 90°-rotated relative to the on-screen layout. To keep
          our sprite picks consistent with the original in-game shading, we look
          up the sprite for the *L/R-swapped* mask and then horizontally mirror
          its <image>. The two mirrors cancel for road geometry (same cell still
          connects in the same directions) but flip the baked-in shadow back to
          the screen side the original game shows.
        -->
        {@const lookupMask = (mask & 0b1100) | ((mask & 1) << 1) | ((mask & 2) >> 1)}
        {@const url = roadSpriteUrl(lookupMask)}
        {#if url}
          {@const cx0 = cellX(c) - CELL / 2 - ROAD_BLEED / 2}
          {@const cy0 = cellY(r) - CELL / 2 - ROAD_BLEED / 2}
          {@const w = CELL + ROAD_BLEED}
          {@const h = CELL + ROAD_BLEED}
          <g transform="translate({cx0 + w} {cy0}) scale(-1 1)">
            <image href={url} x="0" y="0" width={w} height={h} preserveAspectRatio="none" />
          </g>
        {/if}
      {/if}
    {/each}
  {/each}

  <!-- Halo + dashed ring around next-target cell. -->
  {#if highlight}
    {@const hx = cellX(highlight.col)}
    {@const hCh = puzzle.grid[highlight.row]?.[highlight.col] ?? ''}
    {@const hyOffset = hCh >= 'a' && hCh <= 'z' ? PET_Y_OFFSET
                     : hCh >= 'A' && hCh <= 'Z' ? HOUSE_Y_OFFSET
                     : 0}
    {@const hy = cellY(highlight.row) + hyOffset}
    <!--
      Position via a translate <g> so the inner rotation animateTransform's
      from/to are constants. Otherwise hx/hy changing per step would restart
      the rotation and the dashes appear to glitch.
    -->
    <g transform="translate({hx} {hy})">
      <circle cx="0" cy="0" r={CELL / 2 - 16} fill="#f47049" opacity="0.18" />
      <circle cx="0" cy="0" r={CELL / 2 - 24} fill="none"
        stroke="#f47049" stroke-width="4" stroke-dasharray="10 12" opacity="0.85">
        <animateTransform attributeName="transform" type="rotate"
          from="0 0 0" to="360 0 0" dur="8s" repeatCount="indefinite" />
      </circle>
    </g>
  {/if}

  <!-- Cell contents -->
  {#each rowIndices as r}
    {#each colIndices as c}
      {@const ch = puzzle.grid[r][c]}
      {@const cx = cellX(c)}
      {@const cy = cellY(r)}
      {#if ch === 'X'}
        {#if showStart}
          <text x={cx} y={cy + 10} text-anchor="middle" fill="#f47049" font-size="32" font-weight="700">START</text>
        {/if}
      {:else if ch >= 'a' && ch <= 'z'}
        {@const idx = ch.charCodeAt(0) - 97}
        {@const hue = puzzle.hues[idx]}
        {@const picked = (pickedAnimals & (1 << idx)) !== 0}
        {@const pw = sprites?.animalSizes?.[idx]?.width ?? ANIMALS[idx]?.petWidth ?? 100}
        {@const ph = sprites?.animalSizes?.[idx]?.height ?? ANIMALS[idx]?.petHeight ?? 100}
        {#if !picked}
          {#if sprites?.animals[idx]}
            <image
              href={sprites.animals[idx]}
              x={cx - pw / 2}
              y={cy - ph / 2 + PET_Y_OFFSET}
              width={pw} height={ph}
              preserveAspectRatio="xMidYMid meet"
            />
          {:else}
            <rect x={cx - 60} y={cy - 60} width="120" height="120" rx="26" fill={hueFill(hue, 48, 70)} />
            <text x={cx} y={cy + 18} text-anchor="middle" fill="#fff" font-size="48" font-weight="700">{ch.toUpperCase()}</text>
          {/if}
        {/if}
      {:else if ch >= 'A' && ch <= 'Z'}
        {@const idx = ch.charCodeAt(0) - 65}
        {@const hue = puzzle.hues[idx]}
        {@const delivered = (deliveredHouses & (1 << idx)) !== 0}
        {#if sprites?.houses[idx]}
          <image
            href={sprites.houses[idx]}
            x={cx - HOUSE_SIZE_PX / 2}
            y={cy - HOUSE_SIZE_PX / 2 + HOUSE_Y_OFFSET}
            width={HOUSE_SIZE_PX} height={HOUSE_SIZE_PX}
            preserveAspectRatio="xMidYMid meet"
            filter={delivered ? 'url(#grayscale)' : ''}
            opacity={delivered ? 0.5 : 1}
          />
          {#if delivered}
            <path d="M {cx - 28},{cy + HOUSE_Y_OFFSET + 24} l 18,18 l 36,-36" stroke="#7fee9c" stroke-width="11"
              fill="none" stroke-linecap="round" stroke-linejoin="round" />
          {/if}
        {:else}
          <g opacity={delivered ? 0.4 : 1}>
            <path d="M {cx - 80},{cy - 30} L {cx},{cy - 90} L {cx + 80},{cy - 30} Z" fill="#7a7a7e" />
            <rect x={cx - 70} y={cy - 40} width="140" height="120" rx="14" fill="#5a5a5e" />
            <rect x={cx - 40} y={cy - 18} width="80" height="70" rx="22" fill={hueFill(hue, 45, 60)} />
            <text x={cx} y={cy + 32} text-anchor="middle" fill="#fff" font-size="36" font-weight="700">{ch}</text>
          </g>
        {/if}
      {/if}
    {/each}
  {/each}

  <!-- Car: directional sprite, never rotated. -->
  <image
    href={carUrl}
    x={cellX(car.col) - CAR_SIZE_PX / 2}
    y={cellY(car.row) - CAR_SIZE_PX / 2}
    width={CAR_SIZE_PX} height={CAR_SIZE_PX}
    preserveAspectRatio="xMidYMid meet"
  />
</svg>
