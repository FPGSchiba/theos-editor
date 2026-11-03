import type { Node } from 'prosemirror-model';
import { Plugin, TextSelection, type Command } from 'prosemirror-state';
import { keymap } from 'prosemirror-keymap';
import { Decoration, DecorationSet } from 'prosemirror-view';
import { readHeading } from './beats.ts';

/**
 * Live preview of beats: the heading and the attribute lines show raw (`is-raw`, and `## ` before
 * the heading) while the selection touches them. Their raw text is hidden otherwise, and a hidden
 * block has no browser caret, so the keys that step into one are commands: arrows, Backspace and
 * Delete at a block edge move the caret to the near end of the beat block; Enter in a heading goes on.
 */

const isBeat = (node: Node | null | undefined): node is Node =>
  node !== null && node !== undefined && (node.type.name === 'beat_heading' || (node.type.name === 'beat_attrs' && node.content.size > 0));
const isEmptyAttrs = (node: Node): boolean => node.type.name === 'beat_attrs' && node.content.size === 0;

function marker(): HTMLElement {
  const span = document.createElement('span');
  span.className = 'pm-marker';
  span.textContent = '## ';
  return span;
}

function build(doc: Node, from: number, to: number): DecorationSet {
  const found: Decoration[] = [];
  doc.forEach((node, pos) => {
    if (!isBeat(node)) return;
    const raw = from <= pos + node.nodeSize - 1 && to >= pos + 1;
    if (raw) found.push(Decoration.node(pos, pos + node.nodeSize, { class: 'is-raw' }));
    if (node.type.name === 'beat_heading' && (raw || readHeading(node.textContent)?.dash !== true)) {
      found.push(Decoration.widget(pos + 1, marker, { side: -1, key: `beat-hash@${pos}` }));
    }
  });
  return DecorationSet.create(doc, found);
}

/** Step from a caret at a block edge into the neighbouring beat block (empty attribute blocks are skipped). */
const step = (forward: boolean, vertical: boolean): Command => (state, dispatch, view) => {
  const { selection } = state;
  if (!(selection instanceof TextSelection) || !selection.empty || selection.$head.depth !== 1) return false;
  const $head = selection.$head;
  if ($head.parentOffset !== (forward ? $head.parent.content.size : 0)) return false;
  if (vertical && !(view?.endOfTextblock(forward ? 'down' : 'up') ?? true)) return false;
  const { doc } = state;
  let index = $head.index(0) + (forward ? 1 : -1);
  while (doc.maybeChild(index) !== null && isEmptyAttrs(doc.child(index))) index += forward ? 1 : -1;
  const next = doc.maybeChild(index);
  if (!isBeat(next)) return false;
  let before = 0;
  for (let i = 0; i < index; i++) before += doc.child(i).nodeSize;
  dispatch?.(state.tr.setSelection(TextSelection.create(doc, forward ? before + 1 : before + next.nodeSize - 1)).scrollIntoView());
  return true;
};

/** Enter in a heading goes on to the attribute lines, or to a new paragraph behind the beat block. */
const leaveHeading: Command = (state, dispatch) => {
  const { $head } = state.selection;
  if ($head.parent.type.name !== 'beat_heading' || $head.depth !== 1) return false;
  if (dispatch === undefined) return true;
  const { doc } = state;
  const index = $head.index(0);
  if (isBeat(doc.maybeChild(index + 1))) {
    dispatch(state.tr.setSelection(TextSelection.create(doc, $head.after() + 1)).scrollIntoView());
    return true;
  }
  let after = $head.after();
  if (doc.maybeChild(index + 1) !== null && isEmptyAttrs(doc.child(index + 1))) after += doc.child(index + 1).nodeSize;
  const tr = state.tr.insert(after, state.schema.nodes.paragraph!.create());
  dispatch(tr.setSelection(TextSelection.create(tr.doc, after + 1)).scrollIntoView());
  return true;
};

export const beatRaw = (): Plugin[] => [
  keymap({
    Enter: leaveHeading,
    ArrowRight: step(true, false),
    ArrowDown: step(true, true),
    Delete: step(true, false),
    ArrowLeft: step(false, false),
    ArrowUp: step(false, true),
    Backspace: step(false, false),
  }),
  new Plugin({
    props: { decorations: (state) => build(state.doc, state.selection.from, state.selection.to) },
  }),
];
