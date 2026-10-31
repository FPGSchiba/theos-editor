import { describe, expect, it } from 'vitest';
import { resolveAnchor } from './anchor.ts';
import { compareAtSites, compareBytes } from './compare.ts';
import { readCorpusText, readManifest } from './manifest.ts';
import { editOffset, expectedAfter, expectedAfterExternalUndo, expectedSites, sites, toLf } from './oracle.ts';
import type { ScriptedEdit } from './types.ts';

describe('compareBytes', () => {
  it('reports identical', () => {
    expect(compareBytes('a\n\u00e9\n', 'a\n\u00e9\n')).toEqual({ identical: true, extraBytes: 0 });
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
    const saved = 'xe\u0301y';
    const expected = 'x\u00e9y';
    expect(saved).not.toBe(expected);
    const result = compareBytes(saved, expected);
    expect(result.identical).toBe(false);
    expect(result.extraBytes).toBe(3);
    expect(compareBytes(expected, saved).extraBytes).toBe(3);
  });

  it('takes the larger differing middle after the common prefix and suffix', () => {
    expect(compareBytes('abc', 'aXXc').extraBytes).toBe(2);
    expect(compareBytes('aXXc', 'abc').extraBytes).toBe(2);
    expect(compareBytes('a\u20acb', 'aXb').extraBytes).toBe(3);
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

    describe('with front matter directly above the first body block', () => {
      const expected = '---\nid: x\n---\n# Title\nBody text\n\nNext para\n';
      const at = expected.indexOf('Body');

      it('is false when only the front matter is damaged', () => {
        expect(compareBytes(expected.replace('id: x', 'id: y'), expected, at).withinBlock).toBe(false);
        expect(compareBytes(expected.replace('---\n#', '--\n#'), expected, at).withinBlock).toBe(false);
        expect(compareBytes(expected.replace('---\nid', '-\nid'), expected, at).withinBlock).toBe(false);
      });

      it('is true when only the body block is damaged', () => {
        expect(compareBytes(expected.replace('Body', 'Bxdy'), expected, at).withinBlock).toBe(true);
        expect(compareBytes(expected.replace('# Title', '#Title'), expected, at).withinBlock).toBe(true);
      });

      it('is false when the damage spans the front matter and the body block', () => {
        const saved = expected.replace('x\n---\n# Title\nBody', 'x\nBody');
        expect(compareBytes(saved, expected, at).withinBlock).toBe(false);
      });
    });

    it('treats a blank line at editOffset as its own block', () => {
      const text = 'A\n\nB\n';
      expect(compareBytes('A\nX\nB\n', text, 2).withinBlock).toBe(true);
      expect(compareBytes('AX\n\nB\n', text, 2).withinBlock).toBe(false);
    });
  });

  describe('hunk', () => {
    it('shows whitespace', () => {
      const result = compareBytes('x\u00a0y\t\r\nnext\n', 'x y\nnext\n');
      expect(result.identical).toBe(false);
      expect(result.hunk).toContain('-x␣y');
      expect(result.hunk).toContain('+x⍽y⇥␍');
    });

    it('maps spaces, tabs, CR and U+00A0 and leaves other text alone', () => {
      const hunk = compareBytes(' \t\r\u00a0z\n', 'q\n').hunk ?? '';
      expect(hunk).toContain('+␣⇥␍⍽z');
      expect(hunk).toContain('-q');
      expect(hunk).not.toMatch(/[\t\r\u00a0]/);
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

describe('compareAtSites', () => {
  const text = 'One\nuno\n\nTwo\ndos\n\nThree\ntres\n';

  it('is compareBytes without an offset when there is no site', () => {
    expect(compareAtSites(text, text, [])).toEqual({ identical: true, extraBytes: 0 });
    expect(compareAtSites(text.replace('dos', 'dxs'), text, [])).not.toHaveProperty('withinBlock');
  });

  it('is true when the damage stays in the block of any one site', () => {
    const saved = text.replace('tres', 'trxs');
    expect(compareAtSites(saved, text, [0, text.indexOf('Three')]).withinBlock).toBe(true);
    expect(compareAtSites(saved, text, [text.indexOf('Three'), 0]).withinBlock).toBe(true);
    expect(compareAtSites(saved, text, [0, text.indexOf('Two')]).withinBlock).toBe(false);
  });

  it('keeps the position, the size and the hunk of the first comparison', () => {
    const saved = text.replace('tres', 'trxs');
    const result = compareAtSites(saved, text, [0, text.indexOf('Three')]);
    expect(result).toMatchObject({ identical: false, firstDiff: { line: 8, column: 3 }, extraBytes: 1 });
    expect(result.hunk).toContain('-tres');
  });

  it('is false when the damage spans the blocks of two sites', () => {
    const saved = text.replace('uno\n\nTwo\ndos\n\nThree', 'uno\nThree');
    expect(compareAtSites(saved, text, [0, text.indexOf('Three')]).withinBlock).toBe(false);
  });

  // One case per multi-site kind, on a small text and on the corpus file that carries both.
  const corpus = readManifest().files.find((file) => file.path === 'records/lore/beliefs/B-901.md');
  const corpusEdit = <K extends ScriptedEdit['kind']>(id: string): ScriptedEdit & { kind: K } => {
    const found = corpus?.edits.find((candidate) => candidate.id === id);
    if (found === undefined) throw new Error(`B-901 has no edit ${id}`);
    return found as ScriptedEdit & { kind: K };
  };
  const small = {
    text,
    external: {
      id: 'e', kind: 'external', local: { at: { find: 'Three', offset: 5 }, text: ' L' },
      external: { at: { find: 'One', offset: 3 }, text: ' E' },
    } as ScriptedEdit & { kind: 'external' },
    undoAll: {
      id: 'u', kind: 'undo-all',
      edits: [
        { id: 'u1', kind: 'insert', at: { find: 'One' }, text: 'A ' },
        { id: 'u2', kind: 'delete', at: { find: 'Two' }, length: 1 },
        { id: 'u3', kind: 'insert', at: { find: 'Three', offset: 5 }, text: ' B' },
      ],
    } as ScriptedEdit & { kind: 'undo-all' },
  };
  const real = {
    text: corpus === undefined ? '' : toLf(readCorpusText(corpus)),
    external: corpusEdit<'external'>('external-while-dirty'),
    undoAll: corpusEdit<'undo-all'>('undo-all'),
  };
  const withInsert = (from: string, at: number, insert: string): string => from.slice(0, at) + insert + from.slice(at);

  for (const [name, { text: original, external, undoAll }] of Object.entries({ small, real })) {
    describe(`on the ${name} text`, () => {
      it('undo-all: one step left behind is block damage, two steps in two blocks are not', () => {
        // The inserts of the edit, the one that comes later in the text first.
        const where = sites(original, undoAll);
        const inserts = undoAll.edits
          .flatMap((step, index) => (step.kind === 'insert' ? [{ at: where[index] ?? -1, text: step.text }] : []))
          .sort((a, b) => b.at - a.at);
        const [later, earlier] = inserts;
        if (later === undefined || earlier === undefined) throw new Error('expected two inserts');
        expect(later.at).toBeGreaterThan(earlier.at);

        const one = withInsert(original, later.at, later.text);
        expect(compareAtSites(one, original, expectedSites(original, undoAll)).withinBlock).toBe(true);
        // With the single edit offset an undo-all has none, so the damage was never labelled block.
        expect(compareBytes(one, original, editOffset(original, undoAll)).withinBlock).toBeUndefined();

        const two = withInsert(one, earlier.at, earlier.text);
        expect(compareAtSites(two, original, expectedSites(original, undoAll)).withinBlock).toBe(false);
      });

      it('external, after the edit: damage at either insert is block damage', () => {
        const expected = expectedAfter(original, external);
        const [local = -1, other = -1] = expectedSites(original, external);
        for (const at of [local, other]) {
          const saved = `${expected.slice(0, at)}#${expected.slice(at + 1)}`;
          expect(compareAtSites(saved, expected, expectedSites(original, external)).withinBlock).toBe(true);
        }
        // The local insert is in another block than the smaller anchor, which `editOffset` points at.
        const damaged = `${expected.slice(0, local)}#${expected.slice(local + 1)}`;
        expect(compareBytes(damaged, expected, editOffset(original, external)).withinBlock).toBe(false);
      });

      it('external, after one undo: the local text left behind is block damage', () => {
        const expected = expectedAfterExternalUndo(original, external);
        const leftBehind = expectedAfter(original, external);
        expect(compareAtSites(leftBehind, expected, expectedSites(original, external, 'undo')).withinBlock).toBe(true);
        // The external anchor alone does not reach the local insert's block.
        const externalAt = resolveAnchor(original, external.external.at);
        expect(compareBytes(leftBehind, expected, externalAt).withinBlock).toBe(false);
      });
    });
  }
});
