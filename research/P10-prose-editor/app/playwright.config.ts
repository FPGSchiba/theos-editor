import { defineConfig } from '@playwright/test';

const port = 5179;
const origin = `http://localhost:${port}`;

export default defineConfig({
  // Only the browser suites; the Vitest `*.test.ts` files are never loaded here.
  testDir: 'tests/e2e',
  testMatch: '**/*.spec.ts',
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  reporter: [['list']],
  use: { baseURL: origin },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
    { name: 'webkit', use: { browserName: 'webkit' } },
  ],
  // Vite is started through node, not pnpm: corepack's shim is not on every PATH.
  webServer: {
    command: `node node_modules/vite/bin/vite.js --port ${port} --strictPort`,
    url: origin,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
    // Show the dev server's output, so a stalled module request is visible in a run.
    stdout: 'pipe',
    stderr: 'pipe',
  },
  // The reports in ../report are only overwritten by a run that asks for it (P10_REPORT=1).
  globalTeardown: process.env.P10_REPORT === '1' ? './tests/e2e/write-reports.ts' : undefined,
});
