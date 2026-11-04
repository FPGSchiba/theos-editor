import { describe, expect, it } from 'vitest';
import { checkSection, longDocsSection, type CheckRow, type LongDocRow } from '../e2e/write-reports.ts';

const check = (patch: Partial<CheckRow>): CheckRow => ({
  variant: 'codemirror',
  engine: 'chromium',
  check: 'F-link-chip',
  ok: true,
  ...patch,
});

describe('the section of the feature, input and integration reports', () => {
  it('counts ok, failed, errors and gaps per variant and engine', () => {
    const text = checkSection(
      [
        check({}),
        check({ check: 'F-link-alias', ok: false, detail: 'no chip is rendered' }),
        check({ check: 'F-voice', ok: false, error: 'Timeout 3000ms exceeded' }),
        check({ engine: 'webkit', check: 'I-ime-webkit', ok: false, detail: 'gap: no composition input in Playwright WebKit' }),
        check({ engine: 'webkit' }),
      ],
      'a rendered feature',
    );
    expect(text).toContain('| codemirror | chromium | 3 | 1 | 2 | 1 | 0 |');
    expect(text).toContain('| codemirror | webkit | 2 | 1 | 1 | 0 | 1 |');
  });

  it('lists one line per failed check with its detail or error', () => {
    const text = checkSection(
      [
        check({ check: 'F-link-alias', ok: false, detail: 'no chip is rendered' }),
        check({ check: 'F-voice', ok: false, error: 'Timeout 3000ms exceeded' }),
        check({ check: 'F-note', ok: true, detail: 'n1=open' }),
      ],
      'a rendered feature',
    );
    expect(text).toContain('### codemirror · chromium');
    expect(text).toContain('- `F-link-alias`: no chip is rendered');
    expect(text).toContain('- `F-voice`: error: Timeout 3000ms exceeded');
    expect(text).not.toContain('- `F-note`');
  });

  it('draws a matrix with one column per variant and engine and one verdict per check', () => {
    const text = checkSection(
      [
        check({}),
        check({ engine: 'webkit', ok: false, detail: 'x' }),
        check({ variant: 'prosemirror:stock', check: 'F-link-chip', error: 'boom', ok: false }),
        check({ engine: 'webkit', check: 'I-ime-webkit', ok: false, detail: 'gap: no composition input in Playwright WebKit' }),
      ],
      'a rendered feature',
    );
    expect(text).toContain('| check | codemirror · chromium | codemirror · webkit | prosemirror:stock · chromium |');
    expect(text).toContain('| `F-link-chip` | ok | FAIL | error |');
    expect(text).toContain('| `I-ime-webkit` | - | gap | - |');
  });

  it('says so when nothing failed', () => {
    expect(checkSection([check({})], 'a rendered feature')).toContain('No failed checks.');
  });
});

describe('the long-documents section', () => {
  const row = (patch: Partial<LongDocRow>): LongDocRow => ({
    variant: 'codemirror',
    engine: 'chromium',
    size: 250_000,
    label: 'large',
    bytes: 250_115,
    loadMs: 65.6,
    typing: {
      start: { median: 3.5, p95: 13.6, n: 200 },
      middle: { median: 4.1, p95: 14.1, n: 200 },
      end: { median: 45.7, p95: 53.9, n: 200 },
    },
    typingFrame: {
      start: { median: 1.2, p95: 9.9, n: 200 },
      middle: { median: 2.4, p95: 10.1, n: 200 },
      end: { median: 30.2, p95: 41.6, n: 200 },
    },
    scrollMs: 8.4,
    scrollReachedEnd: true,
    heapBytes: 10_600_000,
    ...patch,
  });

  it('puts the numbers of each file in one table row, smallest file first, and names the machine', () => {
    const text = longDocsSection(
      [row({}), row({ size: 5_000, label: 'typical', bytes: 5_186, loadMs: 20.2 })],
      'Test CPU, 8 logical cores, 16.0 GiB RAM',
    );
    expect(text).toContain('Machine: Test CPU, 8 logical cores, 16.0 GiB RAM.');
    expect(text.indexOf('typical (5 KB)')).toBeLessThan(text.indexOf('large (250 KB)'));
    expect(text).toContain('typingFrame start | typingFrame middle | typingFrame end');
    expect(text).toContain(
      '| codemirror | chromium | large (250 KB) | 66 | 3.5 / 14 | 4.1 / 14 | 46 / 54 | 1.2 / 9.9 | 2.4 / 10 | 30 / 42 | 8.4 | 10.6 |',
    );
  });

  it('shows what is missing as a dash, a scroll that never ended with ">", and lists the errors', () => {
    const text = longDocsSection(
      [
        row({
          engine: 'webkit', size: 2_500_000, label: 'stress', bytes: 2_499_767,
          scrollReachedEnd: false, scrollMs: 10_000, heapBytes: undefined,
        }),
        row({ engine: 'webkit', label: 'broken', error: 'long-docs: five keystrokes led to no DOM change' }),
      ],
      'Test CPU',
    );
    expect(text).toContain(
      '| codemirror | webkit | stress (2.5 MB) | 66 | 3.5 / 14 | 4.1 / 14 | 46 / 54 | 1.2 / 9.9 | 2.4 / 10 | 30 / 42 | >10000 | - |',
    );
    expect(text).toContain('- codemirror · webkit · broken (250 KB): long-docs: five keystrokes led to no DOM change');
  });
});
