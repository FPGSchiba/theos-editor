import { history, invertedEffects, redo, undo } from '@codemirror/commands';
import { EditorState, StateEffect, StateField, Transaction, type Range } from '@codemirror/state';
import { Decoration, EditorView, type DecorationSet } from '@codemirror/view';
import type { Comment, CommentSession } from './shared.ts';

/**
 * Comments on text ranges in CodeMirror 6. The state's document is the whole file, so a file
 * offset is a document position. The comments are a `DecorationSet` of `Decoration.mark` ranges
 * (class `tu-comment`, id in the spec) in a `StateField`; the library maps it through every
 * change. Edge rule: a mark is not inclusive, so text typed at its start stays outside it (the
 * range shifts) and text typed at its end stays outside it too.
 */

const addEffect = StateEffect.define<Comment>({
  map: (c, changes) => ({ id: c.id, from: changes.mapPos(c.from, 1), to: changes.mapPos(c.to, -1) }),
});
const removeEffect = StateEffect.define<string>();

const mark = (c: Comment): Range<Decoration> => Decoration.mark({ class: 'tu-comment', commentId: c.id }).range(c.from, c.to);
const idOf = (value: Decoration): string => (value.spec as { commentId: string }).commentId;

/** The comments of a set, as file offsets. */
function commentsOf(set: DecorationSet): Comment[] {
  const found: Comment[] = [];
  for (const cursor = set.iter(); cursor.value !== null; cursor.next()) found.push({ id: idOf(cursor.value), from: cursor.from, to: cursor.to });
  return found;
}

/** The comments, mapped through every change; a comment whose text was deleted entirely is dropped. */
export const commentField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(set, tr) {
    let next = set.map(tr.changes).update({ filter: (from, to) => from < to });
    for (const effect of tr.effects) {
      if (effect.is(addEffect)) next = next.update({ add: [mark(effect.value)] });
      else if (effect.is(removeEffect)) next = next.update({ filter: (_from, _to, value) => idOf(value) !== effect.value });
    }
    return next;
  },
  provide: (field) => EditorView.decorations.from(field),
});

/** What undoes a transaction's comment changes, in the coordinates of the document before it. */
const invert = invertedEffects.of((tr) => {
  const before = commentsOf(tr.startState.field(commentField));
  const after = new Set(commentsOf(tr.state.field(commentField)).map((c) => c.id));
  const inverse: StateEffect<unknown>[] = [];
  for (const effect of tr.effects) if (effect.is(addEffect)) inverse.push(removeEffect.of(effect.value.id));
  // Every comment that was there before and is gone now (removed, or its text deleted) comes back by undo.
  for (const c of before) if (!after.has(c.id)) inverse.push(addEffect.of(c));
  return inverse;
});

/** An editor state for the whole file with the comment field and an undo history. */
export function createState(text: string): EditorState {
  return EditorState.create({ doc: text, extensions: [history(), commentField, invert] });
}

/** A comment session on an `EditorState`, driven the way a view would, without one. */
export class CodeMirrorComments implements CommentSession {
  state: EditorState;

  constructor(text: string) {
    this.state = createState(text);
  }

  text(): string {
    return this.state.doc.toString();
  }

  add(comment: Comment): void {
    this.state = this.state.update({ effects: addEffect.of(comment) }).state;
  }

  remove(id: string): void {
    this.state = this.state.update({ effects: removeEffect.of(id) }).state;
  }

  list(): Comment[] {
    return commentsOf(this.state.field(commentField));
  }

  restore(comments: Comment[]): void {
    this.state = this.state.update({ effects: comments.map((c) => addEffect.of(c)), annotations: Transaction.addToHistory.of(false) }).state;
  }

  edit(from: number, to: number, insert: string): void {
    this.state = this.state.update({ changes: { from, to, insert }, userEvent: 'input.type' }).state;
  }

  external(from: number, to: number, insert: string): void {
    this.state = this.state.update({ changes: { from, to, insert }, annotations: Transaction.addToHistory.of(false) }).state;
  }

  undo(): void {
    undo({ state: this.state, dispatch: (tr) => (this.state = tr.state) });
  }

  redo(): void {
    redo({ state: this.state, dispatch: (tr) => (this.state = tr.state) });
  }
}
