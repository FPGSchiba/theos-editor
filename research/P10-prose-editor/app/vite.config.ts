import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [svelte()],
  // The repo root (resolved against this folder), so the dev server may serve design/theos-ui/.
  server: { fs: { allow: ['../../..'] } },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts', 'tests/docs/**/*.test.ts'],
  },
});
