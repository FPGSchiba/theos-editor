import { defineConfig } from 'vitest/config';

// The report checks (`pnpm test:reports`) read research/P10-prose-editor/report/, which only a
// full measurement run writes (Task 17), so they are not part of `pnpm test:unit`.
export default defineConfig({
  test: {
    environment: 'node',
    include: ['tests/reports/**/*.test.ts'],
  },
});
