import { Plugin, PluginKey, TextSelection, type EditorState, type Transaction } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';
import { BEAT_KEY_ORDER, type LinkPlace } from '../../harness/types.ts';

/**
 * A link that is being typed: `[[` and what follows up to the caret. It stays plain text (marked
 * like the reference screen's typing state) and no chip is drawn for it; the popover offers the
 * records. The text only becomes a link node when an option is picked or the `]]` is typed.
 */

/** The longest query that still counts as a search; a longer text behind `[[` is just text. */
const MAX_QUERY = 80;
const BEAT_KEY = /(?:^|[\s\uFFFC])(at|with|says|reveals|when)::/g;

export interface Typing {
  /** Where the `[[` starts, and where the typed text ends (the caret). */
  from: number;
  to: number;
  query: string;
  /** Escape was pressed: the typed text stays plain text and the popover stays away. */
  dismissed: boolean;
}

export const typingKey = new PluginKey<Typing | undefined>('p10-typing');

/** The text of the textblock before the caret, a link counting as one character. */
function textBefore(state: EditorState): { text: string; start: number } | undefined {
  const { selection } = state;
  if (!(selection instanceof TextSelection) || !selection.empty) return undefined;
  const $head = selection.$head;
  if (!$head.parent.isTextblock || $head.parent.type.spec.code === true) return undefined;
  return { text: $head.parent.textBetween(0, $head.parentOffset, undefined, '￼'), start: $head.start() };
}

function findTrigger(state: EditorState): Pick<Typing, 'from' | 'to' | 'query'> | undefined {
  const before = textBefore(state);
  if (before === undefined) return undefined;
  const open = before.text.lastIndexOf('[[');
  if (open === -1) return undefined;
  const query = before.text.slice(open + 2);
  if (query.length > MAX_QUERY || /[[\]|￼]/.test(query)) return undefined;
  return { from: before.start + open, to: state.selection.head, query };
}

/**
 * The place of a `[[`: after a `key::` of a beat the kinds of that key are allowed, anywhere else
 * is prose. The stock parser turns the line breaks of a paragraph into spaces, so the key is the
 * last `key::` before the caret in the textblock, not the one at the start of a line.
 */
export function placeOf(state: EditorState, at: number): LinkPlace {
  const $at = state.doc.resolve(at);
  const text = $at.parent.textBetween(0, $at.parentOffset, undefined, '￼');
  let key: string | undefined;
  for (const found of text.matchAll(BEAT_KEY)) key = found[1];
  const beat = BEAT_KEY_ORDER.find((candidate) => candidate === key);
  return beat === undefined ? { in: 'prose' } : { in: 'beat', key: beat };
}

/** Typing as the user does it grows the document; undo, redo, paste and external changes do not count. */
function typed(tr: Transaction): boolean {
  return tr.docChanged
    && tr.doc.content.size > tr.before.content.size
    && tr.getMeta('paste') === undefined
    && tr.getMeta('history$') === undefined
    && tr.getMeta('addToHistory') !== false;
}

export const dismissTyping = (state: EditorState): Transaction => state.tr.setMeta(typingKey, 'dismiss');

export function typingPlugin(): Plugin<Typing | undefined> {
  return new Plugin<Typing | undefined>({
    key: typingKey,
    state: {
      init: () => undefined,
      apply(tr, value, _before, state) {
        const dismiss = tr.getMeta(typingKey) === 'dismiss';
        if (!tr.docChanged && !tr.selectionSet && !dismiss) return value;
        const found = findTrigger(state);
        if (found === undefined) return undefined;
        const continues = value !== undefined && tr.mapping.map(value.from, -1) === found.from;
        if (!continues && !typed(tr)) return undefined;
        return { ...found, dismissed: dismiss || (continues && value?.dismissed === true) };
      },
    },
    props: {
      decorations(state) {
        const typing = typingKey.getState(state);
        if (typing === undefined || typing.dismissed) return null;
        return DecorationSet.create(state.doc, [Decoration.inline(typing.from, typing.to, { class: 'tu-typing__q' })]);
      },
    },
  });
}
