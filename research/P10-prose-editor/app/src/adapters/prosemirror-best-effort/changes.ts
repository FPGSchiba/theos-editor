import type { Node } from 'prosemirror-model';
import type { Transaction } from 'prosemirror-state';

/**
 * The part of the document a transaction (or a run of them) replaced. Positions in front of
 * `start` are the same in both documents; behind the end they are shifted by the size change.
 */
export interface Change {
  start: number;
  /** The end of the replaced range in the document before. */
  endBefore: number;
  /** The end of the replacement in the document after. */
  endAfter: number;
}

/** The minimal range in which two documents differ, undefined when they are equal. */
export function diffOf(before: Node, after: Node): Change | undefined {
  const start = before.content.findDiffStart(after.content);
  if (start === null) return undefined;
  const end = before.content.findDiffEnd(after.content);
  let endBefore = end?.a ?? start;
  let endAfter = end?.b ?? start;
  // The diff end may lie in front of the start when equal text sits next to an insertion.
  const overlap = start - Math.min(endBefore, endAfter);
  if (overlap > 0) {
    endBefore += overlap;
    endAfter += overlap;
  }
  return { start, endBefore, endAfter };
}

/**
 * What a transaction replaced: the ranges its steps replace (a select-all replace covers the
 * whole document, its edges included), joined with the document diff (which also sees mark
 * steps, whose maps are empty).
 */
export function changeOf(tr: Transaction): Change | undefined {
  const diff = diffOf(tr.before, tr.doc);
  if (diff === undefined) return undefined;
  let start = diff.start;
  let endAfter = diff.endAfter;
  tr.mapping.maps.forEach((map, index) => {
    const rest = tr.mapping.slice(index + 1);
    map.forEach((_oldStart, _oldEnd, newStart, newEnd) => {
      start = Math.min(start, rest.map(newStart, -1));
      endAfter = Math.max(endAfter, rest.map(newEnd, 1));
    });
  });
  const grown = tr.doc.content.size - tr.before.content.size;
  return { start, endBefore: endAfter - grown, endAfter };
}

/**
 * `group` (before → the document `tr` starts from) followed by `next` (the change of `tr`), as
 * one change, when the two touch; undefined when they lie apart. Typing a word, or `[[query`
 * and the pick that replaces it, is one change: no source text lies between its steps.
 */
export function joinChanges(group: Change, tr: Transaction, next: Change): Change | undefined {
  if (next.start > group.endAfter || next.endBefore < group.start) return undefined;
  return {
    start: Math.min(group.start, next.start),
    endBefore: next.endBefore > group.endAfter ? next.endBefore - (group.endAfter - group.endBefore) : group.endBefore,
    endAfter: Math.max(tr.mapping.map(group.endAfter, 1), next.endAfter),
  };
}
