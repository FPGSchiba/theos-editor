// Opening a project (P11 §5.9): "Open project…" (Ctrl+O, and in the palette) shows the OS folder
// dialog and opens the folder chosen. A folder that is not a project shows P03's message above the
// windows and keeps the open project. The dialog's command, `plugin:dialog|open`, is answered by
// the test; `project_open` by the recordings.

import { expect, test, type Page } from '@playwright/test';

import { replay, type Replay } from './support/replay';

/** The folder dialog's command. */
const DIALOG = 'plugin:dialog|open';

/** The calls from the dialog's on. */
function fromDialog(ipc: Replay): Replay['calls'] {
  const at = ipc.calls.findIndex((c) => c.cmd === DIALOG);
  return at < 0 ? [] : ipc.calls.slice(at);
}

/** Starts the app, on the recorded last project when `last`; the dialog answers `chosen`. */
async function start(page: Page, last: boolean, chosen: string | null): Promise<Replay> {
  const ipc = await replay(page, {
    os: 'windows',
    use: [last ? 'project_last' : 'project_last-none', 'codex_tree-who'],
    answer: { [DIALOG]: chosen },
  });
  await page.goto('/');
  if (last) {
    await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  } else {
    await expect(page.locator('.tu-shell__work')).toContainText('Open a content project: Ctrl+O');
  }
  return ipc;
}

// AC: CE-32.1
test('ctrl_o_opens_the_folder_dialog_and_the_folder_chosen', async ({ page }) => {
  const ipc = await start(page, false, '<project>');
  await page.keyboard.press('Control+O');
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  await expect(page.getByRole('navigation', { name: 'Layout presets' })).toBeVisible();
  const calls = fromDialog(ipc);
  expect(calls[0]?.args).toMatchObject({ options: { directory: true } });
  expect(calls[1]).toEqual({ cmd: 'project_open', args: { path: '<project>' } });
});

// AC: CE-32.1
test('the_empty_work_area_has_an_open_project_button', async ({ page }) => {
  const ipc = await start(page, false, '<project>');
  await page.locator('.tu-shell__work').getByRole('button', { name: 'Open project…' }).click();
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  expect(fromDialog(ipc).map((c) => c.cmd).slice(0, 2)).toEqual([DIALOG, 'project_open']);
});

// AC: CE-32.2
test('open_project_runs_from_the_palette', async ({ page }) => {
  const ipc = await start(page, false, '<project>');
  await page.keyboard.press('Control+K');
  await page.keyboard.type('Open project');
  const row = page.getByRole('option', { name: /Open project…/ });
  await expect(row).toHaveAttribute('aria-disabled', 'false');
  await expect(row).toContainText('Ctrl+O');
  await page.keyboard.press('Enter');
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  expect(fromDialog(ipc).map((c) => c.cmd).slice(0, 2)).toEqual([DIALOG, 'project_open']);
});

test('a_cancelled_dialog_changes_nothing', async ({ page }) => {
  const ipc = await start(page, true, null);
  const opens = ipc.calls.filter((c) => c.cmd === 'project_open').length;
  await page.keyboard.press('Control+O');
  await expect.poll(() => fromDialog(ipc).length).toBe(1);
  await expect(page.locator('.tu-shell__work').getByRole('alert')).toHaveCount(0);
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  expect(ipc.calls.filter((c) => c.cmd === 'project_open')).toHaveLength(opens);
});

// AC: CE-32.1
test('a_folder_without_theos_toml_shows_p03s_message_and_keeps_the_project', async ({ page }) => {
  await start(page, true, '<project>/world');
  await page.keyboard.press('Control+O');
  const banner = page.locator('.tu-shell__work').getByRole('alert');
  await expect(banner).toContainText(
    '`<project>/world` is not a content project: it has no theos.toml',
  );
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  await expect(page.getByRole('navigation', { name: 'Layout presets' })).toBeVisible();
  await expect(page.locator('[data-window="codex-tree"]')).toBeVisible();
});

test('a_failing_app_info_shows_its_error_while_a_project_is_open', async ({ page }) => {
  const fail = { app_info: { kind: 'io', message: 'no answer' } };
  await replay(page, { os: 'windows', use: ['project_last', 'codex_tree-who'], fail });
  await page.goto('/');
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  await expect(page.locator('.tu-shell__work').getByRole('alert')).toContainText(
    'The app did not say its version and OS: no answer',
  );
});
