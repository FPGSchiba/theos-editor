// The toolbar's find field (P11 §5.10), answered by the recorded `codex_search` results.

import { expect, test, type Locator, type Page } from '@playwright/test';

import { replay } from './support/replay';

/** Opens the app on the Codex and returns the find field. */
async function openFind(page: Page): Promise<Locator> {
  const recorded = await replay(page, { os: 'windows', use: ['project_last', 'codex_tree-where'] });
  await page.goto('/');
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
  calls = recorded.calls;
  return page.getByRole('combobox', { name: 'Find record' });
}

/** The calls the page made, of the last `openFind`. */
let calls: readonly { cmd: string }[] = [];

const results = (page: Page): Locator => page.getByRole('listbox', { name: 'Search results' });

// AC: CE-33.3
test('typing_a_kind_shows_its_hits', async ({ page }) => {
  const field = await openFind(page);
  await field.fill(':clue');
  await expect(results(page).getByRole('option')).toHaveCount(2);
  await expect(results(page).getByRole('option').nth(0)).toContainText('C-031');
  await expect(results(page).getByRole('option').nth(1)).toContainText('C-032');
});

// AC: CE-33.3
test('arrows_and_enter_open_the_second_hit_and_select_its_row', async ({ page }) => {
  const field = await openFind(page);
  await field.fill(':clue');
  await expect(results(page).getByRole('option')).toHaveCount(2);
  await field.press('ArrowDown');
  await field.press('ArrowDown');
  await expect(results(page).getByRole('option').nth(1)).toHaveAttribute('aria-selected', 'true');
  await field.press('Enter');
  await expect(page.locator('[data-window="codex-record"] .tu-rechead__id')).toHaveText('C-032');
  await expect(results(page)).toHaveCount(0);
  await expect(page.locator('[data-window="codex-tree"] .tu-tab.is-active')).toHaveText(/Lore/);
  const selected = page.locator('[data-window="codex-tree"] .tu-tree__row.is-selected');
  await expect(selected).toContainText('C-032');
});

// AC: CE-33.3
test('a_match_is_marked_in_the_first_matching_line', async ({ page }) => {
  const field = await openFind(page);
  await field.fill(':clue murmur');
  const first = results(page).getByRole('option').first();
  await expect(first.locator('mark')).toHaveText('murmurs');
  await expect(first).toContainText('title: A spirit murmurs a word in a dead tongue');
});

// AC: CE-33.3
test('escape_closes_the_results', async ({ page }) => {
  const field = await openFind(page);
  await field.fill(':clue');
  await expect(results(page)).toBeVisible();
  await field.press('Escape');
  await expect(results(page)).toHaveCount(0);
});

// AC: CE-33.3
test('an_unknown_kind_says_so', async ({ page }) => {
  const field = await openFind(page);
  await field.fill(':nokind');
  await expect(page.locator('.tu-toolbar .results')).toContainText('no kind `nokind`');
  await expect(results(page).getByRole('option')).toHaveCount(0);
});

// AC: CE-33.3
test('enter_before_the_answer_opens_nothing', async ({ page }) => {
  const field = await openFind(page);
  await field.fill(':clue');
  await field.press('Enter');
  await expect(results(page).getByRole('option')).toHaveCount(2);
  expect(calls.filter((call) => call.cmd === 'record_view')).toEqual([]);
  await expect(page.locator('[data-window="codex-record"] .tu-rechead__id')).toHaveCount(0);
});

// AC: CE-33.3
test('a_hit_opened_from_another_layout_shows_the_codex', async ({ page }) => {
  const field = await openFind(page);
  const rail = page.getByRole('navigation', { name: 'Layout presets' });
  await rail.getByRole('button', { name: 'Review', exact: true }).click();
  await expect(page.locator('.tu-statusbar')).toContainText('Review layout');
  await field.fill(':clue');
  await expect(results(page).getByRole('option')).toHaveCount(2);
  await results(page).getByRole('option').nth(0).click();
  await expect(rail.getByRole('button', { name: 'Codex', exact: true })).toHaveAttribute('aria-current', 'true');
  await expect(page.locator('[data-window="codex-record"] .tu-rechead__id')).toHaveText('C-031');
});

// AC: CE-33.3
test('an_opened_hit_s_tree_row_is_scrolled_into_view', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 200 });
  const field = await openFind(page);
  await field.fill(':clue');
  await expect(results(page).getByRole('option')).toHaveCount(2);
  await results(page).getByRole('option').nth(1).click();
  await expect(page.locator('[data-window="codex-record"] .tu-rechead__id')).toHaveText('C-032');
  const selected = page.locator('[data-window="codex-tree"] .tu-tree__row.is-selected');
  await expect(selected).toContainText('C-032');
  await expect(selected).toBeInViewport();
});
