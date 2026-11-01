import { expect, test, type Page } from '@playwright/test';
import { resolveAnchor } from '../../src/harness/anchor.ts';
import { splitFrontMatter } from '../../src/harness/frontmatter.ts';
import { toLf } from '../../src/harness/oracle.ts';
import type { Anchor } from '../../src/harness/types.ts';
import { corpusFile, settle, visibleText } from './checks.ts';
import { loadFile } from './helpers.ts';

// Probes of the ProseMirror base (stock column) for what the measured suites cannot see. The
// stock serializer rewrites the body, so the byte comparisons of the suites say little about
// whether the caret mapping, the typing, the undo and the front matter protection work: these
// probes check them without comparing the whole file. Each probe loads `prosemirror:stock`
// itself, whatever P10_VARIANTS says, and none of them changes a shared suite.

const VARIANT = 'prosemirror:stock';
const SCRIPT = corpusFile('records/story/scripts/S-004.md');
const BELIEF = corpusFile('records/lore/beliefs/B-901.md');
const HALL = corpusFile('records/world/hearth/f04-choir-hall.md');
const EMPTY_BODY = corpusFile('records/people/hearth-folk/fates-cult/mother-ilse.md');

const open = async (page: Page, file: typeof SCRIPT): Promise<string> => toLf(await loadFile(page, VARIANT, file));
const text = (page: Page): Promise<string> => page.evaluate(() => window.__harness!.text());
const caret = (page: Page): Promise<number> => page.evaluate(() => window.__harness!.caret());
const caretTo = (page: Page, at: number): Promise<void> =>
  page.evaluate((offset) => { window.__harness!.focus(); window.__harness!.caretTo(offset); }, at);

/** Places in plain prose: a paragraph, a heading, a quote line, a setext paragraph. */
const PLAIN: Array<[typeof SCRIPT, Anchor]> = [
  [SCRIPT, { find: 'first night', offset: 3 }],
  [SCRIPT, { find: 'Arrival', offset: 2 }],
  [SCRIPT, { find: 'Hum with me', offset: 4 }],
  [BELIEF, { find: 'grandmother', offset: 4 }],
  [BELIEF, { find: 'Synthetic heading level two', offset: 10 }],
  [BELIEF, { find: 'A paragraph under the setext', offset: 5 }],
  [BELIEF, { find: 'a synthetic fenced', offset: 4 }],
  [BELIEF, { find: 'stars', offset: 2 }],
];

test.describe('@harness prosemirror · source offsets and positions', () => {
  for (const [file, anchor] of PLAIN) {
    test(`caretTo and caret agree at ${JSON.stringify(anchor.find)}+${anchor.offset}`, async ({ page }) => {
      const source = await open(page, file);
      const at = resolveAnchor(source, anchor);
      await caretTo(page, at);
      expect(await caret(page)).toBe(at);
    });
  }

  for (const [file, anchor] of PLAIN.filter(([, a]) => a.find !== 'a synthetic fenced' && a.find !== 'stars')) {
    test(`typing at ${JSON.stringify(anchor.find)}+${anchor.offset} lands next to the same characters`, async ({ page }) => {
      const source = await open(page, file);
      const at = resolveAnchor(source, anchor);
      await caretTo(page, at);
      await page.keyboard.type('QQ');
      expect(await text(page)).toContain(`${source.slice(at - 3, at)}QQ${source.slice(at, at + 3)}`);
    });
  }
});

test.describe('@harness prosemirror · front matter', () => {
  test('Write mode keeps the front matter bytes in front of the body, also after select-all and typing', async ({ page }) => {
    const source = await open(page, SCRIPT);
    const { frontMatter } = splitFrontMatter(source);
    expect((await text(page)).startsWith(frontMatter)).toBe(true);
    await page.evaluate(() => window.__harness!.focus());
    await page.keyboard.press('ControlOrMeta+a');
    await page.keyboard.type('replaced');
    expect(await text(page)).toBe(`${frontMatter}replaced`);
  });

  test('Source mode shows the whole file and refuses edits inside the front matter', async ({ page }) => {
    const source = await open(page, SCRIPT);
    const { frontMatter } = splitFrontMatter(source);
    await page.evaluate(() => window.__harness!.setMode('source'));
    const before = await text(page);
    expect(before.startsWith(frontMatter)).toBe(true);
    await caretTo(page, resolveAnchor(source, { find: 'id: S-004', offset: 4 }));
    await page.keyboard.type('zz');
    await page.keyboard.press('Delete');
    await page.keyboard.press('Backspace');
    await caretTo(page, frontMatter.length);
    await page.keyboard.press('Backspace');
    await settle(page);
    expect(await text(page)).toBe(before);
    // The body behind it is editable.
    await caretTo(page, frontMatter.length);
    await page.keyboard.type('ok ');
    expect(await text(page)).toBe(`${frontMatter}ok ${before.slice(frontMatter.length)}`);
  });

  test('Mod-a in Source mode selects the body only', async ({ page }) => {
    const source = await open(page, SCRIPT);
    const { frontMatter } = splitFrontMatter(source);
    await page.evaluate(() => window.__harness!.setMode('source'));
    await page.evaluate(() => window.__harness!.focus());
    await page.keyboard.press('ControlOrMeta+a');
    await page.keyboard.type('only body');
    expect(await text(page)).toBe(`${frontMatter}only body`);
  });

  test('a record with an empty body takes typing right after load', async ({ page }) => {
    const source = await open(page, EMPTY_BODY);
    await page.evaluate(() => window.__harness!.focus());
    await page.keyboard.type('abc');
    expect(await text(page)).toBe(`${source}abc`);
  });

  test('Source mode offsets are the positions of the file', async ({ page }) => {
    await open(page, SCRIPT);
    await page.evaluate(() => window.__harness!.setMode('source'));
    for (const at of [0, 7, 100, 250]) {
      await caretTo(page, at);
      expect(await caret(page)).toBe(at);
    }
  });

  test('Write, Source and Write again keep the saved text', async ({ page }) => {
    await open(page, BELIEF);
    const saved = await text(page);
    await page.evaluate(() => window.__harness!.setMode('source'));
    expect(await text(page)).toBe(saved);
    await page.evaluate(() => window.__harness!.setMode('write'));
    expect(await text(page)).toBe(saved);
  });
});

test.describe('@harness prosemirror · history and external changes', () => {
  test('undo takes back exactly the typed text', async ({ page }) => {
    const source = await open(page, HALL);
    const before = await text(page);
    await caretTo(page, resolveAnchor(source, { find: 'Hearth folk' }));
    await page.keyboard.type('abc');
    expect(await text(page)).not.toBe(before);
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(before);
    await page.keyboard.press('ControlOrMeta+Shift+z');
    expect(await text(page)).toContain('abc');
  });

  test('an external change keeps the caret, and one undo removes only the local edit', async ({ page }) => {
    const source = await open(page, HALL);
    const external = { from: resolveAnchor(source, { find: 'holiest' }), insert: 'EXT ' };
    await page.evaluate(({ from, insert }) => window.__harness!.applyExternal([{ from, to: from, insert }]), external);
    const onlyExternal = await text(page);

    await open(page, HALL);
    const local = resolveAnchor(source, { find: 'Hearth folk' });
    await caretTo(page, local);
    await page.keyboard.insertText('LOCAL ');
    const at = external.from + 'LOCAL '.length;
    await page.evaluate(({ from, insert }) => window.__harness!.applyExternal([{ from, to: from, insert }]), { from: at, insert: 'EXT ' });
    expect(await text(page)).toContain('LOCAL ');
    expect(await text(page)).toContain('EXT holiest');
    expect(await caret(page)).toBe(local + 'LOCAL '.length);
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(onlyExternal);
  });

  test('markSaved empties the history without touching the text', async ({ page }) => {
    const source = await open(page, HALL);
    await caretTo(page, resolveAnchor(source, { find: 'Hearth folk' }));
    await page.keyboard.type('abc');
    const typed = await text(page);
    await page.evaluate(() => window.__harness!.markSaved());
    expect(await page.evaluate(() => window.__harness!.canUndo())).toBe(false);
    expect(await text(page)).toBe(typed);
    expect(await caret(page)).toBeGreaterThan(0);
  });
});

test.describe('@harness prosemirror · live preview', () => {
  test('emphasis markers show raw under the caret and are rendered elsewhere', async ({ page }) => {
    const source = await open(page, BELIEF);
    await caretTo(page, resolveAnchor(source, { find: 'stars', offset: 2 }));
    await settle(page);
    expect(await visibleText(page)).toContain('*stars*');
    await caretTo(page, resolveAnchor(source, { find: 'grandmother', offset: 2 }));
    await settle(page);
    const shown = await visibleText(page);
    expect(shown).not.toContain('*stars*');
    expect(shown).toContain('stars');
  });

  test('heading hashes show raw only on the heading under the caret', async ({ page }) => {
    const source = await open(page, BELIEF);
    await caretTo(page, resolveAnchor(source, { find: 'Synthetic heading level two', offset: 3 }));
    await settle(page);
    const raw = await visibleText(page);
    expect(raw).toContain('## Synthetic heading level two');
    expect(raw).not.toContain('# Synthetic heading level one');
    expect(raw).not.toContain('### Synthetic heading level three');
  });
});

test.describe('@harness prosemirror · input methods', () => {
  const VOICES = corpusFile('records/lore/topics/synthetic-voices-in-world.md');

  for (const [name, file, anchor] of [
    ['prose', SCRIPT, { find: 'first night', offset: 3 }],
    ['voice', VOICES, { find: 'only made noise' }],
  ] as const) {
    test(`a Japanese composition in ${name} lands next to the same characters`, async ({ page, browserName }) => {
      test.skip(browserName !== 'chromium', 'composition goes through a CDP session');
      const source = await open(page, file);
      const at = resolveAnchor(source, anchor);
      await caretTo(page, at);
      const client = await page.context().newCDPSession(page);
      for (const step of ['に', 'にほ', 'にほん']) {
        await client.send('Input.imeSetComposition', { text: step, selectionStart: step.length, selectionEnd: step.length });
      }
      await client.send('Input.insertText', { text: '日本語' });
      await client.detach();
      await settle(page);
      expect(await text(page)).toContain(`${source.slice(at - 3, at)}日本語${source.slice(at, at + 3)}`);
    });
  }
});
