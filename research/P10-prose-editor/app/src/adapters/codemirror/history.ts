import { history, redo, undo, undoDepth } from '@codemirror/commands';
import { Compartment, type EditorState } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';

export { redo, undo };

/** The history sits in a compartment because CodeMirror has no call that clears it. */
const slot = new Compartment();

export const historyExtension = slot.of(history());

/** Local edits that can be taken back. */
export const canUndo = (state: EditorState): boolean => undoDepth(state) > 0;

/** Forget every step: take the history out of the configuration and put a fresh one back. */
export function clearHistory(view: EditorView): void {
  view.dispatch({ effects: slot.reconfigure([]) });
  view.dispatch({ effects: slot.reconfigure(history()) });
}
