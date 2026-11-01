import { AllSelection, TextSelection } from 'prosemirror-state';
import type { EditorView } from 'prosemirror-view';
import { resolveAnchor } from '../../harness/anchor.ts';
import type { ScriptedEdit, TextChange } from '../../harness/types.ts';

/** What the scripted edits need from the adapter, in either mode. */
export interface EditTarget {
  view: EditorView;
  /** The text the offsets of anchors refer to. */
  sourceText(): string;
  /** Document position of a source offset. */
  toPos(offset: number): number;
  /** Source offset where the body starts (the length of the front matter). */
  bodyStart(): number;
  /** Whether Write mode is showing (select-all then covers the whole document). */
  writing(): boolean;
  applyExternal(changes: TextChange[]): void;
  canUndo(): boolean;
  undo(): void;
}

/** The select-all of the keymap in Write mode: the body is the whole document. */
export function selectAllBody(view: EditorView): void {
  view.dispatch(view.state.tr.setSelection(new AllSelection(view.state.doc)));
}

function insertAt(target: EditTarget, offset: number, text: string): void {
  const { view } = target;
  const pos = target.toPos(offset);
  const tr = view.state.tr.insertText(text, pos);
  view.dispatch(tr.setSelection(TextSelection.near(tr.doc.resolve(tr.mapping.map(pos, 1)))).scrollIntoView());
}

function deleteRange(target: EditTarget, from: number, to: number): void {
  const { view } = target;
  const tr = view.state.tr.delete(target.toPos(from), target.toPos(to));
  view.dispatch(tr.setSelection(TextSelection.near(tr.doc.resolve(tr.mapping.map(target.toPos(from))))).scrollIntoView());
}

/** The single-site edit kinds, through ProseMirror transactions (never the oracle). */
export function applyEdit(target: EditTarget, edit: ScriptedEdit): void {
  const { view } = target;
  const text = target.sourceText();
  switch (edit.kind) {
    case 'insert':
      insertAt(target, resolveAnchor(text, edit.at), edit.text);
      return;
    case 'paste': {
      // The clipboard's plain text, through the editor's own paste handling.
      const pos = target.toPos(resolveAnchor(text, edit.at));
      view.dispatch(view.state.tr.setSelection(TextSelection.near(view.state.doc.resolve(pos))));
      view.pasteText(edit.plain);
      return;
    }
    case 'delete': {
      const at = resolveAnchor(text, edit.at);
      if (edit.backward === true) deleteRange(target, Math.max(at - edit.length, Math.min(at, target.bodyStart())), at);
      else deleteRange(target, at, Math.min(at + edit.length, text.length));
      return;
    }
    case 'select-all-type': {
      const { state } = view;
      const selection = target.writing()
        ? new AllSelection(state.doc)
        : TextSelection.create(state.doc, Math.min(1 + target.bodyStart(), state.doc.content.size - 1), state.doc.content.size - 1);
      view.dispatch(state.tr.setSelection(selection).insertText(edit.text).scrollIntoView());
      return;
    }
    case 'external': {
      // Both anchors resolve against the original; the local insert may move the external one.
      const local = resolveAnchor(text, edit.local.at);
      const external = resolveAnchor(text, edit.external.at);
      if (local === external) throw new Error('external: the local and the external anchor resolve to the same offset');
      insertAt(target, local, edit.local.text);
      const at = external > local ? external + edit.local.text.length : external;
      target.applyExternal([{ from: at, to: at, insert: edit.external.text }]);
      return;
    }
    case 'undo-all':
      // Each edit is its own step; then everything is taken back, as the user would.
      for (const step of edit.edits) applyEdit(target, step);
      for (let n = 0; n < 50 && target.canUndo(); n++) target.undo();
      return;
    default:
      throw new Error(`prosemirror: the base does not make "${edit.kind}" edits yet`);
  }
}
