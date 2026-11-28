// Freshness (P11 §5.9): the window getting the focus back syncs the open project. A record
// deleted outside the editor closes its tab and the status bar says so; an open record that
// changed loads its page again; a failed sync shows its error above the windows and keeps the
// project. The syncs are answered from the recordings `project_sync-removed` and
// `project_sync-changed`; the focus is the `tauri://focus` event of the mocked window.

import { expect, test, type Locator, type Page } from '@playwright/test';

import { replay, type Replay } from './support/replay';

/** The window's focus event. */
const FOCUS = 'tauri://focus';

/** Opens the app with the recorded last project, on the Codex. */
async function openProject(
  page: Page,
  options: { fail?: Record<string, unknown> } = {},
): Promise<Replay> {
  const ipc = await replay(page, {
    os: 'windows',
    use: ['project_last', 'codex_tree-who'],
    ...options,
  });
  await page.goto('/');
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  return ipc;
}

const tree = (page: Page): Locator => page.locator('[data-window="codex-tree"]');
const record = (page: Page): Locator => page.locator('[data-window="codex-record"]');

/** Opens the record whose row on the tree tab `tab` names `name`, and waits for its page. */
async function openRow(page: Page, tab: string, name: string, id: string): Promise<void> {
  await tree(page).getByRole('tab', { name: tab }).click();
  await tree(page)
    .locator('.tu-tree__row')
    .filter({ has: page.locator('.tu-tree__name', { hasText: name }) })
    .first()
    .click();
  await expect(record(page).locator('.tu-rechead__id')).toHaveText(id);
}

/** How often the page asked for the record `id`. */
const views = (ipc: Replay, id: string): number =>
  ipc.calls.filter((c) => c.cmd === 'record_view' && (c.args as { id?: string }).id === id).length;

test('a_record_deleted_outside_closes_its_tab_on_focus', async ({ page }) => {
  const ipc = await openProject(page);
  await openRow(page, 'Lore', 'C-032', 'C-032');
  await expect(record(page).getByRole('tab')).toHaveText(['C-032']);

  ipc.use('project_sync-removed');
  await ipc.emit(FOCUS);

  await expect(page.locator('.tu-statusbar')).toContainText(
    'C-032 was deleted outside the editor',
  );
  await expect(page.locator('.tu-statusbar')).toContainText('21 records');
  await expect(record(page).locator('.tu-rechead')).toHaveCount(0);
  await expect(record(page)).toContainText('The record you open.');
  expect(ipc.calls.filter((c) => c.cmd === 'project_sync')).toEqual([
    { cmd: 'project_sync', args: { reason: 'focus' } },
  ]);

  // The next sync closes no tab, so the notice goes.
  await ipc.emit(FOCUS);
  await expect.poll(() => ipc.calls.filter((c) => c.cmd === 'project_sync').length).toBe(2);
  await expect(page.locator('.tu-statusbar')).not.toContainText('deleted outside the editor');
});

test('a_record_changed_outside_reloads_its_view_on_focus', async ({ page }) => {
  const ipc = await openProject(page);
  await openRow(page, 'Where', 'The Choir-Hall', 'f04-choir-hall');
  const before = views(ipc, 'f04-choir-hall');

  ipc.use('project_sync-changed');
  await ipc.emit(FOCUS);

  await expect.poll(() => views(ipc, 'f04-choir-hall')).toBe(before + 1);
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('f04-choir-hall');
  await expect(record(page).locator('.tu-rechead__title')).toHaveText('The Choir-Hall');
  await expect(page.locator('.tu-statusbar')).not.toContainText('deleted outside the editor');
});

test('focus_without_a_project_syncs_nothing', async ({ page }) => {
  const ipc = await replay(page, { os: 'windows', use: ['project_last-none'] });
  await page.goto('/');
  await expect(page.locator('.tu-shell__work')).toContainText('Open a content project');
  await ipc.emit(FOCUS);
  await ipc.emit(FOCUS);
  await expect(page.locator('.tu-shell__work').getByRole('alert')).toHaveCount(0);
  expect(ipc.calls.filter((c) => c.cmd === 'project_sync')).toEqual([]);
});

test('a_failed_sync_shows_its_error_and_keeps_the_project', async ({ page }) => {
  const fail = { project_sync: { kind: 'index', message: 'the cache is damaged' } };
  const ipc = await openProject(page, { fail });
  await ipc.emit(FOCUS);
  const banner = page.locator('.tu-shell__work').getByRole('alert');
  await expect(banner).toContainText('The project could not be synced: the cache is damaged');
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  await expect(page.getByRole('navigation', { name: 'Layout presets' })).toBeVisible();
  await expect(tree(page)).toBeVisible();
  await banner.getByRole('button', { name: 'Dismiss' }).click();
  await expect(banner).toHaveCount(0);
});
