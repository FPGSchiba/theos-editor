import { test, type Page } from '@playwright/test';
import { resolveAnchor } from '../../src/harness/anchor.ts';
import { compareBytes } from '../../src/harness/compare.ts';
import type { CorpusFile } from '../../src/harness/manifest.ts';
import { editOffset, expectedAfter, toLf } from '../../src/harness/oracle.ts';
import type { Anchor } from '../../src/harness/types.ts';
import { caretAt, corpusFile, judge, runCheck, savedText, settle, type Verdict } from './checks.ts';
import { loadFile, variantsUnderTest } from './helpers.ts';

// Spell check and input methods (CE-06.1 criterion 6, spec §4.7). Prose has to be real text in a
// contenteditable with spell check on, and chips and attribute lines have to opt out of it
// (both engines). IME composition and dead keys go through a DevTools session, which only
// Chromium has: a Japanese composition and the two dead keys, each in prose, right after a chip
// and inside a voice block, with the saved bytes compared against the oracle. WebKit gets the
// committed text through `keyboard.insertText` instead and one row that names the gap.

const LINKS = corpusFile('records/lore/topics/synthetic-links-explaining.md');
const VOICES = corpusFile('records/lore/topics/synthetic-voices-in-world.md');
const SCRIPT = corpusFile('records/story/scripts/S-004.md');

/** Where the text is entered: in prose, right behind a chip, and inside the text of a voice block. */
interface Place {
  name: 'prose' | 'chip' | 'voice';
  file: CorpusFile;
  at: Anchor;
}

const link = '[[fates-shrine]]';
const PLACES: Place[] = [
  // Before "operator-plate" in a plain paragraph.
  { name: 'prose', file: LINKS, at: { find: 'operator-plate names' } },
  // Right behind the first chip of the file: "A link to a record: [[fates-shrine]]|."
  { name: 'chip', file: LINKS, at: { find: link, offset: link.length } },
  // In the middle of a voice line, before "only": "Sing it wrong and you have |only made noise…"
  { name: 'voice', file: VOICES, at: { find: 'only made noise' } },
];

/** What the Japanese IME commits for the composition に, にほ, にほん, にほんご. */
const JAPANESE = { steps: ['に', 'にほ', 'にほん', 'にほんご'], commit: '日本語' };
/** A dead key is a composition of the bare accent; the layout commits the accented letter. */
const CIRCUMFLEX = { steps: ['^'], commit: 'ê' };
const UMLAUT = { steps: ['¨'], commit: 'ü' };
/** Everything the checks above commit, as one insertion for WebKit. */
const INSERTED = '日本語êü';

/**
 * Compose through the DevTools protocol: each step replaces the composition (`imeSetComposition`
 * with the caret at its end), the commit replaces the last one with the final text.
 */
async function compose(page: Page, steps: string[], commit: string): Promise<void> {
  const client = await page.context().newCDPSession(page);
  try {
    for (const text of steps) {
      await client.send('Input.imeSetComposition', { text, selectionStart: text.length, selectionEnd: text.length });
    }
    await client.send('Input.insertText', { text: commit });
  } finally {
    await client.detach().catch(() => undefined);
  }
}

/** How many composition events the page saw since `watchComposition`. */
interface CompositionCounts {
  start: number;
  end: number;
}

/**
 * Count `compositionstart` and `compositionend` on the page from now on, in the capture phase on
 * the window so that the editor cannot hide them. A check that composes has to see both: without
 * them `Input.insertText` was a plain insert, which would pass the byte comparison all the same.
 */
function watchComposition(page: Page): Promise<void> {
  return page.evaluate(() => {
    const counts = { start: 0, end: 0 };
    window.addEventListener('compositionstart', () => { counts.start += 1; }, true);
    window.addEventListener('compositionend', () => { counts.end += 1; }, true);
    Object.assign(window, { p10Composition: counts });
  });
}

const compositionCounts = (page: Page): Promise<CompositionCounts> =>
  page.evaluate(() => ({ ...(window as unknown as { p10Composition: CompositionCounts }).p10Composition }));

/** Compare what the editor saves with the original plus `text` at the anchor, byte by byte. */
async function againstOracle(page: Page, source: string, id: string, at: Anchor, text: string): Promise<Verdict> {
  await settle(page);
  const edit = { id, kind: 'insert', at, text } as const;
  const result = compareBytes(await savedText(page), expectedAfter(source, edit), editOffset(source, edit));
  if (result.identical) return judge([], `saved bytes equal the oracle insert of ${JSON.stringify(text)}`);
  const position = result.firstDiff === undefined ? '' : ` at ${result.firstDiff.line}:${result.firstDiff.column}`;
  return judge([
    `the saved text differs from the oracle insert of ${JSON.stringify(text)}${position}, ${result.extraBytes} byte(s) beyond it`,
  ]);
}

const subject = (variant: string, engine: string, check: string) => ({ variant, engine, check });

for (const variant of variantsUnderTest()) {
  // ── Both engines: spell check attributes ───────────────────────────────────

  test(`@input I-spellcheck-prose · ${variant.name}`, async ({ page, browserName }) => {
    await runCheck('input', subject(variant.name, browserName, 'I-spellcheck-prose'), async () => {
      await loadFile(page, variant.name, SCRIPT);
      await settle(page);
      // The element that holds a word of prose as a real text node.
      const found = await page.evaluate((word) => {
        const walker = document.createTreeWalker(document.getElementById('editor-host')!, NodeFilter.SHOW_TEXT);
        for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
          const owner = node.parentElement;
          if (owner !== null && (node.nodeValue ?? '').includes(word)) {
            return { tag: owner.tagName.toLowerCase(), editable: owner.isContentEditable, spellcheck: owner.spellcheck };
          }
        }
        return undefined;
      }, 'first night');
      if (found === undefined) {
        return judge(['no text node of the editor holds the prose "first night": the text is in a form field or not in the DOM']);
      }
      const problems: string[] = [];
      if (!found.editable) problems.push(`the <${found.tag}> that holds the prose is not contenteditable`);
      if (!found.spellcheck) problems.push(`spellcheck is off on the <${found.tag}> that holds the prose`);
      return judge(problems, `prose in a contenteditable <${found.tag}> with spellcheck on`);
    });
  });

  test(`@input I-spellcheck-off · ${variant.name}`, async ({ page, browserName }) => {
    await runCheck('input', subject(variant.name, browserName, 'I-spellcheck-off'), async () => {
      await loadFile(page, variant.name, SCRIPT);
      await settle(page);
      const flags = await page.evaluate(() => {
        const host = document.getElementById('editor-host')!;
        const spellcheck = (selector: string) => [...host.querySelectorAll<HTMLElement>(selector)].map((node) => node.spellcheck);
        return { chips: spellcheck('.tu-link'), rows: spellcheck('[data-beat-row]') };
      });
      const problems: string[] = [];
      if (flags.chips.length === 0) problems.push('no .tu-link chip is rendered');
      else if (flags.chips.some((on) => on)) problems.push(`${flags.chips.filter((on) => on).length} of ${flags.chips.length} chips have spellcheck on`);
      if (flags.rows.length === 0) problems.push('no [data-beat-row] is rendered');
      else if (flags.rows.some((on) => on)) problems.push(`${flags.rows.filter((on) => on).length} of ${flags.rows.length} beat rows have spellcheck on`);
      return judge(problems, `${flags.chips.length} chips and ${flags.rows.length} beat rows have spellcheck off`);
    });
  });

  // ── Chromium: IME composition and dead keys through a CDP session ──────────

  for (const place of PLACES) {
    const methods = [
      { check: `I-ime-ja-${place.name}`, ...JAPANESE },
      { check: `I-deadkey-circ-${place.name}`, ...CIRCUMFLEX },
      { check: `I-deadkey-uml-${place.name}`, ...UMLAUT },
    ];
    for (const method of methods) {
      test(`@input ${method.check} · ${variant.name}`, async ({ page, browserName }) => {
        test.skip(browserName !== 'chromium', 'IME composition goes through a CDP session, which only Chromium offers');
        await runCheck('input', subject(variant.name, browserName, method.check), async () => {
          const source = toLf(await loadFile(page, variant.name, place.file));
          await caretAt(page, resolveAnchor(source, place.at));
          await watchComposition(page);
          await compose(page, method.steps, method.commit);
          await settle(page);
          const counts = await compositionCounts(page);
          const seen = `compositionstart ${counts.start}, compositionend ${counts.end}`;
          // Without a composition the check did not test composition: it could not run.
          if (counts.start === 0 || counts.end === 0) throw new Error(`no composition events (${seen})`);
          const verdict = await againstOracle(page, source, method.check, place.at, method.commit);
          return { ...verdict, detail: `${seen}; ${verdict.detail ?? ''}` };
        });
      });
    }

    // ── WebKit: the committed text only ──────────────────────────────────────
    test(`@input I-insert-${place.name} · ${variant.name}`, async ({ page, browserName }) => {
      test.skip(browserName !== 'webkit', 'the insertText stand-in is for WebKit; Chromium composes through CDP');
      await runCheck('input', subject(variant.name, browserName, `I-insert-${place.name}`), async () => {
        const source = toLf(await loadFile(page, variant.name, place.file));
        await caretAt(page, resolveAnchor(source, place.at));
        await page.keyboard.insertText(INSERTED);
        return againstOracle(page, source, `I-insert-${place.name}`, place.at, INSERTED);
      });
    });
  }

  test(`@input I-ime-webkit · ${variant.name}`, async ({ browserName }) => {
    test.skip(browserName !== 'webkit', 'only WebKit has the gap');
    await runCheck(
      'input',
      subject(variant.name, browserName, 'I-ime-webkit'),
      async () => ({ ok: false, detail: 'gap: no composition input in Playwright WebKit' }),
      { gap: true },
    );
  });
}
