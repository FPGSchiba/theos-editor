import { history, redo, undo, undoDepth } from 'prosemirror-history';
import { EditorState, type Plugin } from 'prosemirror-state';

export { history, redo, undo };

/** Local edits that can be taken back. */
export const canUndo = (state: EditorState): boolean => undoDepth(state) > 0;

/**
 * ProseMirror has no call that clears the history (prosemirror issue 1042), so the state is
 * created again with the same document, selection and fresh plugins.
 */
export function withoutHistory(state: EditorState, plugins: Plugin[]): EditorState {
  return EditorState.create({ doc: state.doc, selection: state.selection, plugins });
}
