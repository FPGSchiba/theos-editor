import { describe, expect, it } from 'vitest';
import { LINK_RANGE, PROSE, PROSE_MID, rangeOf, type CommentSession } from './shared.ts';

/**
 * The seven behaviours, written once and run against both editors, so both sides are probed with
 * the same tests. `make` builds a fresh session on the corpus file.
 */
export function commentBehaviours(make: () => CommentSession): void {
  // `inBlock`: text of the same block stands before the range, so a deletion over its start stays in one block.
  for (const [label, quote, inBlock] of [['(a) prose', PROSE, false], ['(a2) prose inside its paragraph', PROSE_MID, true], ['(b) link', LINK_RANGE, true]] as const) {
    const { from, to } = rangeOf(quote);
    const session = (): CommentSession => {
      const s = make();
      s.add({ id: 'c1', from, to });
      return s;
    };

    describe(`range ${label}`, () => {
      // AC: CE-06.3
      it('1. adding and removing leave the file text byte-identical, and list gives the offsets', () => {
        const s = make();
        const before = s.text();
        s.add({ id: 'c1', from, to });
        expect(s.text()).toBe(before);
        expect(s.list()).toEqual([{ id: 'c1', from, to }]);
        s.remove('c1');
        expect(s.text()).toBe(before);
        expect(s.list()).toEqual([]);
      });

      // AC: CE-06.3
      it('2a. an insertion before the range shifts it', () => {
        const s = session();
        s.edit(from - 10, from - 10, 'XYZ');
        expect(s.list()).toEqual([{ id: 'c1', from: from + 3, to: to + 3 }]);
      });

      // AC: CE-06.3
      it('2b. an insertion inside the range grows it', () => {
        const s = session();
        s.edit(from + 4, from + 4, 'XYZ');
        expect(s.list()).toEqual([{ id: 'c1', from, to: to + 3 }]);
      });

      // AC: CE-06.3
      it('2c. a deletion over the end shrinks it', () => {
        const s = session();
        s.edit(to - 3, to + 2, '');
        const [c] = s.list();
        expect(c?.from).toBe(from);
        expect(c?.to).toBeLessThan(to);
        expect(c?.to).toBeGreaterThan(from);
      });

      // Only where the start of the range is not the start of its block (see the limits tests for the other case).
      if (inBlock) {
        // AC: CE-06.3
        it('2d. a deletion over the start shrinks it', () => {
          const s = session();
          s.edit(from - 4, from + 3, '');
          const [c] = s.list();
          expect(c?.from).toBe(from - 4);
          expect(c?.to).toBe(to - 7);
        });
      }

      // AC: CE-06.3
      it('2e. typing at the exact end does not extend it', () => {
        const s = session();
        s.edit(to, to, 'XYZ');
        expect(s.list()).toEqual([{ id: 'c1', from, to }]);
      });

      // AC: CE-06.3
      it('2f. start edge rule: typing at the exact start shifts the range, it does not grow it', () => {
        const s = session();
        s.edit(from, from, 'XYZ');
        expect(s.list()).toEqual([{ id: 'c1', from: from + 3, to: to + 3 }]);
      });

      // AC: CE-06.3
      it('3. deleting all the commented text drops the comment; undo brings back text and comment', () => {
        const s = session();
        const before = s.text();
        s.edit(from, to, '');
        expect(s.list()).toEqual([]);
        s.undo();
        expect(s.text()).toBe(before);
        expect(s.list()).toEqual([{ id: 'c1', from, to }]);
        s.redo();
        expect(s.list()).toEqual([]);
      });

      // AC: CE-06.3
      it('4. undo of an add removes the comment, redo restores it', () => {
        const s = session();
        s.undo();
        expect(s.list()).toEqual([]);
        s.redo();
        expect(s.list()).toEqual([{ id: 'c1', from, to }]);
      });

      // AC: CE-06.3
      it('5. a change from outside the editor maps the comment like any other change', () => {
        const s = session();
        s.external(from - 10, from - 10, 'XYZ');
        expect(s.list()).toEqual([{ id: 'c1', from: from + 3, to: to + 3 }]);
        s.external(from + 7, from + 7, 'QQ');
        expect(s.list()).toEqual([{ id: 'c1', from: from + 3, to: to + 5 }]);
      });

      // AC: CE-06.3
      it('6. round trip: list, a fresh state from the same text, restore, same offsets', () => {
        const s = session();
        const saved = s.list();
        const fresh = make();
        fresh.restore(saved);
        expect(fresh.list()).toEqual(saved);
        expect(fresh.text()).toBe(make().text());
      });
    });
  }

  describe('7. the link range behaves like the prose range', () => {
    // AC: CE-06.3
    it('through an edit before it and an edit inside it', () => {
      const results = [PROSE, LINK_RANGE].map((quote) => {
        const { from, to } = rangeOf(quote);
        const s = make();
        s.add({ id: 'c', from, to });
        s.edit(from - 10, from - 10, 'XYZ');
        s.edit(from + 3 + 4, from + 3 + 4, 'QQ');
        const [c] = s.list();
        return { from: (c?.from ?? 0) - from, to: (c?.to ?? 0) - to };
      });
      expect(results).toEqual([{ from: 3, to: 5 }, { from: 3, to: 5 }]);
    });
  });
}
