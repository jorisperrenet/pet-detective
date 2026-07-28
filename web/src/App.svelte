<script lang="ts">
  import { onMount } from 'svelte';
  import LevelBrowser from './lib/components/LevelBrowser.svelte';
  import LevelEditor from './lib/components/LevelEditor.svelte';
  import SiteHeader from './lib/site-kit/SiteHeader.svelte';
  import SiteFooter from './lib/site-kit/SiteFooter.svelte';

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

<SiteHeader
  projectName="Pet Detective"
  currentProjectId="pet-detective"
  projectHref="/pet-detective/"
  logoSrc="{import.meta.env.BASE_URL}personal-logo.svg"
/>
<main class="flex min-h-screen flex-col bg-[#f9fbff] text-gray-900 dark:bg-[#111827] dark:text-gray-100">
  <header class="px-4 sm:px-6 pt-1.5 pb-1 flex items-center justify-between max-w-5xl mx-auto w-full gap-3">
    <div class="flex items-center gap-3 min-w-0">
      <img src="{import.meta.env.BASE_URL}sprites/board/card.svg" alt="" class="w-10 h-10 sm:w-11 sm:h-11 shrink-0" />
      <span class="truncate text-base font-semibold text-gray-900 dark:text-gray-100 sm:text-xl">Pet Detective Solver</span>
    </div>
    <div class="flex shrink-0 overflow-hidden rounded-lg border border-gray-300 text-xs dark:border-gray-700">
      <button
        class="px-3 py-1.5 transition-colors"
        class:bg-car={mode === 'find'}
        class:text-gray-950={mode === 'find'}
        class:text-gray-600={mode !== 'find'}
        class:dark:text-gray-300={mode !== 'find'}
        class:hover:text-gray-900={mode !== 'find'}
        class:dark:hover:text-white={mode !== 'find'}
        onclick={() => setMode('find')}
      >Find a level</button>
      <button
        class="border-l border-gray-300 px-3 py-1.5 transition-colors dark:border-gray-700"
        class:bg-car={mode === 'library'}
        class:text-gray-950={mode === 'library'}
        class:text-gray-600={mode !== 'library'}
        class:dark:text-gray-300={mode !== 'library'}
        class:hover:text-gray-900={mode !== 'library'}
        class:dark:hover:text-white={mode !== 'library'}
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

  <aside class="mx-auto w-full max-w-3xl px-4 pb-6 sm:px-6" aria-labelledby="solver-note-heading">
    <div class="rounded-2xl border border-gray-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-800">
      <h2 id="solver-note-heading" class="text-base font-bold text-gray-950 dark:text-white">Behind the solver</h2>
      <p class="mt-2 text-sm leading-6 text-gray-600 dark:text-gray-300">
        Lumosity notes that sufficiently large routing problems can become computationally intractable.
        This Rust solver checked all <span class="font-mono text-gray-800 dark:text-gray-200">2,558</span> shipped levels in
        <span class="font-mono text-gray-800 dark:text-gray-200">2.3&nbsp;seconds</span> on six cores.
        The game’s supplied route is already optimal for <span class="font-mono text-gray-800 dark:text-gray-200">2,504</span>
        of them—an impressive result. Of the remaining 54, 32 use one extra fuel and 22 use two.
      </p>
    </div>
  </aside>

</main>
<SiteFooter
  projectName="Pet Detective"
  sourceHref="https://github.com/jorisperrenet/pet-detective"
  notice="Pet Detective, Lumosity, the Lumosity logo and all in-game artwork shown here are © Lumos Labs, Inc. All rights reserved. This is an independent, unaffiliated solver and visualiser—not the game."
/>
