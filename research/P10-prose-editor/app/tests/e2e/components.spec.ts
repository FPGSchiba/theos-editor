import { expect, test } from '@playwright/test';

// The DOM contract of the shared components (context.md), checked on the showcase page
// (`/?view=components`). Every prototype renders through these components, so the later
// suites can rely on exactly these selectors.

test.beforeEach(async ({ page }) => {
  await page.goto('/?view=components');
});

test.describe('@harness link chip', () => {
  test('a resolved chip carries its id, kind and title and is not spell-checked', async ({ page }) => {
    const chip = page.locator('[data-showcase="chips"] .tu-link[data-link-id="mother-ilse"]');
    await expect(chip).toHaveCount(1);
    await expect(chip).not.toHaveClass(/is-broken/);
    await expect(chip.locator('.tu-kind[data-kind="npc"]')).toHaveCount(1);
    await expect(chip.locator('.tu-link__title')).toHaveText('Mother Ilse');
    expect(await chip.evaluate((node) => (node as HTMLElement).spellcheck)).toBe(false);
  });

  test('an alias is the shown text, a code id is shown before the title', async ({ page }) => {
    await expect(page.locator('[data-showcase="chips"] .tu-link[data-link-id="fates-shrine"] .tu-link__title')).toHaveText("Fates' shrine");
    const truth = page.locator('[data-showcase="chips"] .tu-link[data-link-id="T-012"]');
    await expect(truth.locator('.tu-link__id')).toHaveText('T-012');
    await expect(truth.locator('.tu-kind[data-kind="truth"]')).toHaveCount(1);
  });

  test('an unresolved id is a broken chip', async ({ page }) => {
    const chip = page.locator('[data-showcase="chips"] .tu-link[data-link-id="song-cellar"]');
    await expect(chip).toHaveClass(/is-broken/);
    await expect(chip.locator('.tu-link__title')).toHaveText('song-cellar');
  });
});

test.describe('@harness link popover', () => {
  test.beforeEach(async ({ page }) => {
    await page.locator('[data-showcase-open-popover]').click();
  });

  test('rows carry the id and kind of each option', async ({ page }) => {
    const popover = page.locator('.tu-pop[data-link-popover]');
    await expect(popover).toHaveCount(1);
    await expect(popover.locator('.tu-pop__row[data-link-option="T-012"][data-kind="truth"]')).toHaveCount(1);
    await expect(popover.locator('.tu-pop__row[data-link-option="B-020"][data-kind="belief"]')).toHaveCount(1);
    await expect(popover.locator('.tu-pop__row[data-link-option="C-033"][data-kind="clue"]')).toHaveCount(1);
  });

  test('arrow keys move, Enter picks the active row', async ({ page }) => {
    const rows = page.locator('.tu-pop__row');
    await expect(rows.first()).toHaveClass(/is-active/);
    await page.keyboard.press('ArrowDown');
    await expect(rows.nth(1)).toHaveClass(/is-active/);
    const second = await rows.nth(1).getAttribute('data-link-option');
    await page.keyboard.press('Enter');
    await expect(page.locator('[data-showcase-picked]')).toHaveText(second!);
  });

  test('a click picks that row and the popover goes away', async ({ page }) => {
    await page.locator('.tu-pop__row[data-link-option="C-033"]').click();
    await expect(page.locator('[data-showcase-picked]')).toHaveText('C-033');
    await expect(page.locator('.tu-pop[data-link-popover]')).toHaveCount(0);
  });

  test('Escape closes without picking', async ({ page }) => {
    await page.keyboard.press('Escape');
    await expect(page.locator('[data-showcase-closed]')).toHaveText('1');
    await expect(page.locator('[data-showcase-picked]')).toHaveText('');
    await expect(page.locator('.tu-pop[data-link-popover]')).toHaveCount(0);
  });
});

test.describe('@harness callouts', () => {
  test('a voice block has a head naming speaker, beliefs and tier, and a caption', async ({ page }) => {
    const voice = page.locator('[data-callout="voice"]');
    await expect(voice).toHaveClass(/tu-voice/);
    const head = voice.locator('.tu-voice__head');
    await expect(head).toContainText('Throat-Kin binders');
    await expect(head).toContainText('B-020');
    await expect(head).toContainText('T2');
    await expect(voice.locator('.tu-voice__by')).toHaveText('a binder, teaching the naming-song');
  });

  test('developer words are marked with a title', async ({ page }) => {
    const words = page.locator('[data-dev-word]');
    await expect(words).toHaveText(['confirm', 'tier']);
    await expect(words.first()).toHaveAttribute('title', 'developer word');
    await expect(page.locator('[data-callout="voice"] [data-dev-word]')).toHaveCount(2);
  });

  test('a design callout and a Design note line', async ({ page }) => {
    await expect(page.locator('[data-callout="design"]')).toHaveCount(1);
    await expect(page.locator('.tu-note')).toContainText('Design note:');
  });

  test('a note callout carries its number and state', async ({ page }) => {
    const open = page.locator('[data-callout="note"][data-note="n1"]');
    await expect(open).toHaveAttribute('data-note-state', 'open');
    await expect(open.locator('.tu-link[data-link-id="f04-terrain-v2"]')).toHaveCount(1);
    await expect(page.locator('[data-callout="note"][data-note="n4"]')).toHaveAttribute('data-note-state', 'resolved');
  });
});

test.describe('@harness beats', () => {
  test('a heading shows the number and the name', async ({ page }) => {
    await expect(page.locator('[data-beat="1"]')).toHaveText('1 · Arrival');
    await expect(page.locator('[data-beat="2"]')).toHaveText('2 · The murmur');
  });

  test('a row holds the params, with links as chips, and is not spell-checked', async ({ page }) => {
    const row = page.locator('[data-beat-row="1"]');
    await expect(row.locator('.tu-beat')).toHaveCount(1);
    await expect(row.locator('[data-beat-param="at"] .tu-link[data-link-id="fates-shrine"]')).toHaveCount(1);
    await expect(row.locator('[data-beat-param="when"]')).toContainText('T1');
    expect(await row.evaluate((node) => (node as HTMLElement).spellcheck)).toBe(false);
  });

  test('a finding shows on its beat only', async ({ page }) => {
    await expect(page.locator('[data-beat-row="2"] [data-beat-finding][data-finding-rule="never-confirm"]')).toHaveCount(1);
    await expect(page.locator('[data-beat-row="1"] [data-beat-finding]')).toHaveCount(0);
  });

  test('clicking a param opens its field; Enter commits the raw value', async ({ page }) => {
    await page.locator('[data-beat-row="1"] [data-beat-param="when"]').click();
    const input = page.locator('[data-beat-row="1"] [data-beat-input]');
    await expect(input).toHaveValue('T1');
    await input.fill('T2');
    await input.press('Enter');
    await expect(page.locator('[data-showcase-commits]')).toHaveText('when:: T2');
    await expect(input).toHaveCount(0);
  });

  test('+ add offers the missing keys; a new param is committed with its key', async ({ page }) => {
    const row = page.locator('[data-beat-row="1"]');
    await row.locator('[data-beat-add]').click();
    await expect(row.locator('[data-beat-add-key]')).toHaveCount(1);
    await expect(row.locator('[data-beat-add-key="reveals"]')).toHaveCount(1);
    await row.locator('[data-beat-add-key="reveals"]').click();
    const input = row.locator('[data-beat-input]');
    await input.fill('[[C-031]]');
    await input.press('Enter');
    await expect(page.locator('[data-showcase-commits]')).toHaveText('reveals:: [[C-031]]');
    await expect(row.locator('[data-beat-param="reveals"] .tu-link[data-link-id="C-031"]')).toHaveCount(1);
  });

  test('Escape drops an edit without committing', async ({ page }) => {
    await page.locator('[data-beat-row="1"] [data-beat-param="at"]').click();
    await page.locator('[data-beat-input]').press('Escape');
    await expect(page.locator('[data-beat-input]')).toHaveCount(0);
    await expect(page.locator('[data-showcase-commits]')).toHaveText('');
  });
});
