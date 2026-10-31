import { expect, test, type Page } from '@playwright/test';
import { readManifest } from '../../src/harness/manifest.ts';
import { resolveAnchor } from '../../src/harness/anchor.ts';
import { compareBytes } from '../../src/harness/compare.ts';
import { editOffset, expectedAfter, expectedAfterExternalUndo, toLf } from '../../src/harness/oracle.ts';
import type { ScriptedEdit } from '../../src/harness/types.ts';
import { undoOnce } from './drivers.ts';
import { edit, loadFile, record, strict, variantsUnderTest, type EditDriver } from './helpers.ts';
import type { RoundTripRow } from './write-reports.ts';

// The round-trip measurement behind CE-06.2: every corpus file goes into each variant, is saved
// and compared byte by byte with the oracle, unedited and after each scripted edit of the
// manifest, once through the editor's commands (`api`) and once through real input (`input`).
// A mismatch is a measurement, so it is recorded and the test passes; only P10_STRICT=1 fails it.

const { files } = readManifest();
const DRIVERS: EditDriver[] = ['api', 'input'];

/** The feature tag of each edit kind (the plan's conventions). */
const TAG: Record<ScriptedEdit['kind'], '@base' | '@links' | '@callouts' | '@beats'> = {
  insert: '@base',
  delete: '@base',
  paste: '@base',
  'select-all-type': '@base',
  external: '@base',
  'undo-all': '@base',
  'link-insert': '@links',
  'link-delete': '@links',
  'note-state': '@callouts',
  'beat-param': '@beats',
};

interface Subject {
  variant: string;
  engine: string;
  file: string;
  check: string;
  driver: RoundTripRow['driver'];
}

/** What the editor saves right now, and what it says about the level of that save. */
interface Saved {
  text: string;
  adapterLevel?: 'span' | 'block';
}

async function save(page: Page): Promise<Saved> {
  const { text, level } = await page.evaluate(() => {
    const harness = window.__harness!;
    const saved = harness.text();
    return { text: saved, level: harness.lastSaveLevel() };
  });
  return level === 'span' || level === 'block' ? { text, adapterLevel: level } : { text };
}

/** The row for one comparison of saved bytes with the expected text. */
function compared(subject: Subject, saved: Saved, expected: string, offset: number | undefined): RoundTripRow {
  const result = compareBytes(saved.text, expected, offset);
  const row: RoundTripRow = {
    ...subject,
    identical: result.identical,
    extraBytes: result.extraBytes,
    level: result.identical ? 'span' : result.withinBlock === true ? 'block' : 'none',
  };
  if (result.firstDiff !== undefined) row.firstDiff = result.firstDiff;
  if (result.withinBlock !== undefined) row.withinBlock = result.withinBlock;
  if (saved.adapterLevel !== undefined) row.adapterLevel = saved.adapterLevel;
  if (result.hunk !== undefined) row.hunk = result.hunk;
  return row;
}

/** A harness error is recorded, not compared: what the editor holds after it is not the edit's result. */
function failed(subject: Subject, error: unknown): RoundTripRow {
  const message = error instanceof Error ? error.message : String(error);
  return {
    ...subject,
    identical: false,
    extraBytes: 0,
    level: 'none',
    error: (message.split('\n')[0] ?? message).slice(0, 300),
  };
}

/** Record the row, and under P10_STRICT=1 fail the test on an error or a mismatch. */
function finish(row: RoundTripRow): void {
  record('round-trip', row);
  if (!strict()) return;
  expect(row.error, `${row.check} (${row.driver}) could not run`).toBeUndefined();
  const at = row.firstDiff === undefined ? '' : ` at ${row.firstDiff.line}:${row.firstDiff.column}`;
  expect(row.identical, `${row.check} (${row.driver}) saved other bytes than the oracle${at}\n${row.hunk ?? ''}`).toBe(true);
}

/** Run the edit and compare; `external` is compared twice, after the edit and after one undo. */
async function measure(
  page: Page,
  subject: Subject,
  driver: EditDriver,
  scripted: ScriptedEdit,
  original: string,
): Promise<RoundTripRow> {
  await edit(page, scripted, driver, original);
  const afterEdit = compared(subject, await save(page), expectedAfter(original, scripted), editOffset(original, scripted));
  if (scripted.kind !== 'external') return afterEdit;

  // The local edit is still unsaved; one undo must take back that edit and nothing else.
  await undoOnce(page, driver);
  const externalAt = resolveAnchor(toLf(original), scripted.external.at);
  const afterUndo = compared(subject, await save(page), expectedAfterExternalUndo(original, scripted), externalAt);
  if (!afterEdit.identical) return { ...afterEdit, stage: 'edit' };
  return afterUndo.identical ? afterEdit : { ...afterUndo, stage: 'undo' };
}

for (const variant of variantsUnderTest()) {
  for (const file of files) {
    const where = `${variant.name} · ${file.path}`;

    // AC: CE-06.2
    test(`@base unedited · ${where}`, async ({ page, browserName }) => {
      const subject: Subject = {
        variant: variant.name, engine: browserName, file: file.path, check: 'unedited', driver: 'load',
      };
      let row: RoundTripRow;
      try {
        const original = await loadFile(page, variant.name, file);
        row = compared(subject, await save(page), toLf(original), undefined);
      } catch (error) {
        row = failed(subject, error);
      }
      finish(row);
    });

    for (const scripted of file.edits) {
      for (const driver of DRIVERS) {
        // AC: CE-06.2
        test(`${TAG[scripted.kind]} ${scripted.id} via ${driver} · ${where}`, async ({ page, browserName }) => {
          const subject: Subject = {
            variant: variant.name, engine: browserName, file: file.path, check: scripted.id, driver,
          };
          let row: RoundTripRow;
          try {
            const original = await loadFile(page, variant.name, file);
            row = await measure(page, subject, driver, scripted, original);
          } catch (error) {
            row = failed(subject, error);
          }
          finish(row);
        });
      }
    }
  }
}
