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
const VOICES = corpusFile('records/lore/topics/synthetic-voices-in-world.md');
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
    // A voice title is drawn by its head (no chips) until the caret is on it, then it is raw with its chips (Task 10).
    const titleLinks = script.match(/> \[!voice\][^\n]*/)![0].match(/\[\[[^\]\n]+\]\]/g)!.length;
    await settle(page);
    expect(await page.locator(chips).count()).toBe(links - titleLinks);
    await page.evaluate((at) => { window.__harness!.focus(); window.__harness!.caretTo(at); }, script.indexOf('> [!voice]') + 3);
    await settle(page);
    expect(await page.locator(chips).count()).toBe(links);
    expect(await text(page)).toBe(script);
  });
});

test.describe('@harness codemirror · callouts', () => {
  const dot = '·';
  const dash = '—';
  const sample = [
    'Intro paragraph.',
    '',
    `> [!voice] [[mother-ilse]] ${dot} [[B-016]] ${dot} T1`,
    '> The Fates spin for those who climb, says the ledger of the frontier.',
    'and they will hear you.',
    '>',
    `> ${dash} a caption`,
    '',
    '> [!design]',
    '> A remark about the tier of it.',
    '',
    'Design note: the tier is a ledger.',
    '',
    `> [!note] n1 ${dot} [[f04-terrain-v1]] ${dot} open`,
    '> Body of the note.',
    '',
    'last',
  ].join('\n');
  const at = (page: Page, offset: number): Promise<void> =>
    page.evaluate((to) => { window.__harness!.focus(); window.__harness!.caretTo(to); }, offset);
  const lines = (page: Page, selector: string): Promise<string[]> =>
    page.locator(`#editor-host .cm-line${selector}`).evaluateAll((all) => all.map((line) => line.textContent ?? ''));
  const voiceLines = (page: Page): Promise<string[]> => lines(page, '[data-callout="voice"]');

  test('a voice block is a run of lines with the contract, a head and a caption, and the markers are hidden', async ({ page }) => {
    await openText(page, sample);
    await at(page, 0);
    await settle(page);
    expect(await page.locator('#editor-host [data-callout="voice"].tu-voice').count()).toBe(5);
    expect((await page.locator('#editor-host .tu-voice__head').allTextContents()).map((head) => head.trim())).toEqual([
      `In-world ${dot} Mother Ilse B-016 ${dot} T1 lens`,
    ]);
    expect(await lines(page, '.tu-voice__by')).toEqual(['a caption']);
    // Every line is drawn without its marker; the lazy line has none.
    expect((await voiceLines(page)).slice(1)).toEqual([
      'The Fates spin for those who climb, says the ledger of the frontier.', 'and they will hear you.', '', 'a caption',
    ]);
    expect(await text(page)).toBe(sample);
  });

  test('a line the caret is on shows its raw Markdown: the markers, the title and its chips', async ({ page }) => {
    await openText(page, sample);
    await at(page, sample.indexOf('The Fates') + 4);
    await settle(page);
    expect((await voiceLines(page))[1]).toBe('> The Fates spin for those who climb, says the ledger of the frontier.');
    expect((await voiceLines(page))[0]).not.toContain('> ');
    await at(page, sample.indexOf('[!voice]') + 3);
    await settle(page);
    const title = page.locator('#editor-host .cm-line[data-callout="voice"]').first();
    await expect(title).toContainText('> [!voice]');
    await expect(title.locator('.tu-link')).toHaveCount(2);
    await expect(page.locator('#editor-host .tu-voice__head')).toHaveCount(0);
    // The caption shows its dash once: as text when raw, as the design system's mark when drawn.
    await at(page, sample.indexOf(`${dash} a caption`) + 3);
    await settle(page);
    expect((await lines(page, '.tu-voice__by'))[0]).toBe(`> ${dash} a caption`);
    expect(await page.locator('#editor-host .tu-voice__by').evaluate((line) => getComputedStyle(line, '::before').content)).toBe('none');
    await at(page, 0);
    await settle(page);
    expect(await page.locator('#editor-host .tu-voice__by').evaluate((line) => getComputedStyle(line, '::before').content)).toContain(dash);
  });

  test('wherever the caret is, its line is raw: no marker hides under the caret', async ({ page }) => {
    await openText(page, sample);
    const start = sample.indexOf('> [!voice]');
    const end = sample.indexOf('last');
    for (let offset = start; offset <= end; offset += 3) {
      await at(page, offset);
      const where = await page.evaluate(() => {
        const node = window.getSelection()?.anchorNode;
        const line = (node instanceof Element ? node : node?.parentElement)?.closest('.cm-line');
        const doc = window.__harness!.text();
        const head = window.__harness!.caret();
        const from = doc.lastIndexOf('\n', head - 1) + 1;
        const next = doc.indexOf('\n', head);
        return { shown: line?.textContent ?? null, marker: doc.slice(from, next === -1 ? doc.length : next).startsWith('>') };
      });
      if (where.marker) expect(where.shown, `the line at ${offset}`).toMatch(/^>/);
    }
  });

  test('Backspace, Delete and a Shift selection take only what is drawn, never a hidden marker', async ({ page }) => {
    await openText(page, sample);
    const body = sample.indexOf('> The Fates');
    // Backspace at the start of a body line joins it to the title: one newline goes, the marker stays.
    await at(page, body);
    await page.keyboard.press('Backspace');
    expect(await text(page)).toBe(sample.slice(0, body - 1) + sample.slice(body));
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(sample);
    // Delete at the end of the line above a block: one newline goes (the marker is drawn once the caret joins the line).
    const above = sample.indexOf('\n> [!design]') - 1;
    await at(page, above);
    await page.keyboard.press('Delete');
    expect(await text(page)).toBe(sample.slice(0, above) + sample.slice(above + 1));
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(sample);
    // A selection from the end of a line into the next shows that line raw before anything is deleted.
    const titleEnd = sample.indexOf('\n', sample.indexOf('> [!voice]'));
    await at(page, titleEnd);
    await page.keyboard.press('Shift+ArrowRight');
    await page.keyboard.press('Shift+ArrowRight');
    await settle(page);
    expect((await voiceLines(page))[1]).toMatch(/^> The Fates/);
    await page.keyboard.press('Delete');
    expect(await text(page)).toBe(sample.slice(0, titleEnd) + sample.slice(titleEnd + 2));
  });

  test('the arrow keys walk through a block line by line, and the bytes stay the file', async ({ page }) => {
    await openText(page, sample);
    await at(page, 0);
    const heads: number[] = [];
    for (let step = 0; step < 12; step++) {
      await page.keyboard.press('ArrowDown');
      heads.push(await caret(page));
    }
    expect(heads).toEqual([...heads].sort((a, b) => a - b));
    expect(new Set(heads).size).toBe(heads.length);
    for (let step = 0; step < 12; step++) await page.keyboard.press('ArrowUp');
    expect(await caret(page)).toBeLessThanOrEqual(sample.indexOf('\n'));
    expect(await text(page)).toBe(sample);
  });

  test('a click on a head puts the caret on the title line, which turns raw', async ({ page }) => {
    await openText(page, sample);
    await at(page, 0);
    await settle(page);
    await page.locator('#editor-host .tu-voice__head').click();
    await expect(page.locator('#editor-host .tu-voice__head')).toHaveCount(0);
    const head = await caret(page);
    expect(head).toBeGreaterThanOrEqual(sample.indexOf('> [!voice]'));
    expect(head).toBeLessThanOrEqual(sample.indexOf('\n', sample.indexOf('> [!voice]')));
    expect(await text(page)).toBe(sample);
  });

  test('design callouts, Design note lines and note callouts follow the contract', async ({ page }) => {
    await openText(page, sample);
    await at(page, 0);
    await settle(page);
    const design = page.locator('#editor-host [data-callout="design"]');
    await expect(design).toHaveCount(2);
    await expect(design.last()).toHaveText('A remark about the tier of it.');
    await expect(page.locator('#editor-host .tu-note')).toHaveText('Design note: the tier is a ledger.');
    await expect(page.locator('#editor-host .tu-note[data-callout]')).toHaveCount(0);
    const note = page.locator('#editor-host [data-callout="note"][data-note="n1"][data-note-state="open"]');
    await expect(note).toHaveCount(2);
    await expect(page.locator('#editor-host .hx-note__head .tu-link')).toHaveCount(1);
  });

  test('developer words are marked in voice text only, as whole words', async ({ page }) => {
    await openText(page, sample.replace('Intro paragraph.', 'A ledger in the intro.'));
    await at(page, 0);
    await settle(page);
    // Voice text has "ledger" (and "frontier", which holds "tier" but is another word); the design callout,
    // the Design note line and the prose are not voice text.
    expect(await page.locator('#editor-host [data-dev-word]').allTextContents()).toEqual(['ledger']);
    expect(await page.locator('#editor-host [data-dev-word]').first().getAttribute('title')).toBe('developer word');
    expect(await page.locator('#editor-host [data-callout="voice"] [data-dev-word]').count()).toBe(1);
  });

  test('a block in code is not a callout, and a Design note line in code is plain', async ({ page }) => {
    const code = '```\n> [!voice] [[mother-ilse]]\n> text\nDesign note: x\n```\n\nafter';
    await openText(page, code);
    await at(page, code.length);
    await settle(page);
    await expect(page.locator('#editor-host [data-callout]')).toHaveCount(0);
    await expect(page.locator('#editor-host .tu-note')).toHaveCount(0);
  });

  test('note-state replaces the last segment of the title when it is a state, and appends one when it is not', async ({ page }) => {
    const notes = [
      `> [!note] n1 ${dot} open`, '> a', '',
      `> [!note] n2 ${dot} [[f04-terrain-v1]]`, '> b', '',
      `> [!note] n3 ${dot} resolved by [[f04-terrain-v1]]`, '> c', '',
      '> [!note] n4', '> d', '',
      `> [!note] n10 ${dot} dropped`, '> e',
    ].join('\n');
    await openText(page, notes);
    const apply = (note: string, state: string): Promise<void> =>
      page.evaluate(([n, s]) => window.__harness!.apply({ id: 'probe', kind: 'note-state', note: n!, state: s! }), [note, state]);
    await apply('n1', 'resolved by [[x]]');
    await apply('n2', 'open');
    await apply('n3', 'dropped');
    await apply('n4', 'open');
    await apply('n1', 'dropped');
    expect(await text(page)).toBe([
      `> [!note] n1 ${dot} dropped`, '> a', '',
      `> [!note] n2 ${dot} [[f04-terrain-v1]] ${dot} open`, '> b', '',
      `> [!note] n3 ${dot} dropped`, '> c', '',
      `> [!note] n4 ${dot} open`, '> d', '',
      `> [!note] n10 ${dot} dropped`, '> e',
    ].join('\n'));
    await at(page, 0);
    await settle(page);
    const states = await page.locator('#editor-host .cm-line[data-callout="note"]').evaluateAll((all) =>
      all.map((line) => `${line.getAttribute('data-note')}=${line.getAttribute('data-note-state')}`));
    expect(states).toEqual(['n1=dropped', 'n1=dropped', 'n2=open', 'n2=open', 'n3=dropped', 'n3=dropped', 'n4=open', 'n4=open', 'n10=dropped', 'n10=dropped']);
    await expect(page.evaluate(() => window.__harness!.apply({ id: 'probe', kind: 'note-state', note: 'n9', state: 'open' }))).rejects.toThrow(/no note/);
    for (let n = 0; n < 5; n++) await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(notes);
  });

  test('a note head and a voice head follow the index, and the text stays', async ({ page }) => {
    await openText(page, sample);
    await at(page, 0);
    await settle(page);
    await page.evaluate(() => window.__harness!.updateIndex([
      { id: 'mother-ilse', kind: 'npc', title: 'Ilse, renamed' }, { id: 'f04-terrain-v1', kind: 'concept-version', title: 'Terrain, renamed' },
    ]));
    await expect(page.locator('#editor-host .tu-voice__head')).toContainText('Ilse, renamed');
    await expect(page.locator('#editor-host .hx-note__head .tu-link__title')).toHaveText('Terrain, renamed');
    expect(await text(page)).toBe(sample);
  });

  test('Source mode shows every callout raw, Write mode draws them again', async ({ page }) => {
    await openText(page, sample);
    await page.evaluate(() => window.__harness!.setMode('source'));
    await settle(page);
    expect(await page.locator('#editor-host [data-callout], #editor-host .tu-note, #editor-host [data-dev-word]').count()).toBe(0);
    expect(await visibleText(page)).toContain('> [!voice]');
    await page.evaluate(() => window.__harness!.setMode('write'));
    await at(page, 0);
    await expect(page.locator('#editor-host .tu-voice__head')).toHaveCount(1);
    expect(await text(page)).toBe(sample);
  });

  test('typing in a voice line keeps its markers and its developer words', async ({ page }) => {
    await openText(page, sample);
    await at(page, sample.indexOf('says the ledger'));
    await page.keyboard.type('and loudly ');
    expect(await text(page)).toBe(sample.replace('says the ledger', 'and loudly says the ledger'));
    await settle(page);
    expect((await voiceLines(page))[1]).toMatch(/^> The Fates spin .*and loudly says the ledger/);
    await expect(page.locator('#editor-host [data-callout="voice"] [data-dev-word]')).toHaveText('ledger');
  });

  test('a note-state right after typing on the same title is its own undo step', async ({ page }) => {
    const note = '> [!note] n1 · [[v]]\n> a';
    await openText(page, note);
    await at(page, note.indexOf('\n'));
    await page.keyboard.type('x');
    await page.evaluate(() => window.__harness!.apply({ id: 'probe', kind: 'note-state', note: 'n1', state: 'dropped' }));
    expect(await text(page)).toBe('> [!note] n1 · [[v]]x · dropped\n> a');
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe('> [!note] n1 · [[v]]x\n> a');
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(note);
  });
});

test.describe('@harness codemirror · beats', () => {
  const sample = [
    'Intro.',
    '',
    '## Beat 1 — Arrival',
    'at:: [[fates-shrine]]',
    'when:: T1',
    '',
    'Prose after the beat.',
    '',
    '## Beat 2 — Bare',
    '',
    'More prose.',
  ].join('\n');
  const at = (page: Page, offset: number): Promise<void> =>
    page.evaluate((to) => { window.__harness!.focus(); window.__harness!.caretTo(to); }, offset);
  const row1 = '[data-beat-row="1"]';

  test('the heading and the lines show raw under the caret and rendered elsewhere', async ({ page }) => {
    await openText(page, sample);
    await at(page, 0);
    await settle(page);
    await expect(page.locator('[data-beat="1"]')).toHaveText('1 · Arrival');
    await expect(page.locator(row1)).toHaveCount(1);
    await at(page, sample.indexOf('when::'));
    await settle(page);
    await expect(page.locator(row1)).toHaveCount(0);
    expect(await visibleText(page)).toContain('at:: ');
    await at(page, sample.indexOf('## Beat 1') + 3);
    await settle(page);
    await expect(page.locator('[data-beat="1"]')).toHaveCount(0);
    expect(await visibleText(page)).toContain('## Beat 1 — Arrival');
    await expect(page.locator(row1)).toHaveCount(1);
  });

  test('Backspace after the row and a select-all delete never take unseen lines', async ({ page }) => {
    await openText(page, sample);
    await at(page, sample.indexOf('\nProse after') + 1);
    await page.keyboard.press('Backspace');
    // One newline is gone (the blank line); both attribute lines are still in the text.
    const after = await text(page);
    expect(after).toBe(sample.replace('T1\n\nProse', 'T1\nProse'));
    await at(page, sample.indexOf('## Beat 1') - 1);
    await page.keyboard.press('Shift+ArrowDown');
    await page.keyboard.press('Shift+ArrowDown');
    await settle(page);
    // The selection touches the heading and the first attribute lines, which are drawn raw.
    await expect(page.locator(row1)).toHaveCount(0);
  });

  test('typing in the row never reaches the editor, and a repeated key does not throw', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await openText(page, sample);
    await at(page, 0);
    await page.locator(`${row1} [data-beat-param="at"] .tu-beat__key`).click();
    await page.locator(`${row1} [data-beat-input]`).fill('x');
    await page.keyboard.type('yz');
    expect(await text(page)).toBe(sample);
    await page.locator(`${row1} [data-beat-input]`).press('Enter');
    expect(await text(page)).toBe(sample.replace('at:: [[fates-shrine]]', 'at:: xyz'));
    // A second `at::` line (a duplicate key) draws without an error.
    await page.evaluate(() => window.__harness!.focus());
    await at(page, (await text(page)).indexOf('when::'));
    await page.keyboard.type('at:: dup\n');
    await at(page, 0);
    await settle(page);
    expect(errors).toEqual([]);
    await expect(page.locator(`${row1} [data-beat-param="at"]`)).toHaveCount(2);
  });

  test('a bare beat gets a row with "+ add" and the line goes after the heading', async ({ page }) => {
    await openText(page, sample);
    await at(page, 0);
    const row = page.locator('[data-beat-row="2"]');
    await row.locator('[data-beat-add]').click();
    await row.locator('[data-beat-add-key="says"]').click();
    await row.locator('[data-beat-input]').fill('[[B-016]]');
    await row.locator('[data-beat-input]').press('Enter');
    expect(await text(page)).toBe(sample.replace('## Beat 2 — Bare\n', '## Beat 2 — Bare\nsays:: [[B-016]]\n'));
  });
});
