import { expect, test, type Page } from '@playwright/test';
import { splitFrontMatter } from '../../src/harness/frontmatter.ts';
import { toLf } from '../../src/harness/oracle.ts';
import { corpusFile, settle, visibleText } from './checks.ts';
import { loadFile } from './helpers.ts';

// Probes of the CodeMirror 6 base for what the measured suites cannot see: they always place the
// caret before they type, they only load corpus files, and they never look at the first body line
// of a short record. Each probe loads the `codemirror` variant itself, whatever P10_VARIANTS says,
// and none of them changes a shared suite.

const SCRIPT = corpusFile('records/story/scripts/S-004.md');
const EMPTY_BODY = corpusFile('records/people/hearth-folk/fates-cult/mother-ilse.md');

/** Mount the `codemirror` variant on a text that is not a corpus file. */
async function openText(page: Page, text: string): Promise<void> {
  if (page.url() === 'about:blank') await page.goto('/');
  await page.waitForFunction(() => window.__harness !== undefined);
  await page.evaluate((content) => window.__harness!.load('codemirror', content, 'records/probe.md'), text);
}

const openFile = async (page: Page, file: typeof SCRIPT): Promise<string> =>
  toLf(await loadFile(page, 'codemirror', file));
const text = (page: Page): Promise<string> => page.evaluate(() => window.__harness!.text());
const caret = (page: Page): Promise<number> => page.evaluate(() => window.__harness!.caret());
const focus = (page: Page): Promise<void> => page.evaluate(() => window.__harness!.focus());
const canUndo = (page: Page): Promise<boolean> => page.evaluate(() => window.__harness!.canUndo());

test.describe('@harness codemirror · the caret after load', () => {
  test('typing right after load lands at the body start', async ({ page }) => {
    const source = await openFile(page, SCRIPT);
    const { frontMatter } = splitFrontMatter(source);
    expect(await caret(page)).toBe(frontMatter.length);
    await focus(page);
    await page.keyboard.type('abc');
    expect(await text(page)).toBe(`${frontMatter}abc${source.slice(frontMatter.length)}`);
    expect(await caret(page)).toBe(frontMatter.length + 3);
  });

  test('ArrowRight right after load steps over one character of the body', async ({ page }) => {
    const source = await openFile(page, SCRIPT);
    const start = splitFrontMatter(source).frontMatter.length;
    await focus(page);
    await page.keyboard.press('ArrowRight');
    expect(await caret(page)).toBe(start + 1);
  });

  test('a record with an empty body takes typing right after load', async ({ page }) => {
    const source = await openFile(page, EMPTY_BODY);
    await focus(page);
    await page.keyboard.type('abc');
    expect(await text(page)).toBe(`${source}abc`);
  });

  test('switching back to Write moves a caret out of the front matter', async ({ page }) => {
    const source = await openFile(page, SCRIPT);
    const start = splitFrontMatter(source).frontMatter.length;
    await page.evaluate(() => window.__harness!.setMode('source'));
    await page.evaluate(() => { window.__harness!.focus(); window.__harness!.caretTo(3); });
    expect(await caret(page)).toBe(3);
    await page.evaluate(() => window.__harness!.setMode('write'));
    expect(await caret(page)).toBe(start);
    await page.keyboard.type('x');
    expect(await text(page)).toBe(`${source.slice(0, start)}x${source.slice(start)}`);
  });

  test('arrow keys, Home and Shift+ArrowUp cannot enter the hidden front matter', async ({ page }) => {
    const source = await openFile(page, SCRIPT);
    const start = splitFrontMatter(source).frontMatter.length;
    await focus(page);
    for (const key of ['ArrowUp', 'ArrowLeft', 'ControlOrMeta+Home', 'Shift+ArrowUp', 'Shift+ArrowLeft']) {
      await page.keyboard.press(key);
      expect(await caret(page), `${key} left the caret at the body start`).toBe(start);
    }
    expect(await text(page)).toBe(source);
  });
});

test.describe('@harness codemirror · front matter that ends the file without a newline', () => {
  const source = '---\nid: probe\n---';

  test('Write mode: typing at its end starts the body on a new line, one undo takes both back', async ({ page }) => {
    await openText(page, source);
    expect(await caret(page)).toBe(source.length);
    await focus(page);
    await page.keyboard.insertText('abc');
    expect(await text(page)).toBe(`${source}\nabc`);
    expect(await caret(page)).toBe(source.length + 4);
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(source);
    expect(await canUndo(page)).toBe(false);
  });

  test('Write mode: typed key by key it is the same, and the body is not hidden afterwards', async ({ page }) => {
    await openText(page, source);
    await focus(page);
    await page.keyboard.type('abc');
    expect(await text(page)).toBe(`${source}\nabc`);
    await settle(page);
    expect(await visibleText(page)).toContain('abc');
    expect(await visibleText(page)).not.toContain('id: probe');
    // The newline is part of the front matter now: Backspace at the body start cannot take it.
    await page.evaluate((at) => window.__harness!.caretTo(at), source.length + 1);
    await page.keyboard.press('Backspace');
    expect(await text(page)).toBe(`${source}\nabc`);
  });

  test('Source mode: typing at its end does not touch the closing fence', async ({ page }) => {
    await openText(page, source);
    await page.evaluate(() => window.__harness!.setMode('source'));
    await page.evaluate((at) => { window.__harness!.focus(); window.__harness!.caretTo(at); }, source.length);
    await page.keyboard.insertText('abc');
    expect(await text(page)).toBe(`${source}\nabc`);
  });
});

test.describe('@harness codemirror · the first body line', () => {
  const record = '---\nid: probe\n---\n# First *em*\n\nSecond line.';

  test('a heading on the first body line is rendered, and raw under the caret', async ({ page }) => {
    await openText(page, record);
    // The caret starts on the first body line, which is raw; the rendered look needs it elsewhere.
    await page.evaluate((at) => window.__harness!.caretTo(at), record.indexOf('Second'));
    await settle(page);
    const first = page.locator('#editor-host .cm-line').first();
    await expect(first).toHaveClass(/ms-title/);
    await expect(first).toHaveText('First em');
    expect(await visibleText(page)).not.toContain('id: probe');
    await page.evaluate((at) => window.__harness!.caretTo(at), record.indexOf('First'));
    await expect(first).toHaveText('# First *em*');
    await expect(first).toHaveClass(/ms-title/);
  });

  test('the first body line is a line of its own, not merged with the front matter', async ({ page }) => {
    const plain = '---\nid: probe\n---\nPlain *line*.\n\nNext.';
    await openText(page, plain);
    await page.evaluate((at) => window.__harness!.caretTo(at), plain.indexOf('Next'));
    await settle(page);
    const lines = await page.locator('#editor-host .cm-line').allTextContents();
    expect(lines.slice(0, 3)).toEqual(['Plain line.', '', 'Next.']);
  });
});

test.describe('@harness codemirror · links', () => {
  const sample = 'Go to [[fates-shrine]] now, [[mother-ilse]][[B-020|a song]] end.\n\n`[[fates-shrine]]` stays code.\n\n```\n[[mother-ilse]]\n```\n\nlast';
  const chips = '#editor-host .tu-link';
  const popover = '.tu-pop[data-link-popover]';
  const at = (page: Page, offset: number): Promise<void> =>
    page.evaluate((to) => { window.__harness!.focus(); window.__harness!.caretTo(to); }, offset);

  test('links in code stay raw, the others are chips, and the bytes are the file', async ({ page }) => {
    await openText(page, sample);
    await settle(page);
    expect(await page.locator(chips).evaluateAll((all) => all.map((chip) => chip.getAttribute('data-link-id')))).toEqual(
      ['fates-shrine', 'mother-ilse', 'B-020'],
    );
    expect(await visibleText(page)).toContain('`[[fates-shrine]]` stays code.');
    expect(await visibleText(page)).toContain('[[mother-ilse]]');
    expect(await text(page)).toBe(sample);
  });

  test('the caret steps over a chip both ways, Shift selects it whole, Delete and Backspace take it whole', async ({ page }) => {
    await openText(page, sample);
    const link = '[[mother-ilse]]';
    const alias = '[[B-020|a song]]';
    const start = sample.indexOf(link);
    await at(page, start);
    await page.keyboard.press('ArrowRight');
    expect(await caret(page)).toBe(start + link.length);
    await page.keyboard.press('ArrowRight');
    expect(await caret(page)).toBe(start + link.length + alias.length);
    await page.keyboard.press('ArrowLeft');
    expect(await caret(page)).toBe(start + link.length);
    await page.keyboard.press('ArrowLeft');
    expect(await caret(page)).toBe(start);
    await page.keyboard.press('Shift+ArrowRight');
    await page.keyboard.press('Delete');
    expect(await text(page)).toBe(sample.replace(link, ''));
    await page.keyboard.press('Delete');
    expect(await text(page)).toBe(sample.replace(link + alias, ''));
    // Backspace behind a chip: one press, one link.
    await page.keyboard.press('ControlOrMeta+z');
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(sample);
    await at(page, start + link.length + alias.length);
    await page.keyboard.press('Backspace');
    expect(await text(page)).toBe(sample.replace(alias, ''));
    await page.keyboard.press('Backspace');
    expect(await text(page)).toBe(sample.replace(link + alias, ''));
    for (let undo = 0; undo < 5 && (await text(page)) !== sample; undo++) await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(sample);
    await settle(page);
    expect(await page.locator(chips).count()).toBe(3);
  });

  test('a click on a chip puts the caret before or behind the whole link', async ({ page }) => {
    await openText(page, sample);
    const box = (await page.locator(`${chips}[data-link-id="mother-ilse"]`).boundingBox())!;
    await page.mouse.click(box.x + box.width * 0.8, box.y + box.height / 2);
    const link = '[[mother-ilse]]';
    expect([sample.indexOf(link), sample.indexOf(link) + link.length]).toContain(await caret(page));
    await page.keyboard.type('Z');
    expect(await text(page)).toMatch(/Z\[\[mother-ilse\]\]|\[\[mother-ilse\]\]Z/);
    expect(await page.locator(chips).count()).toBe(3);
  });

  test('a link typed by hand is raw and marked while it is typed, a chip when it is finished', async ({ page }) => {
    await openText(page, sample);
    await at(page, sample.length);
    await page.keyboard.type('x [[fates-sh');
    await expect(page.locator('#editor-host .tu-typing__q')).toHaveText('[[fates-sh');
    expect(await page.locator(chips).count()).toBe(3);
    await page.keyboard.type('rine]]');
    await expect(page.locator('#editor-host .tu-typing__q')).toHaveCount(0);
    await expect(page.locator(chips)).toHaveCount(4);
    expect(await text(page)).toBe(`${sample}x [[fates-shrine]]`);
  });

  test('the keyboard picks: ArrowDown moves, Enter links and closes, undo leaves the typed text', async ({ page }) => {
    await openText(page, sample);
    await at(page, sample.length);
    await page.keyboard.type(' [[Listen');
    await expect(page.locator(popover)).toHaveCount(1);
    const rows = await page.locator(`${popover} .tu-pop__row`).evaluateAll((all) => all.map((row) => row.getAttribute('data-link-option')));
    expect(rows.length).toBeGreaterThan(1);
    await page.keyboard.press('ArrowDown');
    // An update that does not touch the typed text (the caret is set where it is) leaves the active row alone.
    await page.evaluate(() => window.__harness!.caretTo(window.__harness!.caret()));
    await settle(page);
    await expect(page.locator(`${popover} .tu-pop__row.is-active`)).toHaveAttribute('data-link-option', rows[1]!);
    await page.keyboard.press('Enter');
    await expect(page.locator(popover)).toHaveCount(0);
    const link = `[[${rows[1]}]]`;
    expect(await text(page)).toBe(`${sample} ${link}`);
    expect(await caret(page)).toBe(sample.length + 1 + link.length);
    // The editor kept its focus: typing goes behind the chip.
    await page.keyboard.type('!');
    expect(await text(page)).toBe(`${sample} ${link}!`);
    // The pick and the `!` behind it are one history step; what is left is the text as it was typed.
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(`${sample} [[Listen`);
    await expect(page.locator(popover)).toHaveCount(0);
  });

  test('Escape keeps the typed text plain: no popover for it, however much is typed after', async ({ page }) => {
    await openText(page, sample);
    await at(page, sample.length);
    await page.keyboard.type(' [[son');
    await expect(page.locator(popover)).toHaveCount(1);
    await page.keyboard.press('Escape');
    await expect(page.locator(popover)).toHaveCount(0);
    await expect(page.locator('#editor-host .tu-typing__q')).toHaveCount(0);
    await page.keyboard.type('g');
    await expect(page.locator(popover)).toHaveCount(0);
    // A new `[[` opens it again.
    await page.keyboard.type(' [[B-0');
    await expect(page.locator(popover)).toHaveCount(1);
    expect(await text(page)).toBe(`${sample} [[song [[B-0`);
  });

  test('the popover closes when the caret leaves the typed text, on blur, and with the editor', async ({ page }) => {
    await openText(page, sample);
    await at(page, sample.length);
    await page.keyboard.type(' [[Lis');
    await expect(page.locator(popover)).toHaveCount(1);
    for (let step = 0; step < 6; step++) await page.keyboard.press('ArrowLeft');
    await expect(page.locator(popover)).toHaveCount(0);
    await at(page, sample.length);
    await page.keyboard.type('[[x');
    await expect(page.locator(popover)).toHaveCount(1);
    await page.evaluate(() => (document.activeElement as HTMLElement).blur());
    await expect(page.locator(popover)).toHaveCount(0);
    await at(page, sample.length);
    await page.keyboard.type(' [[y');
    await expect(page.locator(popover)).toHaveCount(1);
    await page.evaluate(() => window.__harness!.load('codemirror', 'plain', 'records/probe.md'));
    await expect(page.locator('.cm-link-popover')).toHaveCount(0);
  });

  test('chips are rebuilt only when what they draw changes', async ({ page }) => {
    await openText(page, sample);
    await page.evaluate(() => document.querySelectorAll('#editor-host .tu-link').forEach((chip) => Object.assign(chip, { p10: true })));
    // Typing on the same line and a new index that does not touch these records keep the chips.
    await at(page, sample.indexOf('Go'));
    await page.keyboard.type('abc ');
    await page.evaluate(() => window.__harness!.updateIndex([{ id: 'unrelated', kind: 'topic', title: 'Unrelated' }]));
    await settle(page);
    const kept = (): Promise<boolean[]> =>
      page.locator(chips).evaluateAll((all) => all.map((chip) => (chip as HTMLElement & { p10?: boolean }).p10 === true));
    expect(await kept()).toEqual([true, true, true]);
    await page.evaluate(() => window.__harness!.updateIndex([{ id: 'mother-ilse', kind: 'npc', title: 'Ilse, renamed' }]));
    await expect(page.locator(`${chips}[data-link-id="mother-ilse"] .tu-link__title`)).toHaveText('Ilse, renamed');
    // The chip next to it may be rebuilt with it (CodeMirror redraws the changed range); none is left behind.
    expect((await kept()).slice(0, 2)).toEqual([true, false]);
    expect(await page.locator(chips).count()).toBe(3);
    // A dangling link becomes a resolved one when its record appears.
    await page.evaluate(() => window.__harness!.load('codemirror', '[[later]] and [[later]]', 'records/probe.md'));
    await expect(page.locator(`${chips}.is-broken`)).toHaveCount(2);
    await page.evaluate(() => window.__harness!.updateIndex([{ id: 'later', kind: 'topic', title: 'Later' }]));
    await expect(page.locator(`${chips}.is-broken`)).toHaveCount(0);
    await expect(page.locator(`${chips} .tu-link__title`)).toHaveText(['Later', 'Later']);
  });

  test('Source mode shows the links raw, Write mode draws the chips again', async ({ page }) => {
    await openText(page, sample);
    await page.evaluate(() => window.__harness!.setMode('source'));
    await settle(page);
    expect(await page.locator(chips).count()).toBe(0);
    expect(await visibleText(page)).toContain('[[fates-shrine]]');
    await page.evaluate(() => window.__harness!.setMode('write'));
    await expect(page.locator(chips)).toHaveCount(3);
  });

  test('chips in a callout title and a beat attribute keep their bytes', async ({ page }) => {
    const script = toLf(await loadFile(page, 'codemirror', SCRIPT));
    const links = script.match(/\[\[[^\]\n]+\]\]/g)!.length;
    await settle(page);
    expect(await page.locator(chips).count()).toBe(links);
    expect(await text(page)).toBe(script);
  });
});
