<script lang="ts">
  import { onMount } from 'svelte';
  import LevelBrowser from './lib/components/LevelBrowser.svelte';
  import LevelEditor from './lib/components/LevelEditor.svelte';

  type Mode = 'find' | 'library';
  let mode = $state<Mode>('find');

  function modeFromUrl(): Mode {
    if (typeof window === 'undefined') return 'find';
    const p = new URLSearchParams(window.location.search);
    // A `?level=` URL always means "show that level in the library".
    if (p.has('level')) return 'library';
    if (p.get('mode') === 'library') return 'library';
    return 'find';
  }

  function setMode(m: Mode) {
    if (mode === m) return;
    mode = m;
    if (typeof window === 'undefined') return;
    const p = new URLSearchParams(window.location.search);
    if (m === 'library') {
      p.set('mode', 'library');
    } else {
      // Find mode also gets an explicit URL marker so the toggle is unambiguous.
      p.set('mode', 'find');
      // Drop library-only params when switching to find mode.
      p.delete('group');
      p.delete('page');
      p.delete('level');
      p.delete('flip');
      p.delete('map');
    }
    const qs = p.toString();
    window.history.pushState(null, '', qs ? `?${qs}` : window.location.pathname);
  }

  onMount(() => {
    mode = modeFromUrl();
    const onpop = () => { mode = modeFromUrl(); };
    window.addEventListener('popstate', onpop);
    return () => window.removeEventListener('popstate', onpop);
  });
</script>

<main class="min-h-screen flex flex-col">
  <header class="px-4 sm:px-6 pt-1.5 pb-1 flex items-center justify-between max-w-5xl mx-auto w-full gap-3">
    <div class="flex items-center gap-3 min-w-0">
      <img src="/sprites/board/card.svg" alt="" class="w-10 h-10 sm:w-11 sm:h-11 shrink-0" />
      <span class="text-base sm:text-xl font-semibold text-gray-100 truncate">Pet Detective Solver</span>
    </div>
    <div class="flex rounded-lg border border-white/10 overflow-hidden text-xs shrink-0">
      <button
        class="px-3 py-1.5 transition-colors"
        class:bg-car={mode === 'find'}
        class:text-white={mode === 'find'}
        class:text-gray-400={mode !== 'find'}
        class:hover:text-gray-200={mode !== 'find'}
        onclick={() => setMode('find')}
      >Find a level</button>
      <button
        class="px-3 py-1.5 transition-colors border-l border-white/10"
        class:bg-car={mode === 'library'}
        class:text-white={mode === 'library'}
        class:text-gray-400={mode !== 'library'}
        class:hover:text-gray-200={mode !== 'library'}
        onclick={() => setMode('library')}
      >Solution library</button>
    </div>
  </header>

  <section class="flex-1 px-4 sm:px-6 pb-6 max-w-5xl mx-auto w-full">
    {#if mode === 'library'}
      <LevelBrowser />
    {:else}
      <LevelEditor />
    {/if}
  </section>

  <footer class="px-6 py-6 text-center text-xs text-gray-500 max-w-2xl mx-auto leading-relaxed space-y-2">
    <p>
      Pet Detective, Lumosity, the Lumosity logo and every piece of in-game artwork shown here are
      © <a href="https://www.lumoslabs.com" target="_blank" rel="noopener" class="underline decoration-dotted hover:text-gray-300">Lumos&nbsp;Labs,&nbsp;Inc.</a>
      All rights reserved.
    </p>
    <p>
      This page is an unaffiliated solver and visualiser — <strong>not the game</strong>. To actually play Pet Detective
      (and everything else around it) head to <a href="https://www.lumosity.com/en/blog/pet-detective-behind-the-game" target="_blank" rel="noopener" class="underline decoration-dotted hover:text-gray-300">Lumosity</a>.
    </p>
    <p>
      Lumosity's own write-up muses that for big enough routing problems
      <em>"computers couldn't even find a solution in the lifetime of the universe."</em>
      The Rust solver here chewed through all <span class="font-mono text-gray-400">2558</span> shipped levels in
      <span class="font-mono text-gray-300">2.3&nbsp;s</span> on six cores. The in-game route is optimal for
      <span class="font-mono text-gray-300">2504</span>; the remaining <span class="font-mono text-gray-300">54</span> ship a sub-optimal route —
      <span class="font-mono">32</span> wasted one fuel and <span class="font-mono">22</span> wasted two.
    </p>
  </footer>
</main>
