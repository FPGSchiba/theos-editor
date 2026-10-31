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
