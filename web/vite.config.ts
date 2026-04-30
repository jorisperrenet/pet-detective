import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// For GitHub Pages: set base to '/<repo-name>/' via env, fallback to './' for relative.
const base = process.env.VITE_BASE ?? './';

export default defineConfig({
  base,
  plugins: [svelte()],
  build: {
    target: 'es2022',
  },
});
