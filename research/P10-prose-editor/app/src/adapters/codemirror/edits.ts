import { EditorSelection } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { resolveAnchor } from '../../harness/anchor.ts';
import type { ScriptedEdit } from '../../harness/types.ts';
import { bodyStart } from './frontmatter.ts';
import { setNoteState } from './notestate.ts';
import { completeLink } from './typing.ts';

/** The select-all of the keymap: the body only, the front matter is not the author's text. */
export function selectBody(view: EditorView): boolean {
  const { state } = view;
  view.dispatch({ selection: EditorSelection.range(state.field(bodyStart), state.doc.length), userEvent: 'select' });
  return true;
}

/** A change at `from`..`to` with the caret behind it, as a transaction with the user event of typing. */
function change(view: EditorView, from: number, to: number, insert: string, userEvent: string): void {
  view.dispatch({
    changes: { from, to, insert },
    selection: { anchor: from + view.state.toText(insert).length },
    userEvent,
    scrollIntoView: true,
  });
}

/**
 * The single-site edit kinds, made through the editor's transactions. The front matter filter
 * sees them like typing: a backward delete that reaches into it is cut at the body start.
 */
export function applyEdit(view: EditorView, edit: ScriptedEdit): void {
  const text = view.state.doc.toString();
  switch (edit.kind) {
    case 'insert': {
      const at = resolveAnchor(text, edit.at);
      change(view, at, at, edit.text, 'input.type');
      return;
    }
    case 'paste': {
      // The clipboard's plain text; `toText` makes its line ends LF, as the paste handler does.
      const at = resolveAnchor(text, edit.at);
      change(view, at, at, edit.plain, 'input.paste');
      return;
    }
    case 'delete': {
      const at = resolveAnchor(text, edit.at);
      if (edit.backward === true) change(view, Math.max(at - edit.length, 0), at, '', 'delete.backward');
      else change(view, at, Math.min(at + edit.length, text.length), '', 'delete.forward');
      return;
    }
    case 'select-all-type': {
      selectBody(view);
      view.dispatch(view.state.update(view.state.replaceSelection(edit.text), { userEvent: 'input.type', scrollIntoView: true }));
      return;
    }
    case 'link-insert': {
      // As the author does it: the typed `[[query` first, then the pick takes its place.
      const at = resolveAnchor(text, edit.at);
      change(view, at, at, `[[${edit.query}`, 'input.type');
      completeLink(view, at, at + 2 + edit.query.length, edit.pick);
      return;
    }
    case 'link-delete': {
      // The whole link goes in one step, as one Backspace behind its chip does.
      const from = resolveAnchor(text, edit.link);
      const close = text.indexOf(']]', from + 2);
      if (!text.startsWith('[[', from) || close === -1) throw new Error('link-delete: the anchor is not at a complete link');
      view.dispatch({ changes: { from, to: close + 2 }, selection: { anchor: from }, userEvent: 'delete.backward', scrollIntoView: true });
      return;
    }
    case 'note-state':
      setNoteState(view, edit.note, edit.state);
      return;
    default:
      throw new Error(`codemirror: the base does not make "${edit.kind}" edits yet`);
  }
}
