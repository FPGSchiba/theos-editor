import { StateEffect, StateField, type EditorState, type Extension } from '@codemirror/state';
import type { EditorContext } from '../../harness/types.ts';

/** The host's context (index, places, keymap) in the editor state, so the link features read the current one. */
export const setContext = StateEffect.define<EditorContext>();

const contextField = StateField.define<EditorContext>({
  create: () => { throw new Error('codemirror: the editor context is given at creation'); },
  update(ctx, tr) {
    for (const effect of tr.effects) if (effect.is(setContext)) return effect.value;
    return ctx;
  },
});

export const contextExtension = (ctx: EditorContext): Extension => contextField.init(() => ctx);

/** The context the editor holds now. */
export const contextOf = (state: EditorState): EditorContext => state.field(contextField);
