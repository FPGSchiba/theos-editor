import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readManifest } from '../../src/harness/manifest.ts';
import { variants } from '../../src/harness/variants.ts';
import type { RoundTripRow } from '../e2e/write-reports.ts';
import { missingColumns, missingRows } from './coverage.ts';

/** `research/P10-prose-editor/report/round-trip.json`, written by the full measurement run (Task 17). */
const REPORT = fileURLToPath(new URL('../../../report/round-trip.json', import.meta.url));

function readRows(): RoundTripRow[] {
  expect(
    existsSync(REPORT),
    'report/round-trip.json is missing: it is written by the full measurement run (P10_REPORT=1 pnpm exec playwright test)',
  ).toBe(true);
  return (JSON.parse(readFileSync(REPORT, 'utf8')) as { rows: RoundTripRow[] }).rows;
}

describe('report/round-trip.json', () => {
  // AC: CE-06.2
  it('has a row for every prototype, engine, file, unedited check and edit with both drivers', () => {
    expect(missingRows(readRows(), variants, readManifest().files)).toEqual([]);
  });

  // AC: CE-06.2
  it('has both columns, stock and best-effort, of every WYSIWYG component', () => {
    expect(missingColumns(readRows(), variants)).toEqual([]);
  });
});
