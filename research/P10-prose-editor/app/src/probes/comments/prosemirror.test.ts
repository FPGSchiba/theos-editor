import { describe, expect, it } from 'vitest';
import { PositionMap } from '../../adapters/prosemirror/positions.ts';
import { commentBehaviours } from './behaviours.ts';
import { ProseMirrorComments } from './prosemirror.ts';
import { FILE, PROSE, rangeOf } from './shared.ts';

const make = (): ProseMirrorComments => new ProseMirrorComments(FILE);

describe('ProseMirror comments', () => {
  commentBehaviours(make);

  describe('limits', () => {
    // AC: CE-06.3
    it('the stock position map is exact for the corpus file', () => {
      const s = make();
      const map = new PositionMap(s.sourceText(), FILE.indexOf('A link to a record'), s.state.doc);
      expect(map.exact).toBe(true);
    });

    // AC: CE-06.3
    it('observed: the saved text is the stock serializer output, not the file (it was already so before any comment)', () => {
      const text = make().text();
      expect(text).not.toBe(FILE);
      expect(text).toContain('*[[C-033|the operator-plate clue]]*');
    });

    // AC: CE-06.3
    it('a range across a block boundary', () => {
      const s = make();
      const from = FILE.indexOf('ancestors');
      const to = FILE.indexOf('The alias form') + 3;
      s.add({ id: 'x', from, to });
      expect(s.list()).toEqual([{ id: 'x', from, to }]);
    });

    // AC: CE-06.3
    it('a range that starts inside the link syntax snaps to the start of the chip (observed)', () => {
      const s = make();
      const chip = FILE.indexOf('[[fates-shrine]]');
      s.add({ id: 'x', from: chip + 5, to: chip + 20 });
      expect(s.list()).toEqual([{ id: 'x', from: chip, to: chip + 20 }]);
    });

    // AC: CE-06.3
    it('a range that lies entirely inside the link syntax is lost: both ends snap to the chip (observed)', () => {
      const s = make();
      const chip = FILE.indexOf('[[fates-shrine]]');
      s.add({ id: 'x', from: chip + 5, to: chip + 10 });
      expect(s.list()).toEqual([]);
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
