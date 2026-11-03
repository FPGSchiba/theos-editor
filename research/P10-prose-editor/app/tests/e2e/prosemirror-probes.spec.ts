import { expect, test, type Page } from '@playwright/test';
import { defaultMarkdownParser } from 'prosemirror-markdown';
import { parseBody } from '../../src/adapters/prosemirror/markdown.ts';
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

// ── Links (Task 13) ──────────────────────────────────────────────────────────

async function openText(page: Page, content: string): Promise<void> {
  if (page.url() === 'about:blank') await page.goto('/');
  await page.waitForFunction(() => window.__harness !== undefined);
  await page.evaluate((body) => window.__harness!.load('prosemirror:stock', body, 'records/probe.md'), content);
}

const WIKILINK = /\[\[[^\]\n]+\]\]/g;

test.describe('@harness prosemirror · links', () => {
  const sample = 'Go to [[fates-shrine]] now, [[mother-ilse]][[B-020|a song]] end.\n\n`[[fates-shrine]]` stays code.\n\n```\n[[mother-ilse]]\n```\n\nlast';
  const chips = '#editor-host .tu-link';
  const popover = '.tu-pop[data-link-popover]';
  const focus = (page: Page): Promise<void> => page.evaluate(() => window.__harness!.focus());

  test('the link node writes [[id]] and [[id|shown]] back exactly; links in code stay raw', async ({ page }) => {
    await openText(page, sample);
    await settle(page);
    expect(await page.locator(chips).evaluateAll((all) => all.map((chip) => chip.getAttribute('data-link-id')))).toEqual(
      ['fates-shrine', 'mother-ilse', 'B-020'],
    );
    expect(await text(page)).toBe(sample);
  });

  test('every link of the links file survives the serializer, alias included', async ({ page }) => {
    const source = toLf(await loadFile(page, VARIANT, corpusFile('records/lore/topics/synthetic-links-explaining.md')));
    expect((await text(page)).match(WIKILINK)).toEqual(source.match(WIKILINK));
  });

  test('links in beat attribute lines are chips; the callout title keeps its links as raw text (the head draws them)', async ({ page }) => {
    const script = toLf(await loadFile(page, VARIANT, SCRIPT));
    await settle(page);
    // Chips are atoms in the document: every attribute line keeps its. A callout title is raw text of the callout node.
    const titles = script.split('\n').filter((line) => line.startsWith('> [!'));
    const inTitles = titles.join('\n').match(WIKILINK)?.length ?? 0;
    expect(await page.locator(chips).count()).toBe(script.match(WIKILINK)!.length - inTitles);
    expect((await text(page)).match(WIKILINK)).toEqual(script.match(WIKILINK));
  });

  test('the caret steps over a chip both ways; Backspace and Delete take it whole', async ({ page }) => {
    await openText(page, sample);
    const link = '[[mother-ilse]]';
    const alias = '[[B-020|a song]]';
    const start = sample.indexOf(link);
    await caretTo(page, start);
    await focus(page);
    await page.keyboard.press('ArrowRight');
    expect(await caret(page)).toBe(start + link.length);
    await page.keyboard.press('ArrowRight');
    expect(await caret(page)).toBe(start + link.length + alias.length);
    await page.keyboard.press('ArrowLeft');
    expect(await caret(page)).toBe(start + link.length);
    await page.keyboard.press('Backspace');
    expect(await text(page)).toBe(sample.replace(link, ''));
    await page.keyboard.press('Delete');
    expect(await text(page)).toBe(sample.replace(link, '').replace(alias, ''));
    expect(await page.locator(chips).count()).toBe(1);
  });

  test('a link typed by hand is raw while typed and a chip when the ]] is typed', async ({ page }) => {
    await openText(page, sample);
    await caretTo(page, sample.length);
    await focus(page);
    await page.keyboard.type(' [[mother-ilse');
    expect(await page.locator(chips).count()).toBe(3);
    expect(await visibleText(page)).toContain('[[mother-ilse');
    await page.keyboard.type(']]');
    await expect(page.locator(chips)).toHaveCount(4);
    await page.keyboard.type('!');
    expect((await text(page)).endsWith('last [[mother-ilse]]!')).toBe(true);
  });

  test('the popover offers options, Enter picks, and the typed [[query becomes the link', async ({ page }) => {
    await openText(page, sample);
    await caretTo(page, sample.length);
    await focus(page);
    await page.keyboard.type(' [[Listen');
    await expect(page.locator(popover)).toHaveCount(1);
    const rows = await page.locator(`${popover} .tu-pop__row`).evaluateAll((all) => all.map((row) => row.getAttribute('data-link-option')));
    expect(rows.length).toBeGreaterThan(1);
    await page.keyboard.press('ArrowDown');
    await expect(page.locator(`${popover} .tu-pop__row.is-active`)).toHaveAttribute('data-link-option', rows[1]!);
    await page.keyboard.press('Enter');
    await expect(page.locator(popover)).toHaveCount(0);
    expect(await text(page)).toBe(`${sample} [[${rows[1]}]]`);
    await page.keyboard.type('!');
    expect((await text(page)).endsWith(`[[${rows[1]}]]!`)).toBe(true);
  });

  test('Escape keeps the typed text plain, with no popover for what follows', async ({ page }) => {
    await openText(page, sample);
    await caretTo(page, sample.length);
    await focus(page);
    await page.keyboard.type(' [[son');
    await expect(page.locator(popover)).toHaveCount(1);
    await page.keyboard.press('Escape');
    await expect(page.locator(popover)).toHaveCount(0);
    await page.keyboard.type('g');
    await expect(page.locator(popover)).toHaveCount(0);
    expect(await visibleText(page)).toContain('[[song');
    // The text stays text; the stock serializer escapes the brackets when it writes it (a measurement).
    expect(await text(page)).toContain(String.raw`last \[\[song`);
    await page.keyboard.type(' [[mo');
    await expect(page.locator(popover)).toHaveCount(1);
  });

  test('the popover closes on blur and sits outside the editor DOM', async ({ page }) => {
    await openText(page, sample);
    await caretTo(page, sample.length);
    await focus(page);
    await page.keyboard.type(' [[mo');
    await expect(page.locator(popover)).toHaveCount(1);
    expect(await page.locator(`#editor-host ${popover}`).count()).toBe(0);
    await page.evaluate(() => (document.activeElement as HTMLElement).blur());
    await expect(page.locator(popover)).toHaveCount(0);
  });

  test('chips keep their DOM across typing', async ({ page }) => {
    await openText(page, sample);
    await settle(page);
    await page.evaluate(() => document.querySelectorAll('#editor-host .tu-link').forEach((chip) => Object.assign(chip, { p10: true })));
    await caretTo(page, sample.length);
    await focus(page);
    await page.keyboard.type(' more');
    expect(await page.locator(chips).evaluateAll((all) => all.every((chip) => (chip as HTMLElement & { p10?: boolean }).p10 === true))).toBe(true);
  });

  test('Source mode shows the links raw, Write mode draws the chips again', async ({ page }) => {
    await openText(page, sample);
    await page.evaluate(() => window.__harness!.setMode('source'));
    await settle(page);
    expect(await page.locator(chips).count()).toBe(0);
    await page.evaluate(() => window.__harness!.setMode('write'));
    await expect(page.locator(chips)).toHaveCount(3);
  });

  test('the stock defaultMarkdownParser is not changed by the adapter: it still parses [[x]]', () => {
    expect(parseBody('a [[x]] b').firstChild!.childCount).toBe(3);
    expect(() => defaultMarkdownParser.parse('a [[x]] b')).not.toThrow();
    expect(defaultMarkdownParser.parse('a [[x]] b').textContent).toBe('a [[x]] b');
  });

  test('the place after a hard break reads the key of the new line', async ({ page }) => {
    const beat = 'at:: [[fates-shrine]]  \nwith:: ';
    await openText(page, beat);
    await caretTo(page, beat.length);
    await focus(page);
    await page.keyboard.type('[[mo');
    await expect(page.locator(popover)).toHaveCount(1);
    await expect(page.locator(`${popover} .tu-pop__head`)).toContainText('with:: only');
  });

  test('link-insert and link-delete through apply act on the right place', async ({ page }) => {
    await openText(page, sample);
    await page.evaluate(() => window.__harness!.apply({ id: 'i', kind: 'link-insert', at: { find: ' now' }, query: 'ilse', pick: 'mother-ilse' }));
    expect((await text(page)).startsWith('Go to [[fates-shrine]][[mother-ilse]] now')).toBe(true);
    await page.evaluate(() => window.__harness!.apply({ id: 'd', kind: 'link-delete', link: { find: '[[B-020' } }));
    expect(await text(page)).not.toContain('[[B-020');
    expect(await page.locator(chips).count()).toBe(3);
  });
});

test.describe('@harness prosemirror · callouts', () => {
  const VOICES_FILE = corpusFile('records/lore/topics/synthetic-voices-in-world.md');
  const NOTES_FILE = corpusFile('records/maps/synthetic-notes.md');

  test('the callout node alone loses nothing: titles, "> " lines, empty ">" lines and captions are written back', async ({ page }) => {
    await open(page, VOICES_FILE);
    const saved = await text(page);
    for (const line of [
      '> [!voice] [[throat-kin]] · [[B-020]] · T2',
      '> — a binder, teaching the naming-song',
      '> — a saying on the shrine stair',
      '> — a real caption after it',
      '> [!design]',
      '> A synthetic designer remark inside an in-world text.',
      '> [!voice] [[mother-ilse]] · [[B-016]] · T1',
    ]) expect(saved.split('\n'), line).toContain(line);
    // The caption that ended a paragraph sits directly under the line before it; the other keeps its empty line.
    expect(saved).toContain('empty hall.\n> — a binder');
    expect(saved).toContain('>\n> — a saying');
    // The stock serializer is what turns the "--" line into "\--" and joins soft lines.
    expect(saved).toContain('> \\-- a line that starts with two hyphens is not a caption.\n> — a real caption after it');
  });

  test('every note callout keeps its title through the serializer', async ({ page }) => {
    const source = await open(page, NOTES_FILE);
    const saved = await text(page);
    const titles = source.split('\n').filter((line) => line.startsWith('> [!note]'));
    expect(titles.length).toBe(5);
    for (const title of titles) expect(saved.split('\n')).toContain(title);
    // The caption line of a note stays a line of its own (data model 9.3).
    expect(saved).toContain('> [!note] n1 · open\n> A note that is still open.\n> — Jann\n');
  });

  test('a paste with line breaks into a callout title stays one line, and saves as one line', async ({ page }) => {
    await open(page, VOICES_FILE);
    await page.evaluate(() => window.__harness!.apply({ id: 'p', kind: 'paste', at: { find: 'T2' }, plain: 'a\nb\r\nc' }));
    expect(await text(page)).toContain('> [!voice] [[throat-kin]] · [[B-020]] · a b cT2\n');
    expect((await text(page)).split('\n').filter((line) => line === 'b' || line === 'c')).toEqual([]);
  });

  test('the title shows raw while the caret is in it, the head otherwise', async ({ page }) => {
    const source = await open(page, VOICES_FILE);
    const title = '.pm-callout-title.is-raw';
    await expect(page.locator(title)).toHaveCount(0);
    await caretTo(page, resolveAnchor(source, { find: '[[throat-kin]] · [[B-020]]', offset: 4 }));
    await expect(page.locator(title)).toHaveCount(1);
    await expect(page.locator(title)).toHaveText('[[throat-kin]] · [[B-020]] · T2');
    expect(await caret(page)).toBe(resolveAnchor(source, { find: '[[throat-kin]] · [[B-020]]', offset: 4 }));
    await caretTo(page, resolveAnchor(source, { find: 'Sing a name', offset: 3 }));
    await expect(page.locator(title)).toHaveCount(0);
  });

  test('note-state through apply is its own undo step, keeps the head in step and the other notes alone', async ({ page }) => {
    const source = await open(page, NOTES_FILE);
    await page.evaluate(() => window.__harness!.apply({ id: 'x', kind: 'insert', at: { find: 'A note that was dropped.' }, text: 'Q' }));
    await page.evaluate(() => window.__harness!.apply({ id: 's', kind: 'note-state', note: 'n2', state: 'dropped' }));
    const after = await text(page);
    expect(after).toContain('> [!note] n2 · [[f04-terrain-v1]] · dropped');
    expect(after).toContain('> [!note] n1 · open');
    await expect(page.locator('[data-callout="note"][data-note="n2"]')).toHaveAttribute('data-note-state', 'dropped');
    await page.evaluate(() => window.__harness!.undo());
    const undone = await text(page);
    expect(undone).toContain('> [!note] n2 · [[f04-terrain-v1]] · open');
    expect(undone).toContain('QA note that was dropped.');
    expect(source).toContain('n2 · [[f04-terrain-v1]] · open');
  });

  test('developer words are flagged in voice text only, and a typed one is flagged too', async ({ page }) => {
    await open(page, VOICES_FILE);
    const words = page.locator('[data-dev-word]');
    await expect(words).toHaveCount(1);
    await expect(words.first()).toHaveText('ledger');
    expect(await words.first().evaluate((node) => node.closest('[data-callout="voice"]') !== null)).toBe(true);
    await page.evaluate(() => window.__harness!.apply({ id: 'w', kind: 'insert', at: { find: 'only made noise' }, text: 'ledger ' }));
    await expect(words).toHaveCount(2);
  });

  test('the heads are mounted once per callout and redrawn when the index says something else', async ({ page }) => {
    await open(page, VOICES_FILE);
    await expect(page.locator('.tu-voice__head')).toHaveCount(6);
    await page.evaluate(() => window.__harness!.apply({ id: 'x', kind: 'insert', at: { find: 'A synthetic voice line' }, text: 'Z' }));
    await expect(page.locator('.tu-voice__head')).toHaveCount(6);
  });
});

test.describe('@harness prosemirror · typing in callouts', () => {
  test('typing in voice text and in the raw title lands next to the same characters', async ({ page }) => {
    const source = await open(page, corpusFile('records/lore/topics/synthetic-voices-in-world.md'));
    await caretTo(page, resolveAnchor(source, { find: 'only made noise' }));
    await page.keyboard.type('QQ');
    expect(await text(page)).toContain('you have QQonly made noise');
    await caretTo(page, resolveAnchor(source, { find: 'T2' }));
    await page.keyboard.type('x');
    expect(await text(page)).toContain('> [!voice] [[throat-kin]] · [[B-020]] · xT2\n');
    await expect(page.locator('[data-callout="voice"]').nth(1)).toHaveClass(/has-raw-title/);
  });
});

test.describe('@harness prosemirror · beats', () => {
  const sample = [
    'Intro.',
    '',
    '## Beat 1 — Arrival',
    'at::   [[fates-shrine]]',
    'when:: T1',
    '',
    'Prose after the beat.',
    '',
    '## Beat 2 — Bare',
    '',
    'More prose.',
    '',
    '## Beat 3 — Tight',
    'says:: [[B-016]]',
    '> [!design]',
    '> Directly under.',
    '',
    '## Beat 4 — Rest',
    'with:: [[mother-ilse]]',
    'Prose on the next line.',
  ].join('\n');
  const openText = async (page: Page, content: string): Promise<void> => {
    if (page.url() === 'about:blank') await page.goto('/');
    await page.waitForFunction(() => window.__harness !== undefined);
    await page.evaluate((body) => window.__harness!.load('prosemirror:stock', body, 'records/probe.md'), content);
  };
  const at = (page: Page, offset: number): Promise<void> =>
    page.evaluate((to) => { window.__harness!.focus(); window.__harness!.caretTo(to); }, offset);
  const row1 = '[data-beat-row="1"]';

  for (const path of ['records/story/scripts/S-004.md', 'records/story/scripts/S-901.md']) {
    test(`every beat of ${path} keeps its heading and attribute lines exactly`, async ({ page }) => {
      const source = await open(page, corpusFile(path));
      const saved = await text(page);
      const blocks = source.match(/^## Beat \d+\b.*\n(?:[a-z]+::.*\n?)*/gm) ?? [];
      expect(blocks.length).toBeGreaterThan(0);
      for (const block of blocks) expect(saved).toContain(block.replace(/\n$/, ''));
    });
  }

  test('the beat block of a sample survives, tight and not tight', async ({ page }) => {
    await openText(page, sample);
    // Only the stock effects are left: the blank line before "Prose on the next line" is the rest-of-paragraph rule (tight).
    expect(await text(page)).toBe(sample);
  });

  test('the heading and the lines show raw under the caret and rendered elsewhere', async ({ page }) => {
    await openText(page, sample);
    await at(page, 0);
    await settle(page);
    await expect(page.locator('[data-beat="1"]')).toHaveText('1 · Arrival');
    await expect(page.locator(row1)).toBeVisible();
    await at(page, sample.indexOf('when::'));
    await settle(page);
    await expect(page.locator(row1)).toBeHidden();
    expect(await visibleText(page)).toContain('at::   [[fates-shrine]]');
    await at(page, sample.indexOf('## Beat 1') + 3);
    await settle(page);
    await expect(page.locator('[data-beat="1"]')).toBeHidden();
    expect(await visibleText(page)).toContain('## Beat 1 — Arrival');
  });

  test('arrow keys and Backspace step into a hidden beat block', async ({ page }) => {
    await openText(page, sample);
    await at(page, sample.indexOf('\nProse after') + 1);
    await page.keyboard.press('ArrowLeft');
    expect(await caret(page)).toBe(sample.indexOf('when:: T1') + 'when:: T1'.length);
    await page.keyboard.press('Backspace');
    expect(await text(page)).toContain('when:: T\n');
  });

  test('typing in the row never reaches the editor and the edit changes one line', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await openText(page, sample);
    await at(page, 0);
    await page.locator(`${row1} [data-beat-param="at"] .tu-beat__key`).click();
    await page.locator(`${row1} [data-beat-input]`).fill('x');
    await page.keyboard.type('yz');
    expect(await text(page)).toBe(sample);
    await page.locator(`${row1} [data-beat-input]`).press('Enter');
    expect(await text(page)).toBe(sample.replace('at::   [[fates-shrine]]', 'at::   xyz'));
    expect(errors).toEqual([]);
  });

  test('"+ add" on a bare beat and in the order puts the line where the oracle does', async ({ page }) => {
    await openText(page, sample);
    await at(page, 0);
    const row = page.locator('[data-beat-row="2"]');
    await row.locator('[data-beat-add]').click();
    await row.locator('[data-beat-add-key="says"]').click();
    await row.locator('[data-beat-input]').fill('[[B-016]]');
    await row.locator('[data-beat-input]').press('Enter');
    expect(await text(page)).toBe(sample.replace('## Beat 2 — Bare\n', '## Beat 2 — Bare\nsays:: [[B-016]]\n'));
    await page.evaluate(() => window.__harness!.apply({ id: 'o', kind: 'beat-param', beat: 1, key: 'with', value: '[[mother-ilse]]' }));
    expect(await text(page)).toContain('at::   [[fates-shrine]]\nwith:: [[mother-ilse]]\nwhen:: T1\n');
    await page.evaluate(() => window.__harness!.apply({ id: 'p', kind: 'beat-param', beat: 4, key: 'at', value: 'X' }));
    expect(await text(page)).toContain('## Beat 4 — Rest\nat:: X\nwith:: [[mother-ilse]]\n');
  });

  test('every beat has one row, and the rows have spellcheck off', async ({ page }) => {
    await openText(page, sample);
    await expect(page.locator('[data-beat-row]')).toHaveCount(4);
    await expect(page.locator('[data-beat-row] [data-beat-param]')).toHaveCount(4);
    expect(await page.locator('[data-beat-row]').first().evaluate((node) => (node as HTMLElement).spellcheck)).toBe(false);
  });
});
