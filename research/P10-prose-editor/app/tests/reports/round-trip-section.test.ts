import { describe, expect, it } from 'vitest';
import { roundTripSection, type RoundTripRow } from '../e2e/write-reports.ts';

const row = (patch: Partial<RoundTripRow>): RoundTripRow => ({
  variant: 'codemirror',
  engine: 'chromium',
  file: 'records/a.md',
  check: 'unedited',
  driver: 'load',
  identical: true,
  extraBytes: 0,
  level: 'span',
  ...patch,
});

describe('the round-trip report section', () => {
  it('counts unedited files and edits per variant and engine', () => {
    const text = roundTripSection([
      row({}),
      row({ file: 'records/b.md', identical: false, extraBytes: 3, level: 'none', firstDiff: { line: 2, column: 1 }, hunk: '-a\n+b\n' }),
      row({ check: 'word-insert', driver: 'api' }),
      row({ check: 'word-insert', driver: 'input', identical: false, extraBytes: 1, level: 'block' }),
      row({ check: 'word-delete', driver: 'input', identical: false, extraBytes: 0, level: 'none', error: 'feature missing: x' }),
      row({ engine: 'webkit' }),
    ]);
    expect(text).toContain('| codemirror | chromium | 1/2 | 1/3 | 1/1 | 0/2 | 1 |');
    expect(text).toContain('| codemirror | webkit | 1/1 | 0/0 | 0/0 | 0/0 | 0 |');
  });

  it('lists one entry per mismatch with file, edit, driver, position, extra bytes, level and hunk', () => {
    const text = roundTripSection([
      row({
        file: 'records/b.md',
        check: 'link-chip-delete',
        driver: 'input',
        identical: false,
        extraBytes: 14,
        level: 'block',
        adapterLevel: 'span',
        firstDiff: { line: 10, column: 107 },
        hunk: '--- expected\n+++ saved\n@@ -10 +10 @@\n-x\n+y\n',
      }),
    ]);
    expect(text).toContain('### codemirror · chromium');
    expect(text).toContain('#### `records/b.md` · `link-chip-delete` · input');
    expect(text).toContain('- position: 10:107');
    expect(text).toContain('- extra bytes: 14');
    expect(text).toContain('- level: block (the adapter says: span)');
    expect(text).toContain('```diff\n--- expected\n+++ saved\n@@ -10 +10 @@\n-x\n+y\n```');
  });

  it('shows an error without a comparison, and which stage of an external edit failed', () => {
    const text = roundTripSection([
      row({ check: 'beat-param-add', driver: 'input', identical: false, level: 'none', error: 'feature missing: the beat row' }),
      row({ check: 'external-while-dirty', driver: 'api', identical: false, extraBytes: 6, level: 'none', stage: 'undo' }),
    ]);
    expect(text).toContain('- error: feature missing: the beat row');
    expect(text).not.toMatch(/beat-param-add[^#]*- extra bytes/);
    expect(text).toContain('- failed after: one undo');
  });

  it('fences a hunk that itself shows a code fence with a longer fence', () => {
    const text = roundTripSection([
      row({ identical: false, extraBytes: 1, level: 'none', hunk: '-```js\n+```ts\n' }),
    ]);
    expect(text).toContain('````diff\n-```js\n+```ts\n````');
  });

  it('says so when nothing mismatched', () => {
    const text = roundTripSection([row({}), row({ check: 'word-insert', driver: 'api' })]);
    expect(text).toContain('No mismatches.');
    expect(text).toContain('| codemirror | chromium | 1/1 | 1/1 | 1/1 | 0/0 | 0 |');
  });
});
