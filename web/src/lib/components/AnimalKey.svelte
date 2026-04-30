<script lang="ts">
  import { ANIMALS } from '../animals';

  interface Props {
    /** Optional bitmask of which animals to highlight (others dimmed). */
    highlight?: number;
    /** Layout direction. */
    layout?: 'grid' | 'compact';
  }
  let { highlight, layout = 'grid' }: Props = $props();

  function isOn(idx: number): boolean {
    return highlight === undefined || (highlight & (1 << idx)) !== 0;
  }
</script>

{#if layout === 'compact'}
  <div class="flex flex-wrap gap-2">
    {#each ANIMALS as a}
      <div class="flex items-center gap-1.5 px-2 py-1 rounded-md bg-white/5 border border-white/10"
        class:opacity-30={!isOn(a.index)}>
        <img src={a.petSprite} alt={a.name} class="w-6 h-6 object-contain" />
        <span class="text-[11px] text-gray-300">
          <span class="font-mono font-semibold text-car">{a.letter}</span>
          <span class="text-gray-400"> · {a.name}</span>
        </span>
      </div>
    {/each}
  </div>
{:else}
  <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2">
    {#each ANIMALS as a}
      <div class="flex items-center gap-2 p-2 rounded-md bg-white/5 border border-white/10"
        class:opacity-30={!isOn(a.index)}>
        <img src={a.petSprite} alt={a.name} class="w-10 h-10 object-contain" />
        <div class="flex flex-col leading-tight">
          <span class="text-xs">
            <span class="font-mono font-semibold text-car">{a.letter}</span>
            <span class="text-gray-500"> / </span>
            <span class="font-mono text-gray-300">{a.homeLetter}</span>
          </span>
          <span class="text-xs text-gray-300">{a.name}</span>
        </div>
        <img src={a.houseSprite} alt={a.name + ' house'} class="ml-auto w-7 h-7 object-contain opacity-80" />
      </div>
    {/each}
  </div>
{/if}
