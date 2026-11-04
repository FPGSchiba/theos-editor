import { Transaction } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import type { TextChange } from '../../harness/types.ts';
import { fromOutside } from './frontmatter.ts';

/**
 * The file changed outside the editor. The changes are one transaction that is not added to the
 * history: the history maps its stored steps through it, so undo still takes back only the
 * local edit, and the selection is mapped through it, so the caret stays on its text.
 */
export function applyExternal(view: EditorView, changes: TextChange[]): void {
  view.dispatch({
    changes: changes.map(({ from, to, insert }) => ({ from, to, insert })),
    annotations: [Transaction.addToHistory.of(false), fromOutside.of(true)],
  });
}
