import { describe, expect, it } from 'vitest';
import { compareBytes } from './compare.ts';

describe('compareBytes', () => {
  it('reports identical', () => {
    expect(compareBytes('a\né\n', 'a\né\n')).toEqual({ identical: true, extraBytes: 0 });
  });

  it('reports identical without a hunk, a position or withinBlock', () => {
    const result = compareBytes('same', 'same', 2);
    expect(result).not.toHaveProperty('firstDiff');
    expect(result).not.toHaveProperty('hunk');
    expect(result).not.toHaveProperty('withinBlock');
  });

  it('reports the first differing line and column (1-based)', () => {
    const result = compareBytes('one\ntwx\nthree\n', 'one\ntwo\nthree\n');
    expect(result.identical).toBe(false);
    expect(result.firstDiff).toEqual({ line: 2, column: 3 });
  });

  it('reports column 1 when a line differs at its first character', () => {
    expect(compareBytes('one\nXwo\n', 'one\ntwo\n').firstDiff).toEqual({ line: 2, column: 1 });
  });

  it('reports the end of the shorter text when only a tail is missing', () => {
    expect(compareBytes('abc', 'abc\n').firstDiff).toEqual({ line: 1, column: 4 });
    expect(compareBytes('abc\n', 'abc').firstDiff).toEqual({ line: 1, column: 4 });
  });

  it('does not count a CR as a line break', () => {
    expect(compareBytes('a\r\nb', 'a\nb').firstDiff).toEqual({ line: 1, column: 2 });
  });

  it('counts extraBytes in UTF-8', () => {
    // saved has a decomposed é (e + U+0301: 3 bytes), expected the composed é (2 bytes)
    const saved = 'xéy';
    const expected = 'xéy';
    expect(saved).not.toBe(expected);
    const result = compareBytes(saved, expected);
    expect(result.identical).toBe(false);
    expect(result.extraBytes).toBe(3);
    expect(compareBytes(expected, saved).extraBytes).toBe(3);
  });

  it('takes the larger differing middle after the common prefix and suffix', () => {
    expect(compareBytes('abc', 'aXXc').extraBytes).toBe(2);
    expect(compareBytes('aXXc', 'abc').extraBytes).toBe(2);
    expect(compareBytes('a€b', 'aXb').extraBytes).toBe(3);
    expect(compareBytes('ab', 'a').extraBytes).toBe(1);
  });

  it('does not split a surrogate pair when counting', () => {
    expect(compareBytes('a\u{1F600}b', 'a\u{1F601}b').extraBytes).toBe(4);
    expect(compareBytes('\u{1F600}', '\u{1F601}').firstDiff).toEqual({ line: 1, column: 1 });
  });

  it('does not let prefix and suffix overlap', () => {
    const result = compareBytes('aa', 'aaa');
    expect(result.extraBytes).toBe(1);
  });

  describe('withinBlock', () => {
    const expected = 'First line\nsecond line\n\nOther block\nmore\n\nLast\n';

    it('is undefined without editOffset', () => {
      expect(compareBytes(expected.replace('second', 'secXnd'), expected)).not.toHaveProperty('withinBlock');
    });

    it('is true when the differing range lies in the block around editOffset', () => {
      const saved = expected.replace('second', 'secXnd');
      expect(compareBytes(saved, expected, 2).withinBlock).toBe(true);
      expect(compareBytes(saved, expected, expected.indexOf('second line') + 3).withinBlock).toBe(true);
    });

    it('is false when the differing range lies in another block', () => {
      const saved = expected.replace('Other', 'Othxr');
      expect(compareBytes(saved, expected, 2).withinBlock).toBe(false);
    });

    it('is false when the damage runs past the blank line that ends the block', () => {
      const saved = expected.replace('second line\n\nOther', 'second line\nOther');
      expect(compareBytes(saved, expected, 2).withinBlock).toBe(false);
    });

    it('is false when the differing range reaches before the block', () => {
      const saved = expected.replace('First', 'Fxrst');
      expect(compareBytes(saved, expected, expected.indexOf('Other block') + 1).withinBlock).toBe(false);
    });

    it('counts a missing final newline of the last block as inside that block', () => {
      const text = 'A\n\nLast\n';
      expect(compareBytes('A\n\nLast', text, text.indexOf('Last')).withinBlock).toBe(true);
    });

    it('treats a blank line at editOffset as its own block', () => {
      const text = 'A\n\nB\n';
      expect(compareBytes('A\nX\nB\n', text, 2).withinBlock).toBe(true);
      expect(compareBytes('AX\n\nB\n', text, 2).withinBlock).toBe(false);
    });
  });

  describe('hunk', () => {
    it('shows whitespace', () => {
      const result = compareBytes('x y\t\r\nnext\n', 'x y\nnext\n');
      expect(result.identical).toBe(false);
      expect(result.hunk).toContain('-x␣y');
      expect(result.hunk).toContain('+x⍽y⇥␍');
    });

    it('maps spaces, tabs, CR and U+00A0 and leaves other text alone', () => {
      const hunk = compareBytes(' \t\r z\n', 'q\n').hunk ?? '';
      expect(hunk).toContain('+␣⇥␍⍽z');
      expect(hunk).toContain('-q');
      expect(hunk).not.toMatch(/[\t\r ]/);
    });

    it('stays short and fast when every line differs', () => {
      const lines = Array.from({ length: 20000 }, (_, i) => `line ${i}`);
      const expected = `${lines.join('\n')}\n`;
      const saved = `${lines.map((line) => `${line}!`).join('\n')}\n`;
      const result = compareBytes(saved, expected);
      expect(result.identical).toBe(false);
      expect(result.firstDiff).toEqual({ line: 1, column: 7 });
      expect(result.hunk).toContain('too many changed lines');
      expect(result.hunk).toContain('-line␣0\n');
      expect(result.hunk).toContain('+line␣0!\n');
      expect(result.hunk?.split('\n').length).toBeLessThan(30);
    });

    it('names the two sides and starts at the hunk header', () => {
      const hunk = compareBytes('b\n', 'a\n').hunk ?? '';
      expect(hunk).toContain('--- expected');
      expect(hunk).toContain('+++ saved');
      expect(hunk).toContain('@@');
    });
  });
});
