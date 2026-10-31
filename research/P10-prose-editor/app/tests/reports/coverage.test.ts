import { describe, expect, it } from 'vitest';
import type { CorpusFile } from '../../src/harness/manifest.ts';
import type { VariantInfo } from '../../src/harness/types.ts';
import type { RoundTripRow } from '../e2e/write-reports.ts';
import { ENGINES, missingColumns, missingRows } from './coverage.ts';

// The coverage check of the report cannot run on real rows before the full measurement; these
// tests run it on rows made here, so a bug in the check shows now and not in Task 17.

const files: CorpusFile[] = [
  {
    path: 'records/a.md',
    origin: 'synthetic',
    lineEnds: 'lf',
    covers: [],
    edits: [{ id: 'word-insert', kind: 'insert', at: { find: 'x' }, text: 'y' }],
  },
];
const variant = (patch: Partial<VariantInfo> & { name: string }): VariantInfo => ({
  component: patch.name,
  version: '1',
  wysiwyg: false,
  packages: [],
  ...patch,
});
const row = (variantName: string, engine: string, check: string, driver: RoundTripRow['driver']): RoundTripRow => ({
  variant: variantName, engine, file: 'records/a.md', check, driver, identical: true, extraBytes: 0, level: 'span',
});
/** Every row a variant needs for the one file above. */
const completeRows = (variantName: string): RoundTripRow[] =>
  ENGINES.flatMap((engine) => [
    row(variantName, engine, 'unedited', 'load'),
    row(variantName, engine, 'word-insert', 'api'),
    row(variantName, engine, 'word-insert', 'input'),
  ]);

describe('missingRows', () => {
  const cm = variant({ name: 'codemirror' });

  it('finds nothing missing when every engine, check and driver has a row', () => {
    expect(missingRows(completeRows('codemirror'), [cm], files)).toEqual([]);
  });

  it('names a missing row', () => {
    const rows = completeRows('codemirror').filter((r) => !(r.engine === 'webkit' && r.driver === 'input'));
    expect(missingRows(rows, [cm], files)).toEqual(['codemirror | webkit | records/a.md | word-insert | input']);
  });

  it('wants the unedited row too', () => {
    const rows = completeRows('codemirror').filter((r) => r.check !== 'unedited');
    expect(missingRows(rows, [cm], files)).toHaveLength(ENGINES.length);
  });

  it('does not ask for the baseline', () => {
    expect(missingRows([], [variant({ name: 'baseline', baseline: true })], files)).toEqual([]);
  });
});

describe('missingColumns', () => {
  const stock = variant({ name: 'tiptap:stock', component: 'tiptap', wysiwyg: true, column: 'stock' });
  const bestEffort = variant({ name: 'tiptap:best-effort', component: 'tiptap', wysiwyg: true, column: 'best-effort' });

  it('wants both columns of a WYSIWYG component, each with rows', () => {
    const both = [...completeRows('tiptap:stock'), ...completeRows('tiptap:best-effort')];
    expect(missingColumns(both, [stock, bestEffort])).toEqual([]);
    expect(missingColumns(completeRows('tiptap:stock'), [stock, bestEffort])).toEqual(['tiptap: no rows for tiptap:best-effort']);
    expect(missingColumns(both, [stock])).toEqual(['tiptap: no best-effort variant is registered']);
  });

  it('wants one column of a source-based component', () => {
    expect(missingColumns([], [variant({ name: 'codemirror' })])).toEqual([]);
  });
});
