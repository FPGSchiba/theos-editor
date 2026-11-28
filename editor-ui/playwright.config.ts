import { defineConfig, devices } from '@playwright/test';

// Page tests run against the Vite preview of a fresh build (P11 §5.12), in Chromium only. Vite is
// started through node, not pnpm: Corepack's shim is not on every PATH. The port is not the dev
// server's (5179), so both can run at once.
const PORT = 4179;
const ORIGIN = `http://localhost:${PORT}`;
const VITE = 'node node_modules/vite/bin/vite.js';

export default defineConfig({
  testDir: 'e2e',
  testMatch: '**/*.spec.ts',
  fullyParallel: true,
  forbidOnly: Boolean(process.env['CI']),
  reporter: [['list']],
  use: { baseURL: ORIGIN },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: `${VITE} build && ${VITE} preview --port ${PORT} --strictPort`,
    url: ORIGIN,
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
