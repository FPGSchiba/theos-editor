import type { Node } from 'prosemirror-model';
import type { EditorState, Transaction } from 'prosemirror-state';
import type { TextChange } from '../../harness/types.ts';
import { EXTERNAL } from './source.ts';

/** The text with every change applied (offsets are into `text`; the last change goes first). */
export function applyChanges(text: string, changes: TextChange[]): string {
  return [...changes]
    .sort((a, b) => b.from - a.from)
    .reduce((acc, change) => acc.slice(0, change.from) + change.insert + acc.slice(change.to), text);
}

/**
 * The minimal replacement that turns the document of `state` into `next`, kept out of the undo
 * history: `prosemirror-history` maps its stored steps through it and the selection is mapped
 * by the transaction, so the caret keeps its text and undo takes back only the local edit.
 * Undefined when the two documents are equal.
 */
export function externalTransaction(state: EditorState, next: Node): Transaction | undefined {
  const start = state.doc.content.findDiffStart(next.content);
  if (start === null) return undefined;
  const end = state.doc.content.findDiffEnd(next.content);
  let endOld = end?.a ?? start;
  let endNew = end?.b ?? start;
  // The start may lie past the end when the change is an insertion next to equal text.
  const overlap = start - Math.min(endOld, endNew);
  if (overlap > 0) {
    endOld += overlap;
    endNew += overlap;
  }
  return state.tr.replace(start, endOld, next.slice(start, endNew)).setMeta('addToHistory', false).setMeta(EXTERNAL, true);
}
