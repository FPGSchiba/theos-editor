import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

// Unit tests run in Node; the page tests in e2e/ are Playwright's. The Svelte plugin compiles the
// components that data modules (the presets) import.
export default defineConfig({
  plugins: [svelte()],
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts', 'tests/**/*.test.ts'],
  },
});
