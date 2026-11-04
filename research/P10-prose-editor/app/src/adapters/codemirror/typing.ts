import { StateEffect, StateField, type EditorState } from '@codemirror/state';
import { Decoration, EditorView } from '@codemirror/view';
import { BEAT_KEY_ORDER, type LinkPlace } from '../../harness/types.ts';

/**
 * A link that is being typed: `[[` and what follows up to the caret. It shows raw (marked like the
 * reference screen's typing state) and no chip is drawn over it; the popover offers the records.
 */

/** The longest query that still counts as a search; a longer text behind `[[` is just text. */
const MAX_QUERY = 80;
const BEAT_LINE = /^(at|with|says|reveals|when)::/;

export interface Typing {
  /** Where the `[[` starts, and the caret where the typed text ends. */
  from: number;
  to: number;
  query: string;
  /** Escape was pressed: the typed text stays plain text and the popover stays away. */
  dismissed: boolean;
}

/** The `[[query` that ends at the caret (no `]`, `[` or `|` in the query: those end the search). */
function findTrigger(state: EditorState): Pick<Typing, 'from' | 'to' | 'query'> | undefined {
  const caret = state.selection.main;
  if (!caret.empty) return undefined;
  const line = state.doc.lineAt(caret.head);
  const before = line.text.slice(0, caret.head - line.from);
  const open = before.lastIndexOf('[[');
  if (open === -1) return undefined;
  const query = before.slice(open + 2);
  if (query.length > MAX_QUERY || /[[\]|]/.test(query)) return undefined;
  return { from: line.from + open, to: caret.head, query };
}

/** The place of a `[[`: a beat's `key::` line allows the kinds of that key, anywhere else is prose. */
export function placeOf(state: EditorState, at: number): LinkPlace {
  const key = BEAT_LINE.exec(state.doc.lineAt(at).text)?.[1];
  const beat = BEAT_KEY_ORDER.find((candidate) => candidate === key);
  return beat === undefined ? { in: 'prose' } : { in: 'beat', key: beat };
}

export const dismissTyping = StateEffect.define<null>();

/**
 * Typing starts with typed text that completes a `[[`; it goes on while the caret stays in the
 * same `[[query`, and ends when the text no longer has that shape (or the caret leaves it).
 */
export const typingField = StateField.define<Typing | undefined>({
  create: () => undefined,
  update(value, tr) {
    const dismiss = tr.effects.some((effect) => effect.is(dismissTyping));
    if (!tr.docChanged && !tr.selection && !dismiss) return value;
    const found = findTrigger(tr.state);
    if (found === undefined) return undefined;
    const continues = value !== undefined && tr.changes.mapPos(value.from, -1) === found.from;
    if (!continues && !(tr.docChanged && tr.isUserEvent('input.type'))) return undefined;
    return { ...found, dismissed: dismiss || (continues && value?.dismissed === true) };
  },
  provide: (field) =>
    EditorView.decorations.from(field, (typing) =>
      typing === undefined || typing.dismissed
        ? Decoration.none
        : Decoration.set(Decoration.mark({ class: 'tu-typing__q' }).range(typing.from, typing.to))),
});

/** Replace the typed `[[query` (or nothing, from = to) with the finished link; the caret goes behind it. */
export function completeLink(view: EditorView, from: number, to: number, id: string): void {
  const insert = `[[${id}]]`;
  view.dispatch({
    changes: { from, to, insert },
    selection: { anchor: from + insert.length },
    userEvent: 'input.complete',
    scrollIntoView: true,
  });
}
