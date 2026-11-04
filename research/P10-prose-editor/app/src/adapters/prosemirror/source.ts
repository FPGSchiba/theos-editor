import { EditorState, Plugin, TextSelection, type Command, type Transaction } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';
import { sourceSchema } from './schema.ts';

/**
 * Source mode is the same view with a second document: one block that holds the whole file as
 * text, so an offset of the file is a position minus one. The front matter is shown raw and
 * protected by a transaction filter, in the same component as Write mode.
 */

/** A transaction with this meta comes from outside (the file changed); the filter lets it pass. */
export const EXTERNAL = 'p10External';

export const sourceDoc = (text: string) =>
  sourceSchema.node('doc', null, [sourceSchema.node('source', null, text === '' ? [] : [sourceSchema.text(text)])]);

/** Select everything behind the front matter. */
export const selectSourceBody = (protectedEnd: number): Command => (state, dispatch) => {
  const end = state.doc.content.size - 1;
  const selection = TextSelection.create(state.doc, Math.min(1 + protectedEnd, end), end);
  if (dispatch) dispatch(state.tr.setSelection(selection));
  return true;
};

/** Rejects a transaction that changes anything in the first `protectedEnd` characters. */
export function guard(protectedEnd: number): Plugin {
  return new Plugin({
    filterTransaction(tr: Transaction) {
      if (!tr.docChanged || tr.getMeta(EXTERNAL) === true) return true;
      let allowed = true;
      for (const step of tr.steps) {
        step.getMap().forEach((from) => {
          if (from < 1 + protectedEnd) allowed = false;
        });
      }
      return allowed;
    },
    props: {
      decorations: (state) =>
        protectedEnd === 0 || state.doc.content.size < 2 + protectedEnd
          ? DecorationSet.empty
          : DecorationSet.create(state.doc, [Decoration.inline(1, 1 + protectedEnd, { class: 'pm-frontmatter' })]),
    },
  });
}

/** The Source mode state of `text`, with the caret at source offset `selection`. */
export function sourceState(text: string, plugins: Plugin[], selection: number): EditorState {
  const doc = sourceDoc(text);
  const at = Math.min(Math.max(selection, 0), text.length) + 1;
  return EditorState.create({ doc, plugins, selection: TextSelection.create(doc, at) });
}
