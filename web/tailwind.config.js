/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{svelte,ts,js}'],
  theme: {
    extend: {
      colors: {
        // Mirrors the palette of the original game
        bg: {
          DEFAULT: '#1f1f22',
          deep: '#161618',
          panel: '#26262a',
        },
        road: {
          DEFAULT: '#1a1a1c',
          edge: '#0d0d0f',
          dot: '#9a9a9a',
        },
        house: '#7a7a7e',
        car: '#f47049',
      },
      fontFamily: {
        sans: ['"Inter"', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [],
};
