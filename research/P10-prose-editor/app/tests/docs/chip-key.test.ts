import { describe, expect, it } from 'vitest';
import { chipKeys, uniqueByKey, type LinePart } from '../e2e/chip-key.ts';

// The key behind the chip count of N-remount (tests/e2e/integration.spec.ts).

describe('chipKeys', () => {
  it('tells two chips with the same id on one line apart', () => {
    const line: LinePart[] = ['See ', { chip: 'a' }, ' and ', { chip: 'a' }, ' again'];
    const keys = chipKeys(line);
    expect(keys).toHaveLength(2);
    expect(new Set(keys).size).toBe(2);
  });

  it('numbers chips that have the same id and the same surroundings', () => {
    const keys = chipKeys([{ chip: 'a' }, { chip: 'a' }]);
    expect(new Set(keys).size).toBe(2);
  });

  it('counts a chip once when two reads see it 1 to 3 px apart', () => {
    const line: LinePart[] = ['A link to a record: ', { chip: 'mother-ilse' }, ', done.'];
    const [key] = chipKeys(line);
    const reads = [0, 1, 2, 3].map((shift) => ({ key: chipKeys([...line])[0]!, at: 400 + shift }));
    expect(uniqueByKey(reads, (row) => row.key)).toHaveLength(1);
    expect(reads[0]!.key).toBe(key);
  });

  it('keeps one key per chip whichever part of a long line is read', () => {
    const line: LinePart[] = ['x'.repeat(50), { chip: 'a' }, 'y'.repeat(50)];
    expect(chipKeys(line)[0]).toBe(chipKeys(line)[0]);
    expect(chipKeys(line)[0]).toContain('x'.repeat(20));
    expect(chipKeys(line)[0]).not.toContain('x'.repeat(21));
  });
});
