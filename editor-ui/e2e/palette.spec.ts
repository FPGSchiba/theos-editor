// Shortcuts and the command palette (P11 §5.11): one keymap, `Mod` from `app_info().os`, and a
// palette that lists every command with its keys and filters as you type.

import { expect, test, type Page } from '@playwright/test';

import { replay, type Os, type Replay } from './support/replay';

/** Opens the app with the recorded last project on `os`; collects the page's errors. */
async function openProject(page: Page, os: Os, use: string[] = []): Promise<{ errors: Error[]; ipc: Replay }> {
  const errors: Error[] = [];
  page.on('pageerror', (error) => errors.push(error));
  const ipc = await replay(page, { os, use: ['project_last', ...use] });
  await page.goto('/');
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  return { errors, ipc };
}

const palette = (page: Page) => page.getByRole('dialog', { name: 'Command palette' });
const treeTab = (page: Page, name: string) =>
  page.locator('[data-window="codex-tree"]').getByRole('tab', { name, exact: true });

// AC: CE-32.2
test('palette_runs_commands_on_windows', async ({ page }) => {
  await openProject(page, 'windows');
  const codex = page.getByRole('navigation', { name: 'Layout presets' }).getByRole('button', { name: 'Codex', exact: true });
  await codex.focus();

  await page.keyboard.press('Control+K');
  await expect(palette(page)).toBeVisible();
  await expect(palette(page).getByRole('combobox', { name: 'Command' })).toBeFocused();
  await expect(palette(page).getByRole('option', { name: /Command palette/ })).toContainText('Ctrl+K');

  await page.keyboard.type('tree lore');
  await expect(palette(page).getByRole('option')).toHaveText([/^Tree tab: Lore/]);
  await expect(palette(page).getByRole('option').first()).toContainText('Ctrl+Shift+3');
  await page.keyboard.press('Enter');
  await expect(palette(page)).toBeHidden();
  await expect(treeTab(page, 'Lore')).toHaveAttribute('aria-selected', 'true');

  await page.keyboard.press('Control+K');
  await expect(palette(page)).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(palette(page)).toBeHidden();
  await expect(codex).toBeFocused();
});

// AC: CE-32.2
test('shortcuts_run_commands_on_windows', async ({ page }) => {
  await openProject(page, 'windows');
  await page.keyboard.press('Control+P');
  await expect(page.locator('.tu-toolbar').getByRole('combobox', { name: 'Find record' })).toBeFocused();

  await page.keyboard.press('Control+2');
  const rail = page.getByRole('navigation', { name: 'Layout presets' });
  await expect(rail.getByRole('button', { name: 'Graph', exact: true })).toHaveAttribute('aria-current', 'true');

  await page.keyboard.press('Control+Shift+4');
  await expect(rail.getByRole('button', { name: 'Codex', exact: true })).toHaveAttribute('aria-current', 'true');
  await expect(treeTab(page, 'Things')).toHaveAttribute('aria-selected', 'true');

  await page.keyboard.press('Control+Shift+P');
  await expect(palette(page)).toBeVisible();
});

// AC: CE-32.2
test('mod_is_cmd_on_macos', async ({ page }) => {
  await openProject(page, 'macos');
  await page.keyboard.press('Control+K');
  await expect(palette(page)).toBeHidden();
  await page.keyboard.press('Meta+K');
  await expect(palette(page)).toBeVisible();
  await expect(palette(page).getByRole('option', { name: /Command palette/ })).toContainText('⌘K');
  await expect(page.locator('.tu-toolbar').getByRole('button', { name: /Save/ })).toContainText('⌘S');
});

// AC: CE-32.2
test('save_is_listed_dimmed_and_does_nothing', async ({ page }) => {
  await openProject(page, 'windows');
  await page.keyboard.press('Control+K');
  await page.keyboard.type('save');
  const save = palette(page).getByRole('option', { name: /Save/ });
  await expect(save).toHaveAttribute('aria-disabled', 'true');
  await expect(save).toContainText('arrives with P12');
  await expect(save).toContainText('Ctrl+S');
  await page.keyboard.press('Enter');
  await expect(palette(page)).toBeVisible();
  await expect(save).toHaveAttribute('aria-selected', 'true');
});

// AC: CE-32.2
test('palette_lists_every_command_and_arrows_move', async ({ page }) => {
  await openProject(page, 'windows');
  await page.keyboard.press('Control+K');
  const options = palette(page).getByRole('option');
  await expect(options.first()).toHaveAttribute('aria-selected', 'true');
  await expect(options).toContainText(['Command palette', 'Find record', 'Close tab', 'Tree tab: Where', 'Save']);
  await page.keyboard.press('ArrowDown');
  await expect(options.nth(1)).toHaveAttribute('aria-selected', 'true');
  await page.keyboard.press('ArrowUp');
  await expect(options.first()).toHaveAttribute('aria-selected', 'true');
});

// AC: CE-32.2
test('mod_k_opens_the_palette_from_the_find_field', async ({ page }) => {
  await openProject(page, 'windows');
  await page.locator('.tu-toolbar').getByRole('combobox', { name: 'Find record' }).click();
  await page.keyboard.press('Control+K');
  await expect(palette(page)).toBeVisible();
});

// AC: CE-32.2
test('tab_commands_without_tabs_do_nothing', async ({ page }) => {
  const { errors } = await openProject(page, 'windows');
  await page.keyboard.press('Control+Tab');
  await page.keyboard.press('Control+Shift+Tab');
  await page.keyboard.press('Control+W');
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  await expect(treeTab(page, 'Where')).toHaveAttribute('aria-selected', 'true');
  expect(errors).toEqual([]);
});

// AC: CE-32.2
test('mod_r_reloads_the_project_not_the_page', async ({ page }) => {
  const { ipc } = await openProject(page, 'windows', ['project_sync-reload']);
  // A capturing listener reads each press's `defaultPrevented` once every listener has run:
  // Playwright's key presses are not trusted, so the browser would not reload, close or print
  // even unprevented, and only `defaultPrevented` shows the webview never sees them.
  await page.evaluate(() => {
    const seen: Record<string, boolean> = {};
    (window as unknown as { theosPrevented: Record<string, boolean> }).theosPrevented = seen;
    window.addEventListener(
      'keydown',
      (event) => {
        if (event.ctrlKey && event.key !== 'Control') {
          setTimeout(() => {
            seen[event.code] = event.defaultPrevented;
          });
        }
      },
      { capture: true },
    );
  });
  await page.keyboard.press('Control+R');
  await expect.poll(() => ipc.calls.some((c) => c.cmd === 'project_sync')).toBe(true);
  await page.keyboard.press('Control+W');
  await page.keyboard.press('Control+P');
  // Ctrl+J is no command's key: the listener sees it unprevented.
  await page.keyboard.press('Control+J');
  const prevented = () =>
    page.evaluate(() => (window as unknown as { theosPrevented: Record<string, boolean> }).theosPrevented);
  await expect.poll(prevented).toEqual({ KeyR: true, KeyW: true, KeyP: true, KeyJ: false });
});

// AC: CE-32.2
test('the_palette_is_modal', async ({ page }) => {
  await openProject(page, 'windows');
  const query = palette(page).getByRole('combobox', { name: 'Command' });
  await page.keyboard.press('Control+K');
  await expect(query).toBeFocused();

  await page.keyboard.press('Control+2');
  await page.keyboard.press('Control+P');
  const rail = page.getByRole('navigation', { name: 'Layout presets' });
  await expect(rail.getByRole('button', { name: 'Codex', exact: true })).toHaveAttribute('aria-current', 'true');
  await expect(query).toBeFocused();

  await page.keyboard.press('Tab');
  await expect(query).toBeFocused();
  await page.keyboard.press('Shift+Tab');
  await expect(query).toBeFocused();

  await page.keyboard.press('Control+K');
  await expect(palette(page)).toBeHidden();
});

// AC: CE-32.2
test('the_selected_row_is_the_active_descendant_and_in_view', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 320 });
  await openProject(page, 'windows');
  await page.keyboard.press('Control+K');
  const query = palette(page).getByRole('combobox', { name: 'Command' });
  const options = palette(page).getByRole('option');
  const last = options.last();
  await page.keyboard.press('ArrowUp');
  await expect(last).toHaveAttribute('aria-selected', 'true');
  const id = await last.getAttribute('id');
  expect(id).toBeTruthy();
  await expect(query).toHaveAttribute('aria-activedescendant', id ?? '');
  await expect(last).toBeInViewport();
});

// AC: CE-32.2
test('a_held_key_runs_its_command_once', async ({ page }) => {
  await openProject(page, 'windows');
  await page.keyboard.down('Control');
  // The second press is a repeat; were it run, it would close the palette again.
  await page.keyboard.down('k');
  await page.keyboard.down('k');
  await page.keyboard.up('k');
  await page.keyboard.up('Control');
  await expect(palette(page)).toBeVisible();
});
