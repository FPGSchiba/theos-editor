import { expect, test, type Page } from '@playwright/test';
import { parseBody } from '../../src/adapters/prosemirror/markdown.ts';
import { schema } from '../../src/adapters/prosemirror/schema.ts';
import { resolveAnchor } from '../../src/harness/anchor.ts';
import { splitFrontMatter } from '../../src/harness/frontmatter.ts';
import { toLf } from '../../src/harness/oracle.ts';
import { corpusFile, settle, visibleText } from './checks.ts';
import { loadFile } from './helpers.ts';

// Probes of the ProseMirror best-effort column for what the measured suites cannot see: that
// typing anywhere in plain text is a span edit, that a change without a span mapping rewrites its
// block only and undo brings the bytes back, that offsets and Source mode refer to the saved bytes,
// and that `lastSaveLevel()` says which happened. Each probe loads `prosemirror:best-effort`
// itself, whatever P10_VARIANTS says, and none of them changes a shared suite.

const VARIANT = 'prosemirror:best-effort';
type File = ReturnType<typeof corpusFile>;
const BELIEF = corpusFile('records/lore/beliefs/B-901.md');
const TRUTH = corpusFile('records/lore/truths/T-901.md');
const VOICES = corpusFile('records/lore/topics/synthetic-voices-in-world.md');
const NOTES = corpusFile('records/maps/synthetic-notes.md');
const SCRIPT = corpusFile('records/story/scripts/S-901.md');
const WHITESPACE = corpusFile('records/things/synthetic-whitespace.md');
const HALL = corpusFile('records/world/hearth/f04-choir-hall.md');

const open = async (page: Page, file: File): Promise<string> => toLf(await loadFile(page, VARIANT, file));
const text = (page: Page): Promise<string> => page.evaluate(() => window.__harness!.text());
const level = (page: Page): Promise<string | undefined> => page.evaluate(() => window.__harness!.lastSaveLevel());
const caret = (page: Page): Promise<number> => page.evaluate(() => window.__harness!.caret());
const caretTo = (page: Page, at: number): Promise<void> =>
  page.evaluate((offset) => { window.__harness!.focus(); window.__harness!.caretTo(offset); }, at);
const undoAll = async (page: Page): Promise<void> => {
  for (let n = 0; n < 20 && (await page.evaluate(() => window.__harness!.canUndo())); n++) await page.keyboard.press('ControlOrMeta+z');
};

/**
 * Offsets in the middle of plain words of the body (every third word): not inside link syntax,
 * an autolink or a Markdown link, where a keystroke would change a link target, not in what makes
 * a construct (a callout's `[!type]`, the `Beat <n>` of a beat heading: a letter there changes what
 * the line is, see the fallback probes) and not in a fence's info string, which is no text.
 */
function wordMiddles(source: string): number[] {
  const body = splitFrontMatter(source).frontMatter.length;
  const skip: Array<[number, number]> = [];
  for (const found of source.matchAll(/\[\[[^\]]*\]\]|<[^>\n]*>|\[[^\]\n]*\]\([^)\n]*\)|\[![a-z]+\]|^## Beat \d+|^```.*$/gm)) skip.push([found.index, found.index + found[0].length]);
  const offsets: number[] = [];
  let word = 0;
  for (const found of source.slice(body).matchAll(/[A-Za-z]{4,}/g)) {
    const at = body + found.index + 2;
    if (skip.some(([from, to]) => at >= from && at < to)) continue;
    if (word++ % 3 === 0) offsets.push(at);
  }
  return offsets;
}

test.describe('@harness prosemirror:best-effort · span edits', () => {
  for (const file of [BELIEF, TRUTH, VOICES, NOTES, SCRIPT, WHITESPACE]) {
    test(`a keystroke in a word of ${file.path} changes that spot only, and undo restores the bytes`, async ({ page }) => {
      test.setTimeout(120_000);
      const source = await open(page, file);
      expect(await text(page)).toBe(source);
      const places = wordMiddles(source);
      expect(places.length).toBeGreaterThan(5);
      // A lower-case letter: an upper-case one in an attribute key would make the line no attribute line (next describe).
      for (const at of places) {
        await caretTo(page, at);
        await page.keyboard.type('q');
        expect(await text(page), `typing at ${at}`).toBe(`${source.slice(0, at)}q${source.slice(at)}`);
        expect(await level(page), `level at ${at}`).toBe('span');
        await page.keyboard.press('ControlOrMeta+z');
        expect(await text(page), `undo at ${at}`).toBe(source);
      }
    });
  }

  test('Enter and Backspace in a top-level paragraph split and join it as span edits', async ({ page }) => {
    const source = await open(page, HALL);
    const at = resolveAnchor(source, { find: 'resonating' });
    await caretTo(page, at);
    await page.keyboard.press('Enter');
    expect(await text(page)).toBe(`${source.slice(0, at)}\n\n${source.slice(at)}`);
    expect(await level(page)).toBe('span');
    await page.keyboard.press('Backspace');
    expect(await text(page)).toBe(source);
  });

  test('keyboard select-all and typing replace the body only', async ({ page }) => {
    const source = await open(page, BELIEF);
    await page.evaluate(() => window.__harness!.focus());
    await page.keyboard.press('ControlOrMeta+a');
    await page.keyboard.type('New body.');
    expect(await text(page)).toBe(`${splitFrontMatter(source).frontMatter}New body.`);
    await undoAll(page);
    expect(await text(page)).toBe(source);
  });
});

test.describe('@harness prosemirror:best-effort · fallback per block', () => {
  test('an attribute key that stops being one is written by the serializer; the bytes change only there', async ({ page }) => {
    const source = await open(page, SCRIPT);
    const at = resolveAnchor(source, { find: 'says::', offset: 2 });
    await caretTo(page, at);
    await page.keyboard.type('Q');
    expect(await text(page)).toBe(`${source.slice(0, at)}Q${source.slice(at)}`);
    expect(await level(page)).toBe('block');
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(source);
    expect(await level(page)).toBe('span');
  });

  test('Enter inside an item of a tight list rewrites that list only; undo brings its bytes and the span level back', async ({ page }) => {
    const source = await open(page, TRUTH);
    // No Markdown holds a tight list whose item has two paragraphs, so the list is written again.
    const start = source.indexOf('- first dash item');
    const end = source.indexOf('\n\n* first star item');
    await caretTo(page, resolveAnchor(source, { find: 'dash item', occurrence: 2 }));
    await page.keyboard.press('Enter');
    const saved = await text(page);
    expect(await level(page)).toBe('block');
    // Everything in front of the list and behind it keeps its bytes.
    expect(saved.startsWith(source.slice(0, start))).toBe(true);
    expect(saved.endsWith(source.slice(end))).toBe(true);
    expect(saved).not.toBe(source);
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(source);
    expect(await level(page)).toBe('span');
  });
});

/** The editor's document as the view holds it, as JSON (prosemirror-view keeps its view description on the root element). */
const editorDoc = (page: Page): Promise<unknown> =>
  page.evaluate(() => (document.querySelector('#editor-host .ProseMirror') as unknown as { pmViewDesc: { node: { toJSON(): unknown } } }).pmViewDesc.node.toJSON());

/** Whether the saved body parses to the editor's document. */
async function reparses(page: Page, saved: string): Promise<boolean> {
  return schema.nodeFromJSON(await editorDoc(page)).eq(parseBody(splitFrontMatter(saved).body));
}

test.describe('@harness prosemirror:best-effort · Enter at the end of a paragraph', () => {
  // Mid-file (a paragraph followed by a heading) and on the last block (the file ends with a newline);
  // with and without a save (`text()`) between Enter, which makes an empty paragraph no Markdown holds, and typing.
  const cases = [
    { name: 'mid-file', file: BELIEF, end: 'A paragraph under the first heading.' },
    { name: 'on the last block', file: HALL, end: 'their holiest place.' },
  ];
  for (const { name, file, end } of cases) {
    for (const query of [false, true]) {
      test(`${name}, ${query ? 'with' : 'without'} a save before typing: a new paragraph, span edits, and the text parses to the document`, async ({ page }) => {
        const source = await open(page, file);
        const at = resolveAnchor(source, { find: end, offset: end.length });
        await caretTo(page, at);
        await page.keyboard.press('Enter');
        if (query) {
          // The empty paragraph owns a blank line between two blank lines.
          expect(await text(page)).toBe(`${source.slice(0, at)}\n\n${source.slice(at)}`);
          expect(await level(page)).toBe('span');
        }
        await page.keyboard.type('New one.');
        const saved = await text(page);
        expect(saved).toBe(`${source.slice(0, at)}\n\nNew one.${source.slice(at)}`);
        expect(await level(page)).toBe('span');
        expect(await reparses(page, saved)).toBe(true);
      });
    }
  }
});

/** Load a text of its own (no front matter) into the best-effort variant. */
async function openText(page: Page, content: string): Promise<void> {
  if (page.url() === 'about:blank') await page.goto('/');
  await page.waitForFunction(() => window.__harness !== undefined);
  await page.evaluate(([variant, body]) => window.__harness!.load(variant, body, 'records/probe.md'), [VARIANT, content] as const);
}

test.describe('@harness prosemirror:best-effort · the level of joined and split rewritten blocks', () => {
  test('a block joined to a rewritten one still reports block: the joined text holds serializer bytes', async ({ page }) => {
    const body = 'Prev line.\n\nWords _under_ tail.\n';
    await openText(page, body);
    // A letter in front of an `_` run inside a word: the paragraph is written again (`*under*`).
    await caretTo(page, body.indexOf('_under_'));
    await page.keyboard.type('Y');
    expect(await text(page)).toBe('Prev line.\n\nWords Y*under* tail.\n');
    expect(await level(page)).toBe('block');
    await caretTo(page, (await text(page)).indexOf('Words'));
    await page.keyboard.press('Backspace');
    expect(await text(page)).toBe('Prev line.Words Y*under* tail.\n');
    expect(await level(page)).toBe('block');
  });

  test('the half of a split rewritten block still reports block when the other half is deleted', async ({ page }) => {
    const body = 'Lead _a_ first. Then _more_ end.\n';
    await openText(page, body);
    await caretTo(page, body.indexOf('_a_'));
    await page.keyboard.type('Y');
    expect(await text(page)).toBe('Lead Y*a* first. Then *more* end.\n');
    await caretTo(page, (await text(page)).indexOf('Then'));
    await page.keyboard.press('Enter');
    expect(await level(page)).toBe('block');
    const split = await text(page);
    // The first half goes, from its start to the start of the second half.
    await page.evaluate((length) => window.__harness!.apply({ id: 'cut', kind: 'delete', at: { find: 'Lead' }, length }), split.indexOf('Then'));
    expect(await text(page)).toBe('Then *more* end.\n');
    expect(await level(page)).toBe('block');
  });
});

test.describe('@harness prosemirror:best-effort · paragraph breaks in a callout', () => {
  test('Enter inside a voice line is a blank quote line, and Backspace joins the lines again', async ({ page }) => {
    const source = await open(page, VOICES);
    const at = resolveAnchor(source, { find: 'the way it was' });
    await caretTo(page, at);
    await page.keyboard.press('Enter');
    expect(await text(page)).toBe(`${source.slice(0, at)}\n>\n> ${source.slice(at)}`);
    expect(await level(page)).toBe('span');
    await page.keyboard.press('Backspace');
    expect(await text(page)).toBe(source);
  });
});

/** The node types of the children of the first callout: in the editor, and in the saved text parsed again. */
async function calloutChildren(page: Page, saved: string): Promise<{ editor: string[]; reloaded: string[] }> {
  const types = (doc: { firstChild: { forEach(f: (child: { type: { name: string } }) => void): void } | null }): string[] => {
    const names: string[] = [];
    doc.firstChild?.forEach((child) => names.push(child.type.name));
    return names;
  };
  return { editor: types(schema.nodeFromJSON(await editorDoc(page))), reloaded: types(parseBody(splitFrontMatter(saved).body)) };
}

test.describe('@harness prosemirror:best-effort · Enter at the end of a callout (variant M2)', () => {
  // Stock `splitBlock` makes a `callout_caption` behind the last paragraph of a callout, so typing goes
  // into a caption, and the serializer writes the whole callout again: the line break inside its
  // paragraph, which the edit did not touch, becomes a space. Design has no caption in the data model,
  // and its parser has none, so the saved caption reloads as a paragraph.
  const cases = [
    { type: 'voice', head: '> [!voice] [[throat-kin]]', text: 'A voice line', reloads: true },
    { type: 'design', head: '> [!design]', text: 'A designer remark', reloads: false },
  ];
  for (const { type, head, text: words, reloads } of cases) {
    test(`${type}: Enter at the end of the last paragraph and typing write the callout again, at block level`, async ({ page }) => {
      const body = `${head}\n> ${words}\n> on two lines.\n\nAfter.\n`;
      await openText(page, body);
      await caretTo(page, body.indexOf('on two lines.') + 'on two lines.'.length);
      await page.keyboard.press('Enter');
      await page.keyboard.type('New one.');
      const saved = await text(page);
      expect(saved).toBe(`${head}\n> ${words} on two lines.\n>\n> — New one.\n\nAfter.\n`);
      expect(await level(page)).toBe('block');
      const children = await calloutChildren(page, saved);
      expect(children.editor).toEqual(['callout_title', 'paragraph', 'callout_caption']);
      expect(await reparses(page, saved)).toBe(reloads);
      expect(children.reloaded).toEqual(reloads ? children.editor : ['callout_title', 'paragraph', 'paragraph']);
    });
  }
});

test.describe('@harness prosemirror:best-effort · offsets, modes and beats', () => {
  test('offsets refer to the saved bytes: caretTo and caret agree after edits', async ({ page }) => {
    const source = await open(page, BELIEF);
    await caretTo(page, resolveAnchor(source, { find: 'grandmother' }));
    await page.keyboard.type('long ');
    const saved = await text(page);
    for (const find of ['underscores_,', 'setext heading.', 'literal stars', 'a synthetic fenced', 'continues on the one']) {
      const at = saved.indexOf(find) + 2;
      await caretTo(page, at);
      expect(await caret(page), find).toBe(at);
    }
  });

  test('Source mode shows the saved bytes after an edit, and Write mode keeps them', async ({ page }) => {
    const source = await open(page, BELIEF);
    await caretTo(page, resolveAnchor(source, { find: 'grandmother' }));
    await page.keyboard.type('old ');
    const saved = await text(page);
    expect(saved).toBe(source.replace('grandmother', 'old grandmother'));
    await page.evaluate(() => window.__harness!.setMode('source'));
    await settle(page);
    expect(await text(page)).toBe(saved);
    expect(await visibleText(page)).toContain('_underscores_');
    await page.evaluate(() => window.__harness!.setMode('write'));
    expect(await text(page)).toBe(saved);
  });

  test('the first attribute line of a bare beat gets its newline, and offsets behind it stay right', async ({ page }) => {
    const source = await open(page, SCRIPT);
    await page.evaluate(() => window.__harness!.apply({ id: 'b', kind: 'beat-param', beat: 3, key: 'at', value: '[[fates-shrine]]' }));
    const saved = await text(page);
    expect(saved).toBe(source.replace('## Beat 3 — The murmur\n', '## Beat 3 — The murmur\nat:: [[fates-shrine]]\n'));
    const at = saved.indexOf('drifts down');
    await caretTo(page, at);
    expect(await caret(page)).toBe(at);
    await page.keyboard.type('Q');
    expect(await text(page)).toBe(`${saved.slice(0, at)}Q${saved.slice(at)}`);
  });
});
