// The Codex screen (P11 §5.8): the tree, the record window and the side column of screen 01,
// drawn from the recorded answers of `codex_tree` and `record_view` on the synthetic fixture.

import { expect, test, type Locator, type Page } from '@playwright/test';

import { loadRecordings, replay } from './support/replay';

/** A record as a recording names it. */
interface Ref {
  id: string;
  kind: string | null;
  title: string | null;
}

/** The recorded page of the record `id`. */
function recordedPage(id: string): {
  front_matter: { key: string }[];
  links_in: { slot: string; source: Ref }[];
  used_by: { source: Ref; relations: string[] }[];
  findings: { rule: string; message: string; severity: string }[];
} {
  const recording = loadRecordings().find((r) => r.name === `record_view-${id}`);
  if (recording === undefined) {
    throw new Error(`no recording record_view-${id}`);
  }
  return recording.value as ReturnType<typeof recordedPage>;
}

/** The kinds `codex_tabs` shows id first. */
const ID_FIRST: string[] = (() => {
  const tabs = loadRecordings().find((r) => r.name === 'codex_tabs')?.value as
    | { id_first: string[] }[]
    | undefined;
  return (tabs ?? []).flatMap((tab) => tab.id_first);
})();

/** How a row names `source`: id and title for the kinds shown id first, else the title. */
function named(source: Ref): string {
  const title = source.title ?? source.id;
  return source.kind !== null && ID_FIRST.includes(source.kind) ? `${source.id} ${title}` : title;
}

/** Opens the app with the recorded last project, on the Codex. */
async function openProject(page: Page, use: string[] = ['codex_tree-who']): Promise<void> {
  await replay(page, { os: 'windows', use: ['project_last', ...use] });
  await page.goto('/');
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
}

/** The Codex tree window. */
const tree = (page: Page): Locator => page.locator('[data-window="codex-tree"]');

/** The record window. */
const record = (page: Page): Locator => page.locator('[data-window="codex-record"]');

/** The tree row whose name is `name`. */
const row = (page: Page, name: string): Locator =>
  tree(page).locator('.tu-tree__row').filter({ has: page.locator('.tu-tree__name', { hasText: name }) });

/** Opens the floor from the Where tab. */
async function openFloor(page: Page): Promise<void> {
  await openProject(page);
  await row(page, 'The Choir-Hall').click();
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('f04-choir-hall');
}

// AC: CE-33.1
test('floor_shows_as_on_screen_01', async ({ page }) => {
  await openFloor(page);
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'grid-noir');
  await expect(page.locator('.tu-titlebar')).toContainText('world/hearth/f04-choir-hall');
  await expect(page.locator('.tu-titlebar')).not.toContainText('f04-choir-hall.md');

  const head = record(page).locator('.tu-rechead');
  await expect(head.locator('.tu-rechead__title')).toHaveText('The Choir-Hall');
  await expect(head.locator('.tu-rechead__kind')).toHaveText('floor · in Hearth');
  await expect(head.locator('.tu-status')).toHaveAttribute('data-status', 'draft');
  await expect(head.locator('.tu-rechead__meta')).toContainText('world/hearth/f04-choir-hall.md');
  await expect(head.locator('.tu-rechead__meta')).toContainText('8 links in');
  await expect(head.locator('.tu-rechead__meta')).toContainText('2 children');
  await expect(record(page).locator('.tu-manuscript')).toHaveCount(1);

  const body = record(page).locator('.tu-prose');
  await expect(body.locator('h2')).toHaveText('Synthetic heading');
  const first = body.locator('p').first();
  await expect(first.locator('.tu-link .tu-link__title')).toHaveText(["The Fates' shrine", 'marsh']);
  const lore = body.locator('li .tu-link');
  await expect(lore.locator('.tu-link__id')).toHaveText('C-031');
  await expect(lore.locator('.tu-link__title')).toHaveText('A spirit murmurs a word in a dead tongue');

  const voice = record(page).locator('.tu-voice');
  await expect(voice.locator('.tu-voice__head')).toContainText('Mother Ilse');
  await expect(voice.locator('.tu-voice__head-right')).toContainText('B-016 · T1');
  await expect(voice.locator('.tu-voice__body')).toContainText('The Fates spin for those who climb.');
  await expect(voice.locator('.tu-voice__by')).toHaveText('a synthetic caption on the shrine stair');

  const table = record(page).locator('.lore-table');
  await expect(table.locator('.lore-row')).toHaveCount(2);
  await expect(table.locator('.lore-row').nth(0)).toContainText('distorts');
  await expect(table.locator('.lore-row').nth(0)).toContainText('· never');
  await expect(table.locator('.lore-row').nth(1)).toContainText('hints');
  await expect(table.locator('.lore-row').nth(1)).toContainText('T1');

  const pictures = record(page).locator('.pictures');
  await expect(pictures.locator('figcaption')).toHaveText('◫ f04-choir-hall-art · no pick');
  await expect(pictures).toContainText('Images arrive with P16');

  const keys = recordedPage('f04-choir-hall').front_matter.map((field) => field.key);
  await expect(page.locator('.tu-fm .tu-fm__key')).toHaveText(keys);
  await expect(page.locator('[data-window="codex-front-matter"]')).toContainText('schema · floor v1');
  await expect(page.getByRole('button', { name: '+ field' })).toBeDisabled();
  await expect(page.getByRole('button', { name: '+ field' })).toHaveAttribute(
    'title',
    'Editing arrives with P12',
  );

  // Every finding has a row, errors first, each with its severity, message and rule.
  const order = ['error', 'warning'];
  const findings = [...recordedPage('f04-choir-hall').findings].sort(
    (a, b) => order.indexOf(a.severity) - order.indexOf(b.severity),
  );
  const validation = page.locator('[data-window="codex-validation"] .tu-val');
  await expect(validation).toHaveCount(findings.length);
  await expect(validation.locator('.tu-val__msg')).toHaveText(findings.map((f) => f.message));
  await expect(validation.locator('.tu-val__rule')).toHaveText(findings.map((f) => f.rule));
  await expect(validation.locator('.tu-val__sev')).toHaveText(findings.map((f) => f.severity));
  expect(findings.map((f) => f.rule)).toContain('link/dangling');

  const broken = body.locator('.tu-link.is-reject');
  await expect(broken).toHaveCount(1);
  await expect(broken).toContainText('✕');
  await expect(broken.getByText('broken link:')).toHaveCount(1);
  await expect(broken).toContainText('synthetic-dangling');

  await expect(row(page, 'Hearth').locator('.tu-tree__tag')).toHaveText('1–20');
  await expect(row(page, 'The Choir-Hall').locator('.tu-tree__tag')).toHaveText('draft');
  await expect(row(page, 'The Choir-Hall')).toHaveClass(/is-selected/);
  await expect(tree(page).locator('.tree-footer span')).toHaveText([
    'Where: one parent per place',
    'Who: people → faction → NPC',
    'folders follow this tree; the id is the identity',
  ]);
});

// AC: CE-33.2
test('backlinks_show_and_open', async ({ page }) => {
  await openFloor(page);
  const recorded = recordedPage('f04-choir-hall');
  const usedBy = page.locator('[data-window="codex-used-by"] .tu-prop');
  await expect(usedBy.locator('.tu-prop__val')).toHaveText(recorded.used_by.map((row) => named(row.source)));
  await expect(usedBy.locator('.tu-prop__type')).toHaveText(
    recorded.used_by.map((row) => row.relations.join(' · ')),
  );
  const clue = usedBy.filter({ hasText: 'A spirit murmurs' });
  await expect(clue.locator('.tu-prop__type')).toHaveText('where');
  // The script links the floor in `where` and in two beats: each relation shows once.
  await expect(usedBy.filter({ hasText: 'The shrine hums' }).locator('.tu-prop__type')).toHaveText(
    'where · in a beat',
  );

  const side = page.locator('[data-window="codex-front-matter"]');
  await side.getByRole('tab', { name: `Links in · ${recorded.links_in.length}` }).click();
  const links = side.locator('.links-in .tu-prop');
  await expect(links.locator('.tu-prop__val')).toHaveText(
    recorded.links_in.map((link) => `${link.source.id} ${link.source.title ?? ''}`),
  );
  await expect(links.locator('.tu-prop__type')).toHaveText(recorded.links_in.map((link) => link.slot));

  await links.filter({ hasText: 'S-004' }).first().click();
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('S-004');
  await record(page).getByRole('tab', { name: 'f04-choir-hall' }).click();
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('f04-choir-hall');

  await clue.click();
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('C-031');
  await expect(record(page).getByRole('tab')).toHaveText(['f04-choir-hall', 'S-004', 'C-031']);
  await expect(row(page, 'C-031')).toHaveClass(/is-selected/);
  await expect(tree(page).getByRole('tab', { name: 'Lore' })).toHaveAttribute('aria-selected', 'true');
});

test('html_in_records_is_text', async ({ page }) => {
  await openProject(page);
  await tree(page).getByRole('tab', { name: 'Things' }).click();
  await row(page, '<img').click();
  const window = record(page);
  await expect(window.locator('.tu-rechead__title')).toHaveText(
    '<img src=x onerror=alert(1)> <script>alert(2)</script>',
  );
  await expect(window.locator('.tu-prose')).toContainText('<img src=x onerror=alert(3)>');
  await expect(window.locator('.tu-prose')).toContainText('<script>alert(4)</script>');
  await expect(window.locator('.tu-prose')).toContainText('<script>alert(5)</script>');
  await expect(window.locator('.tu-prose')).toContainText('<img src=x onerror=alert(6)>');
  await expect(page.locator('.tu-shell img, .tu-shell script')).toHaveCount(0);
});

test('empty_tab_explains', async ({ page }) => {
  await openProject(page, ['codex_tree-who-empty']);
  await tree(page).getByRole('tab', { name: 'Who' }).click();
  await expect(tree(page)).toContainText(
    'Records whose kind is people, faction or npc appear here. Add one with theos new, or as a file under people/.',
  );
});

test('never_truth_is_marked_everywhere', async ({ page }) => {
  await openFloor(page);
  const loreRow = record(page).locator('.lore-row').filter({ hasText: 'T-013' });
  await expect(loreRow.locator('[data-never]')).toContainText('never');

  await tree(page).getByRole('tab', { name: 'Lore' }).click();
  const truth = row(page, 'T-013');
  await expect(truth).toHaveAttribute('data-never', 'true');
  await expect(truth).toContainText('never');

  await row(page, 'B-017').click();
  const chip = page.locator('.tu-fm .tu-link').filter({ hasText: 'T-013' });
  await expect(chip).toHaveAttribute('data-never', 'true');
  await expect(chip).toContainText('never');
});

test('external_links_are_text', async ({ page }) => {
  await openProject(page);
  await tree(page).getByRole('tab', { name: 'Things' }).click();
  await row(page, '<img').click();
  const window = record(page);
  const external = window.locator('.tu-prose .external');
  await expect(external).toHaveText([
    'synthetic site (https://example.invalid/x)',
    'synthetic image (https://example.invalid/y.png)',
  ]);
  await expect(window.locator('a, a[href], img')).toHaveCount(0);
  await expect(page.locator('.tu-shell a[href]')).toHaveCount(0);
});

test('records_outside_the_tree_come_after_its_root', async ({ page }) => {
  await openProject(page);
  const top = tree(page).locator('.tu-tree__row[aria-level="1"]');
  await expect(top.locator('.tu-tree__name')).toHaveText([
    'Synthetic tower',
    'Synthetic loop A',
    'Synthetic loop B',
    'Synthetic stray',
  ]);
  await expect(top.first()).toHaveClass(/is-root/);
  for (const index of [1, 2, 3]) {
    await expect(top.nth(index)).not.toHaveClass(/is-root/);
    await expect(top.nth(index).locator('.tu-tree__tag')).toHaveText('no root');
  }
});

// AC: CE-33.1
test('two_files_with_one_id_both_show', async ({ page }) => {
  const errors: Error[] = [];
  page.on('pageerror', (error) => errors.push(error));
  await openProject(page, [
    'codex_tree-who',
    'codex_tree-where-duplicate-id',
    'record_view-tower-duplicate-id',
  ]);
  const twins = tree(page).locator('.tu-tree__row').filter({
    has: page.locator('.tu-tree__name', { hasText: /^Synthetic twin/ }),
  });
  await expect(twins.locator('.tu-tree__name')).toHaveText(['Synthetic twin one', 'Synthetic twin two']);

  // Each file opens its own page and selects only its own row; both stay open as two tabs.
  const heading = record(page).locator('.tu-rechead__title');
  await twins.nth(0).click();
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('synthetic-twin');
  await expect(heading).toHaveText('Synthetic twin one');
  await expect(twins.nth(0)).toHaveAttribute('aria-selected', 'true');
  await expect(twins.nth(1)).toHaveAttribute('aria-selected', 'false');
  await twins.nth(1).click();
  await expect(heading).toHaveText('Synthetic twin two');
  await expect(twins.nth(1)).toHaveAttribute('aria-selected', 'true');
  await expect(twins.nth(0)).toHaveAttribute('aria-selected', 'false');
  await expect(record(page).getByRole('tab')).toHaveText(['synthetic-twin', 'synthetic-twin']);

  await row(page, 'Synthetic tower').click();
  const usedBy = page.locator('[data-window="codex-used-by"]');
  await expect(usedBy).toContainText('Synthetic twin one');
  await expect(usedBy).toContainText('Synthetic twin two');
  expect(errors).toEqual([]);
});

test('the_validation_title_counts_the_findings', async ({ page }) => {
  await openFloor(page);
  const title = page.locator('[data-window="codex-validation"] .tu-tabbar__title');
  const count = recordedPage('f04-choir-hall').findings.length;
  await expect(title.locator('.tu-badge')).toHaveText(String(count));
  await expect(title.locator('.tu-badge')).toHaveClass(/is-reject/);
  await row(page, 'Hearth').click();
  await expect(title.locator('.tu-badge')).toHaveCount(0);
});

test('record_tabs_follow_the_keys', async ({ page }) => {
  await openFloor(page);
  await row(page, 'Hearth').click();
  const tabs = record(page).getByRole('tab');
  await expect(tabs).toHaveText(['f04-choir-hall', 'hearth']);
  await expect(tabs.nth(1)).toHaveAttribute('aria-selected', 'true');

  await page.keyboard.press('Control+Tab');
  await expect(tabs.nth(0)).toHaveAttribute('aria-selected', 'true');
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('f04-choir-hall');

  await page.keyboard.press('Control+W');
  await expect(tabs).toHaveText(['hearth']);
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('hearth');

  await page.keyboard.press('Control+W');
  await expect(record(page).getByRole('tab')).toHaveCount(0);
  await expect(record(page)).toContainText('Pick one in the tree.');
});

test('a_tab_click_shows_its_record', async ({ page }) => {
  await openFloor(page);
  await row(page, 'Hearth').click();
  await record(page).getByRole('tab', { name: 'f04-choir-hall' }).click();
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('f04-choir-hall');
});

test('an_opened_record_starts_at_its_top', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 200 });
  await openFloor(page);
  const body = record(page).locator('.tu-window__body');
  const scrollTop = (): Promise<number> => body.evaluate((element) => element.scrollTop);
  await row(page, 'Hearth').click();
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('hearth');
  // Hearth's page is taller than its window, so a kept scroll position would show.
  expect(await body.evaluate((element) => element.scrollHeight - element.clientHeight)).toBeGreaterThan(40);
  await record(page).getByRole('tab', { name: 'f04-choir-hall' }).click();
  await body.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect.poll(scrollTop).toBeGreaterThan(40);
  await row(page, 'Hearth').click();
  await expect(record(page).locator('.tu-rechead__id')).toHaveText('hearth');
  await expect.poll(scrollTop).toBe(0);
});

test('another_file_with_the_same_id_starts_at_its_top', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 200 });
  await openProject(page, ['codex_tree-who', 'codex_tree-where-duplicate-id']);
  const body = record(page).locator('.tu-window__body');
  const scrollTop = (): Promise<number> => body.evaluate((element) => element.scrollTop);
  const twins = tree(page).locator('.tu-tree__row').filter({
    has: page.locator('.tu-tree__name', { hasText: /^Synthetic twin/ }),
  });
  const heading = record(page).locator('.tu-rechead__title');
  await twins.nth(0).click();
  await expect(heading).toHaveText('Synthetic twin one');
  // The page is taller than its window, so a kept scroll position would show.
  expect(await body.evaluate((element) => element.scrollHeight - element.clientHeight)).toBeGreaterThan(20);
  await body.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect.poll(scrollTop).toBeGreaterThan(20);
  await twins.nth(1).click();
  await expect(heading).toHaveText('Synthetic twin two');
  await expect.poll(scrollTop).toBe(0);
});

// AC: CE-32.2
test('tab_keys_do_nothing_while_another_layout_shows', async ({ page }) => {
  await openFloor(page);
  await row(page, 'Hearth').click();
  const tabs = record(page).getByRole('tab');
  await expect(tabs).toHaveText(['f04-choir-hall', 'hearth']);
  const rail = page.getByRole('navigation', { name: 'Layout presets' });
  await rail.getByRole('button', { name: 'Review', exact: true }).click();
  await expect(page.locator('.tu-statusbar')).toContainText('Review layout');

  await page.keyboard.press('Control+Tab');
  await page.keyboard.press('Control+W');
  await expect(page.locator('.tu-statusbar')).toContainText('Review layout');

  await rail.getByRole('button', { name: 'Codex', exact: true }).click();
  await expect(tabs).toHaveText(['f04-choir-hall', 'hearth']);
  await expect(tabs.nth(1)).toHaveAttribute('aria-selected', 'true');
});
