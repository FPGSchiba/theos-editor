import { describe, expect, it } from 'vitest';
import { commentBehaviours } from './behaviours.ts';
import { CodeMirrorComments } from './codemirror.ts';
import { FILE, PROSE, rangeOf } from './shared.ts';

const make = (): CodeMirrorComments => new CodeMirrorComments(FILE);

describe('CodeMirror 6 comments', () => {
  commentBehaviours(make);

  describe('limits', () => {
    // AC: CE-06.3
    it('a range across a block boundary needs nothing special (the document is flat text)', () => {
      const s = make();
      const from = FILE.indexOf('ancestors');
      const to = FILE.indexOf('The alias form') + 3;
      s.add({ id: 'x', from, to });
      expect(s.list()).toEqual([{ id: 'x', from, to }]);
    });

    // AC: CE-06.3
    it('a range that starts inside the link syntax is kept as given (the link is plain text)', () => {
      const s = make();
      const from = FILE.indexOf('[[fates-shrine]]') + 5;
      s.add({ id: 'x', from, to: from + 5 });
      expect(s.list()).toEqual([{ id: 'x', from, to: from + 5 }]);
    });

    // AC: CE-06.3
    it('a deletion from the paragraph break into the start of a range at a block start shrinks it', () => {
      const { from, to } = rangeOf(PROSE);
      const s = make();
      s.add({ id: 'x', from, to });
      s.edit(from - 2, from + 3, '');
      expect(s.list()).toEqual([{ id: 'x', from: from - 2, to: to - 5 }]);
    });

    // AC: CE-06.3
    it('undo of a partial deletion over the end does not grow the range back (observed)', () => {
      const { from, to } = rangeOf(PROSE);
      const s = make();
      s.add({ id: 'x', from, to });
      s.edit(to - 3, to, '');
      s.undo();
      expect(s.list()).toEqual([{ id: 'x', from, to: to - 3 }]);
    });
  });
});
