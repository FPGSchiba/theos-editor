// The replay helper itself: recorded calls get their recorded answers, and a call without a
// recording fails the test.

import { copyFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { expect, test, type Page } from '@playwright/test';

import { loadRecordings, replay, type Os } from './support/replay';

/** The part of Tauri's page internals that `invoke` of `@tauri-apps/api/core` calls. */
interface TauriInternals {
  __TAURI_INTERNALS__: { invoke: (cmd: string, args: unknown) => Promise<unknown> };
}

/** Calls `cmd` through Tauri's IPC in the page; an error answer comes back as `{ rejected }`. */
function invoke(page: Page, cmd: string, args?: unknown): Promise<unknown> {
  return page.evaluate(
    async ([c, a]) => {
      try {
        return await (window as unknown as TauriInternals).__TAURI_INTERNALS__.invoke(c, a);
      } catch (rejected) {
        return { rejected };
      }
    },
    [cmd, args] as const,
  );
}

const recording = (name: string): unknown =>
  loadRecordings().find((r) => r.name === name)?.value;

/** The recording the app's start needs here: no last project, so it opens nothing. */
const START = ['project_last-none'];

/** Waits until the app has made its start calls (`app_info`, `project_last`). */
async function started(page: Page): Promise<void> {
  await expect(page.locator('.tu-shell__work')).toContainText('Open a content project');
  await expect(page.locator('.tu-statusbar')).toContainText('v<version>');
}

test('a call is answered by the recording with its cmd and args', async ({ page }) => {
  const r = await replay(page, { os: 'linux', use: START });
  await page.goto('/');
  await started(page);
  const before = r.calls.length;
  expect(await invoke(page, 'codex_tree', { tab: 'lore' })).toEqual(recording('codex_tree-lore'));
  expect(await invoke(page, 'project_last')).toBeNull();
  expect(r.calls.slice(before)).toEqual([
    { cmd: 'codex_tree', args: { tab: 'lore' } },
    { cmd: 'project_last', args: {} },
  ]);
});

test('an error recording rejects with its error value', async ({ page }, testInfo) => {
  const dir = testInfo.outputPath('ipc');
  const error = { kind: 'not_found', message: 'no record `nowhere`' };
  mkdirSync(dir, { recursive: true });
  const shipped = join(import.meta.dirname, 'ipc');
  for (const name of ['app_info-linux', 'project_argument-none', ...START]) {
    copyFileSync(join(shipped, `${name}.json`), join(dir, `${name}.json`));
  }
  writeFileSync(
    join(dir, 'record_view-nowhere.json'),
    JSON.stringify({ cmd: 'record_view', args: { id: 'nowhere' }, result: 'error', value: error }),
  );
  await replay(page, { os: 'linux', use: START, dir });
  await page.goto('/');
  expect(await invoke(page, 'record_view', { id: 'nowhere' })).toEqual({ rejected: error });
});

for (const os of ['windows', 'macos', 'linux'] satisfies Os[]) {
  test(`app_info answers with the os ${os}`, async ({ page }) => {
    await replay(page, { os, use: START });
    await page.goto('/');
    expect(await invoke(page, 'app_info')).toMatchObject({ os });
  });
}

test('use picks one of several recordings with the same call', async ({ page }) => {
  const r = await replay(page, { os: 'linux', use: [...START, 'codex_tree-who-empty'] });
  await page.goto('/');
  expect(await invoke(page, 'codex_tree', { tab: 'who' })).toEqual(
    recording('codex_tree-who-empty'),
  );
  r.use('codex_tree-who');
  expect(await invoke(page, 'codex_tree', { tab: 'who' })).toEqual(recording('codex_tree-who'));
  // Without `use`, the one recording that is not derived answers.
  expect(await invoke(page, 'project_open', { path: '<project>' })).toEqual(
    recording('project_open'),
  );
  // Nor does a recording of another fixture answer unless it is named.
  expect(await invoke(page, 'codex_tree', { tab: 'where' })).toEqual(recording('codex_tree-where'));
});

test('a command named in fail rejects with its error value', async ({ page }) => {
  const error = { kind: 'io', message: 'no codex today' };
  const r = await replay(page, { os: 'linux', use: START, fail: { codex_tree: error } });
  await page.goto('/');
  await started(page);
  expect(await invoke(page, 'codex_tree', { tab: 'lore' })).toEqual({ rejected: error });
  expect(r.calls.at(-1)).toEqual({ cmd: 'codex_tree', args: { tab: 'lore' } });
});

test('a command named in answer answers with its value', async ({ page }) => {
  const r = await replay(page, {
    os: 'linux',
    use: START,
    answer: { 'plugin:dialog|open': '<project>' },
  });
  await page.goto('/');
  await started(page);
  const args = { options: { directory: true } };
  expect(await invoke(page, 'plugin:dialog|open', args)).toBe('<project>');
  expect(r.calls.at(-1)).toEqual({ cmd: 'plugin:dialog|open', args });
});

test('use refuses a name that is not a recording', async ({ page }) => {
  await expect(replay(page, { os: 'linux', use: ['codex_tree-nowhere'] })).rejects.toThrow(
    'no recording named codex_tree-nowhere',
  );
});

test('a call without a recording fails the test', async ({ page }) => {
  test.fail(true, 'the replay must fail a test whose page makes an unrecorded call');
  await replay(page, { os: 'linux', use: START });
  await page.goto('/');
  await started(page);
  await invoke(page, 'codex_tree', { tab: 'nowhere' });
});

test('an ambiguous call fails the test', async ({ page }) => {
  test.fail(true, 'project_sync has two recordings and none is named in `use`');
  await replay(page, { os: 'linux', use: START });
  await page.goto('/');
  await started(page);
  await invoke(page, 'project_sync', { reason: 'focus' });
});
