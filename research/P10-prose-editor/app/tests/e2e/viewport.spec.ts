import { expect, test } from '@playwright/test';
import { splitFrontMatter } from '../../src/harness/frontmatter.ts';
import { readCorpusText } from '../../src/harness/manifest.ts';
import { toLf } from '../../src/harness/oracle.ts';
import { collectRows, collectTexts, corpusFile, count, findScrollers, inPage } from './checks.ts';
import { variantsUnderTest } from './helpers.ts';

// A probe of the harness itself: the feature checks read constructs that are not under the caret,
// and an editor that draws only the viewport plus a margin (CodeMirror 6) must be read where it
// draws them. The record here is padded, in the test and not in the corpus, so that its voice
// blocks lie far below the fold; a check that reads the page in one pass would not see them.

const VOICES = corpusFile('records/lore/topics/synthetic-voices-in-world.md');
/** Taller than the 1280x720 window plus the roughly 500 px margin a viewport editor draws. */
const PADDING_PARAGRAPHS = 60;

function padded(): string {
  const text = toLf(readCorpusText(VOICES));
  const { body } = splitFrontMatter(text);
  const head = text.slice(0, text.length - body.length);
  const filler = Array.from(
    { length: PADDING_PARAGRAPHS },
    (_, index) => `Synthetic filler paragraph ${index + 1}, only here to push the voices below the fold of the window.`,
  ).join('\n\n');
  return `${head}${filler}\n\n${body}`;
}

for (const variant of variantsUnderTest()) {
  // AC: CE-06.1
  test(`@harness constructs far below the fold are read while scrolling · ${variant.name}`, async ({ page }) => {
    await page.goto('/');
    await page.waitForFunction(() => window.__harness !== undefined);
    await page.evaluate(([name, content, fileId]) => window.__harness!.load(name, content, fileId), [
      variant.name,
      padded(),
      VOICES.path,
    ] as const);

    const height = await page.evaluate(
      (finder) => Math.max(...((0, eval)(finder)() as Element[]).map((node) => node.scrollHeight)),
      inPage(findScrollers),
    );
    const window720 = await page.evaluate(() => window.innerHeight);
    expect(height, 'the padded record is taller than the window plus a margin').toBeGreaterThan(window720 + 1_000);

    const caretBefore = await page.evaluate(() => window.__harness!.caret());
    const heads = await collectTexts(page, '.tu-voice__head');
    const voices = await collectRows(page, '[data-callout="voice"]', () => ({}));
    const unscrolled = await count(page, '[data-callout="voice"]');
    expect(voices.length, 'a scrolling read finds at least what one pass finds').toBeGreaterThanOrEqual(unscrolled);
    expect(await page.evaluate(() => window.__harness!.caret()), 'reading does not move the caret').toBe(caretBefore);

    if (variant.wysiwyg) {
      expect(heads.some((head) => head.includes('Mother Ilse')), 'the head of the last voice is found').toBe(true);
      expect(heads.some((head) => head.includes('Hearth folk')), 'the head of the first voice is found').toBe(true);
      const words = await collectRows(page, '[data-dev-word]', (node) => ({ text: (node.textContent ?? '').trim() }));
      expect(words.map((word) => word.text.toLowerCase())).toContain('ledger');
    }
  });
}
