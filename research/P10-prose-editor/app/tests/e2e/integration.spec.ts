import { test, type Page } from '@playwright/test';
import { resolveAnchor } from '../../src/harness/anchor.ts';
import { splitFrontMatter } from '../../src/harness/frontmatter.ts';
import type { CorpusFile } from '../../src/harness/manifest.ts';
import { toLf } from '../../src/harness/oracle.ts';
import {
  caretAt, collectRows, corpusFile, count, eventually, judge, runCheck, savedText, settle, texts, type Verdict,
} from './checks.ts';
import { loadFile, variantsUnderTest } from './helpers.ts';

// How the editor fits the app (CE-06.1 criterion 8): an external change that keeps the caret, the
// undo history the host clears on save (CE-34.3), the one keymap, mounting and tearing down again
// and again, and index changes that arrive without a new editor. Every check runs on the
// harness's own entry points (`window.__harness`), so each variant is measured the same way.

const HALL = corpusFile('records/world/hearth/f04-choir-hall.md');
const LINKS = corpusFile('records/lore/topics/synthetic-links-explaining.md');
const SCRIPT = corpusFile('records/story/scripts/S-004.md');
const VOICES = corpusFile('records/lore/topics/synthetic-voices-in-world.md');
const NOTES = corpusFile('records/maps/synthetic-notes.md');

const caretOf = (page: Page): Promise<number> => page.evaluate(() => window.__harness!.caret());
const canUndo = (page: Page): Promise<boolean> => page.evaluate(() => window.__harness!.canUndo());

/** What a check may use besides the page: the console errors collected since the page opened. */
interface RunContext {
  variant: string;
  errors: string[];
}

interface Check {
  id: string;
  file: CorpusFile;
  /** `source` is the LF form of `file` as loaded; `page` has it open in the variant. */
  run: (page: Page, source: string, context: RunContext) => Promise<Verdict>;
}

const checks: Check[] = [
  {
    id: 'N-external-caret',
    file: HALL,
    async run(page, source) {
      // "A half-buried … the |Hearth folk| have turned into their holiest place."
      const start = resolveAnchor(source, { find: 'A half-buried' });
      const caret = resolveAnchor(source, { find: 'Hearth folk' });
      const end = resolveAnchor(source, { find: 'holiest place.', offset: 'holiest place.'.length });
      const before = 'EXT ';
      const after = ' EXT';
      const problems: string[] = [];

      await caretAt(page, caret);
      const placed = await caretOf(page);
      if (placed !== caret) problems.push(`caretTo(${caret}) left the caret at ${placed}`);

      // A change before the caret moves it by the length of the insert.
      await page.evaluate((change) => window.__harness!.applyExternal([change]), { from: start, to: start, insert: before });
      await settle(page);
      const shifted = await caretOf(page);
      if (shifted !== placed + before.length) {
        problems.push(`after an external insert before the caret it is at ${shifted}, expected ${placed + before.length}`);
      }

      // A change behind it leaves it where it is.
      await page.evaluate(
        (change) => window.__harness!.applyExternal([change]),
        { from: end + before.length, to: end + before.length, insert: after },
      );
      await settle(page);
      const kept = await caretOf(page);
      if (kept !== shifted) problems.push(`an external insert behind the caret moved it from ${shifted} to ${kept}`);

      const expected = `${source.slice(0, start)}${before}${source.slice(start, end)}${after}${source.slice(end)}`;
      if ((await savedText(page)) !== expected) problems.push('the text is not the original with both external inserts');
      return judge(problems, 'the caret shifts for a change before it and stays for one behind it');
    },
  },
  {
    id: 'N-history',
    file: HALL,
    async run(page, source) {
      // Review Focus 4: undo after save. markSaved() clears the history, so undo has nothing to take.
      const at = resolveAnchor(source, { find: 'Hearth folk' });
      const problems: string[] = [];

      await caretAt(page, at);
      await page.keyboard.type('abc');
      if (!(await canUndo(page))) problems.push('canUndo() is false after an edit');

      await page.evaluate(() => window.__harness!.markSaved());
      if (await canUndo(page)) problems.push('canUndo() is still true after markSaved()');

      const saved = await savedText(page);
      await page.keyboard.press('ControlOrMeta+z');
      await settle(page);
      if ((await savedText(page)) !== saved) problems.push('ControlOrMeta+z after markSaved() changed the text');

      await caretAt(page, at);
      await page.keyboard.type('x');
      if (!(await canUndo(page))) problems.push('canUndo() is false after a new edit');
      return judge(problems, 'markSaved() empties the history; a new edit fills it again');
    },
  },
  {
    id: 'N-keymap-save',
    file: HALL,
    async run(page) {
      await page.evaluate(() => window.__harness!.focus());
      const before = await page.evaluate(() => window.__harness!.saveCount());
      await page.keyboard.press('ControlOrMeta+s');
      await settle(page);
      const after = await page.evaluate(() => window.__harness!.saveCount());
      return after === before + 1
        ? judge([], 'one ControlOrMeta+s asks the app to save once')
        : judge([`saveCount() went from ${before} to ${after}, expected ${before + 1}`]);
    },
  },
  {
    id: 'N-keymap-undo-redo',
    file: HALL,
    async run(page, source) {
      // One insertion is one undo step in every editor.
      await caretAt(page, resolveAnchor(source, { find: 'Hearth folk' }));
      await page.keyboard.insertText('hello ');
      const edited = await savedText(page);
      const problems: string[] = [];
      if (edited === source) problems.push('the insertion did not change the text, so undo cannot be judged');

      await page.keyboard.press('ControlOrMeta+z');
      await settle(page);
      if ((await savedText(page)) !== source) problems.push('ControlOrMeta+z did not take the edit back');

      await page.keyboard.press('ControlOrMeta+Shift+z');
      await settle(page);
      if ((await savedText(page)) !== edited) problems.push('ControlOrMeta+Shift+z did not bring the edit back');
      return judge(problems, 'ControlOrMeta+z undoes and ControlOrMeta+Shift+z redoes');
    },
  },
  {
    id: 'N-remount',
    file: LINKS,
    async run(page, source, { variant, errors }) {
      // Twenty loads in a row, each one tearing the previous editor down. The first was the link
      // file, then come eighteen others, and the last is the link file again: all of its links are
      // chips in prose, so the chips on the page are exactly its links.
      const others = [SCRIPT, NOTES, VOICES];
      for (let load = 0; load < 18; load++) await loadFile(page, variant, others[load % others.length]!);
      await loadFile(page, variant, LINKS);

      const links = splitFrontMatter(source).body.match(/\[\[[^\]\n]+\]\]/g)?.length ?? 0;
      // A viewport editor draws only the chips near the window: count them over the whole scroll
      // range, each at its document-absolute position, so a recycled element counts once.
      const chipsDrawn = async (): Promise<number> =>
        (await collectRows(page, '.tu-link', (chip) => ({ id: chip.getAttribute('data-link-id') }))).length;
      const shown = await eventually(async () => (await chipsDrawn()) === links);
      const chips = await chipsDrawn();
      const problems: string[] = [];
      if (!shown) problems.push(`after 20 loads the editor shows ${chips} chips, the file has ${links} links`);
      const adapters = await count(page, '.hx-adapter');
      if (adapters !== 1) problems.push(`${adapters} editors are mounted, expected 1`);
      if (errors.length > 0) problems.push(`${errors.length} console error(s), the first: ${errors[0]?.slice(0, 160)}`);
      return judge(problems, `20 loads, ${chips} chips for ${links} links, no console errors`);
    },
  },
  {
    id: 'N-reactive',
    file: LINKS,
    async run(page, source) {
      // The host pushes a changed index in; the editor redraws the chip and is not re-created: its
      // elements, its text, its caret and its undo history are what they were before.
      const title = 'Mother Ilse (renamed by the index)';
      const marked = await page.evaluate(() => {
        const host = document.getElementById('editor-host')!;
        const root = host.querySelector('.hx-adapter')?.firstElementChild;
        const surface = host.querySelector('[contenteditable="true"], textarea');
        if (root === null || root === undefined) return false;
        // Two properties: for a textarea the root and the editable element are the same node.
        Object.assign(root, { p10RootMark: true });
        if (surface !== null) Object.assign(surface, { p10SurfaceMark: true });
        return true;
      });
      if (!marked) return judge(['the editor rendered no root element to mark']);

      // A local edit (so there is something to undo), then a caret in front of it. A redraw that
      // replaces the whole document would lose the history or move the caret.
      await caretAt(page, resolveAnchor(source, { find: 'operator-plate names' }));
      await page.keyboard.insertText('edit ');
      await caretAt(page, resolveAnchor(source, { find: 'A link to a record:', offset: 5 }));
      await settle(page);
      const before = { text: await savedText(page), caret: await caretOf(page), canUndo: await canUndo(page) };

      await page.evaluate((info) => window.__harness!.updateIndex([info]), { id: 'mother-ilse', kind: 'npc' as const, title });
      const sel = '.tu-link[data-link-id="mother-ilse"] .tu-link__title';
      const redrawn = await eventually(async () => (await texts(page, sel)).includes(title));
      await settle(page);
      const after = { text: await savedText(page), caret: await caretOf(page), canUndo: await canUndo(page) };

      const same = await page.evaluate(() => {
        const host = document.getElementById('editor-host')!;
        const root = host.querySelector('.hx-adapter')?.firstElementChild as (Element & { p10RootMark?: boolean }) | null | undefined;
        const surface = host.querySelector('[contenteditable="true"], textarea') as (Element & { p10SurfaceMark?: boolean }) | null;
        return { root: root?.p10RootMark === true, surface: surface === null || surface.p10SurfaceMark === true };
      });
      const problems: string[] = [];
      if (before.text === source) problems.push('the local edit did not change the text, so updateIndex() cannot be judged against it');
      if (!before.canUndo) problems.push('canUndo() is false after a local edit, so the undo history cannot be judged');
      if (!redrawn) {
        const shown = (await texts(page, sel))[0];
        problems.push(shown === undefined ? 'no chip for mother-ilse is rendered' : `the chip of mother-ilse still shows ${JSON.stringify(shown)}, not the new title`);
      }
      if (!same.root) problems.push('the editor root element is a different object after updateIndex()');
      if (!same.surface) problems.push('the editable element is a different object after updateIndex()');
      if (after.text !== before.text) problems.push('updateIndex() changed the text');
      if (after.caret !== before.caret) problems.push(`updateIndex() moved the caret from ${before.caret} to ${after.caret}`);
      if (after.canUndo !== before.canUndo) problems.push(`updateIndex() changed canUndo() from ${before.canUndo} to ${after.canUndo}`);
      return judge(problems, 'the chip shows the new title; elements, text, caret and undo history are unchanged');
    },
  },
];

for (const variant of variantsUnderTest()) {
  for (const check of checks) {
    test(`@integration ${check.id} · ${variant.name}`, async ({ page, browserName }) => {
      // Listen from the first page load on: a mount that logs an error is a finding of N-remount.
      const errors: string[] = [];
      page.on('console', (message) => {
        if (message.type() === 'error' && !message.location().url.endsWith('/favicon.ico')) errors.push(message.text());
      });
      page.on('pageerror', (error) => errors.push(error.message));

      const subject = { variant: variant.name, engine: browserName, check: check.id };
      await runCheck('integration', subject, async () => {
        const text = await loadFile(page, variant.name, check.file);
        return check.run(page, toLf(text), { variant: variant.name, errors });
      });
    });
  }
}
