import type { Node } from 'prosemirror-model';
import { Plugin, TextSelection, type Command } from 'prosemirror-state';
import { keymap } from 'prosemirror-keymap';
import { Decoration, DecorationSet } from 'prosemirror-view';

/**
 * Live preview of callouts and `Design note:` lines. The title line of a callout is drawn as a
 * head; while the selection touches it the head is hidden and the raw title shows (`is-raw` on the
 * title, `has-raw-title` on the callout). A paragraph that starts with `Design note:` gets the
 * class `tu-note`. Also the keys that reach the title, which the browser's caret cannot do
 * because the hidden title has no layout: Backspace, Left or Up at the start of the first
 * block, and Enter in the title.
 */

function build(doc: Node, from: number, to: number): DecorationSet {
  const found: Decoration[] = [];
  doc.descendants((node, pos) => {
    if (node.type.name === 'callout') {
      const end = pos + 1 + node.firstChild!.nodeSize;
      if (from <= end && to >= pos + 1) {
        found.push(Decoration.node(pos + 1, end, { class: 'is-raw' }));
        found.push(Decoration.node(pos, pos + node.nodeSize, { class: 'has-raw-title' }));
      }
      return true;
    }
    if (node.type.name === 'paragraph' && node.textContent.startsWith('Design note:')) {
      found.push(Decoration.node(pos, pos + node.nodeSize, { class: 'tu-note' }));
    }
    return !node.isTextblock;
  });
  return DecorationSet.create(doc, found);
}

/** Move the caret to the end of the title when the selection is an empty caret at the start of its first block. */
const toTitle = (when: (view: Parameters<Command>[2]) => boolean): Command => (state, dispatch, view) => {
  const { selection } = state;
  if (!(selection instanceof TextSelection) || !selection.empty || selection.$head.parentOffset !== 0) return false;
  const $head = selection.$head;
  if ($head.depth < 2 || $head.index($head.depth - 1) !== 1 || $head.node($head.depth - 1).type.name !== 'callout') return false;
  if (!when(view)) return false;
  // The end of the title's text: the title closes right before this block.
  dispatch?.(state.tr.setSelection(TextSelection.create(state.doc, $head.before() - 1)).scrollIntoView());
  return true;
};

/** Enter in the title goes on to the text (a new empty paragraph when there is none). */
const leaveTitle: Command = (state, dispatch) => {
  const { $head } = state.selection;
  if ($head.parent.type.name !== 'callout_title') return false;
  const after = $head.after();
  if (dispatch === undefined) return true;
  if ($head.node(-1).childCount > 1) {
    dispatch(state.tr.setSelection(TextSelection.near(state.doc.resolve(after + 1))).scrollIntoView());
  } else {
    const tr = state.tr.insert(after, state.schema.nodes.paragraph!.create());
    dispatch(tr.setSelection(TextSelection.create(tr.doc, after + 1)).scrollIntoView());
  }
  return true;
};

export const calloutRaw = (): Plugin[] => [
  keymap({
    Enter: leaveTitle,
    Backspace: toTitle(() => true),
    ArrowLeft: toTitle(() => true),
    ArrowUp: toTitle((view) => view?.endOfTextblock('up') ?? true),
  }),
  new Plugin({ props: { decorations: (state) => build(state.doc, state.selection.from, state.selection.to) } }),
];
