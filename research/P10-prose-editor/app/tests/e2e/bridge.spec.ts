import { expect, test, type Page } from '@playwright/test';
import { readCorpusText, readManifest, type CorpusFile } from '../../src/harness/manifest.ts';
import { expectedAfter, expectedAfterExternalUndo, toLf } from '../../src/harness/oracle.ts';
import type { ScriptedEdit } from '../../src/harness/types.ts';
import { edit, loadFile } from './helpers.ts';

// The bridge and the baseline adapter, on the baseline only: they prove the harness that the
// prototype suites stand on (a textarea keeps bytes, so a failure here is a harness bug).

const { files } = readManifest();
const s004 = files.find((file) => file.path === 'records/story/scripts/S-004.md') as CorpusFile;
const original = toLf(readCorpusText(s004));

const text = (page: Page) => page.evaluate(() => window.__harness!.text());
const caret = (page: Page) => page.evaluate(() => window.__harness!.caret());
const caretTo = (page: Page, offset: number) => page.evaluate((o) => window.__harness!.caretTo(o), offset);
const focus = (page: Page) => page.evaluate(() => window.__harness!.focus());
const canUndo = (page: Page) => page.evaluate(() => window.__harness!.canUndo());

test.describe('@harness bridge · baseline', () => {
  test.beforeEach(async ({ page }) => {
    await loadFile(page, 'baseline', s004);
  });

  test('caretTo and caret agree', async ({ page }) => {
    await caretTo(page, 42);
    expect(await caret(page)).toBe(42);
  });

  test('load replaces the previous adapter', async ({ page }) => {
    const other = files.find((file) => file.path === 'records/lore/clues/C-031.md') as CorpusFile;
    await loadFile(page, 'baseline', other);
    expect(await text(page)).toBe(toLf(readCorpusText(other)));
    await expect(page.locator('#editor-host .hx-adapter')).toHaveCount(1);
    await expect(page.locator('#editor-host textarea')).toHaveCount(1);
  });

  test('load rejects an unknown variant and leaves nothing mounted', async ({ page }) => {
    const message = await page.evaluate(() => window.__harness!.load('nope', '', 'x').then(() => 'loaded', (e: Error) => e.message));
    expect(message).toContain('unknown variant');
    await expect(page.locator('#editor-host .hx-adapter')).toHaveCount(0);
  });

  test('typed text is undone and redone through the keymap', async ({ page }) => {
    const at = original.indexOf('The player');
    await focus(page);
    await caretTo(page, at);
    await page.keyboard.type('Hi ');
    expect(await text(page)).toBe(`${original.slice(0, at)}Hi ${original.slice(at)}`);
    expect(await canUndo(page)).toBe(true);
    for (let n = 0; n < 3; n++) await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toBe(original);
    expect(await canUndo(page)).toBe(false);
    await page.keyboard.press('ControlOrMeta+Shift+z');
    expect(await text(page)).toBe(`${original.slice(0, at)}H${original.slice(at)}`);
  });

  test('markSaved clears the undo history', async ({ page }) => {
    await focus(page);
    await caretTo(page, original.indexOf('The player'));
    await page.keyboard.type('x');
    await page.evaluate(() => window.__harness!.markSaved());
    expect(await canUndo(page)).toBe(false);
    await page.keyboard.press('ControlOrMeta+z');
    expect(await text(page)).toContain('xThe player');
    await page.keyboard.type('y');
    expect(await canUndo(page)).toBe(true);
  });

  test('the save key raises saveCount', async ({ page }) => {
    await focus(page);
    expect(await page.evaluate(() => window.__harness!.saveCount())).toBe(0);
    await page.keyboard.press('ControlOrMeta+s');
    expect(await page.evaluate(() => window.__harness!.saveCount())).toBe(1);
  });

  test('an external change moves the caret only when it lies before it', async ({ page }) => {
    const middle = original.indexOf('## Beat 1');
    await caretTo(page, middle);
    await page.evaluate(() => window.__harness!.applyExternal([{ from: 0, to: 0, insert: 'AB' }]));
    expect(await caret(page)).toBe(middle + 2);
    await page.evaluate((from) => window.__harness!.applyExternal([{ from, to: from, insert: 'CD' }]), middle + 10);
    expect(await caret(page)).toBe(middle + 2);
  });

  test('an external change keeps the local edit undoable and nothing else', async ({ page }) => {
    const external = {
      id: 'ext', kind: 'external',
      local: { at: { find: 'The player' }, text: 'LOCAL ' },
      external: { at: { find: '## Beat 2' }, text: 'EXT\n' },
    } as const satisfies ScriptedEdit;
    await edit(page, external, 'api');
    expect(await text(page)).toBe(expectedAfter(original, external));
    await page.evaluate(() => window.__harness!.undo());
    expect(await text(page)).toBe(expectedAfterExternalUndo(original, external));
  });

  test('select-all takes the body only; backspace at the body start leaves the front matter', async ({ page }) => {
    await focus(page);
    await page.keyboard.press('ControlOrMeta+a');
    await page.keyboard.type('new body');
    expect(await text(page)).toBe(expectedAfter(original, { id: 'sa', kind: 'select-all-type', text: 'new body' }));

    await loadFile(page, 'baseline', s004);
    await focus(page);
    await caretTo(page, original.indexOf("The player's"));
    await page.keyboard.press('Backspace');
    expect(await text(page)).toBe(original);
  });

  test('a paste inserts its plain text with LF line ends and ignores the HTML', async ({ page }) => {
    const at = original.indexOf('The player');
    await focus(page);
    await caretTo(page, at);
    await page.evaluate(() => {
      const data = new DataTransfer();
      data.setData('text/plain', 'one\r\ntwo ');
      data.setData('text/html', '<b>rich</b>');
      document.querySelector('textarea')!.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
    });
    expect(await text(page)).toBe(`${original.slice(0, at)}one\ntwo ${original.slice(at)}`);
  });
});

test.describe('@harness bridge · apply equals the oracle for every edit of the manifest', () => {
  for (const file of files) {
    test(file.path, async ({ page }) => {
      const source = toLf(await loadFile(page, 'baseline', file));
      for (const scripted of file.edits) {
        await loadFile(page, 'baseline', file);
        await edit(page, scripted, 'api');
        expect(await text(page), scripted.id).toBe(expectedAfter(source, scripted));
      }
    });
  }
});

test.describe('@harness toolbar', () => {
  test('the pickers load a file into a variant and the switch follows the mode', async ({ page }) => {
    await page.goto('/');
    await page.selectOption('[data-testid=file-picker]', s004.path);
    await expect(page.locator('#editor-host textarea')).toHaveCount(0); // no variant yet
    await page.selectOption('[data-testid=variant-picker]', 'baseline');
    await expect(page.locator('#editor-host textarea')).toHaveValue(original);
    await page.getByRole('button', { name: 'Source' }).click();
    await expect(page.locator('#editor-host textarea')).toHaveAttribute('data-mode', 'source');
    await page.evaluate(() => window.__harness!.setMode('write'));
    await expect(page.getByRole('button', { name: 'Write' })).toHaveClass(/is-active/);
  });
});
