import { fileURLToPath } from 'node:url';

import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig, searchForWorkspaceRoot } from 'vite';

// Theos UI is imported from its one place in the repo (P11 §5.7), so the dev server may serve it.
const THEOS_UI = fileURLToPath(new URL('../design/theos-ui', import.meta.url));

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 5179,
    strictPort: true,
    fs: { allow: [searchForWorkspaceRoot(process.cwd()), THEOS_UI] },
  },
  build: { outDir: 'dist', emptyOutDir: true },
});
