import { expect, test } from '@playwright/test';
import { readCorpusText, readManifest } from '../../src/harness/manifest.ts';
import { toLf } from '../../src/harness/oracle.ts';
import type { VariantInfo } from '../../src/harness/types.ts';
import { loadFile } from './helpers.ts';

// The smoke test always runs the baseline variant (a plain <textarea>), whatever P10_VARIANTS
// says: it proves the harness (app, bridge, helpers) before any prototype exists.
const { files } = readManifest();

test.describe('@base smoke · baseline', () => {
  test('@base variants() lists baseline with baseline: true', async ({ page }) => {
    await loadFile(page, 'baseline', files[0]!);
    const variants = await page.evaluate(() => window.__harness!.variants());
    const baseline = variants.find((variant: VariantInfo) => variant.name === 'baseline');
    expect(baseline).toBeDefined();
    expect(baseline?.baseline).toBe(true);
    expect(baseline?.wysiwyg).toBe(false);
  });

  for (const file of files) {
    test(`@base text() equals the LF form · ${file.path}`, async ({ page }) => {
      await loadFile(page, 'baseline', file);
      const text = await page.evaluate(() => window.__harness!.text());
      expect(text).toBe(toLf(readCorpusText(file)));
    });

    test(`@base setMode('source') keeps the text · ${file.path}`, async ({ page }) => {
      await loadFile(page, 'baseline', file);
      const expected = toLf(readCorpusText(file));
      await page.evaluate(() => window.__harness!.setMode('source'));
      expect(await page.evaluate(() => window.__harness!.text())).toBe(expected);
      await page.evaluate(() => window.__harness!.setMode('write'));
      expect(await page.evaluate(() => window.__harness!.text())).toBe(expected);
    });
  }
});
