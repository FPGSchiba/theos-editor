import { Plugin, TextSelection, type Command } from 'prosemirror-state';

/**
 * Chips are atoms: the caret steps over one in a press, Backspace right behind one and Delete
 * right before one take the whole link. Done as commands, so no engine's native caret rules matter.
 */

function nextLink(dir: 1 | -1, state: Parameters<Command>[0]) {
  const { selection: current } = state;
  if (!(current instanceof TextSelection) || !current.empty) return undefined;
  const node = dir > 0 ? current.$head.nodeAfter : current.$head.nodeBefore;
  return node?.type.name === 'wikilink' ? { pos: current.head, size: node.nodeSize } : undefined;
}

export const stepOverLink = (dir: 1 | -1): Command => (state, dispatch) => {
  const link = nextLink(dir, state);
  if (link === undefined) return false;
  dispatch?.(state.tr.setSelection(TextSelection.create(state.doc, link.pos + dir * link.size)).scrollIntoView());
  return true;
};

export const deleteLink = (dir: 1 | -1): Command => (state, dispatch) => {
  const link = nextLink(dir, state);
  if (link === undefined) return false;
  const from = dir > 0 ? link.pos : link.pos - link.size;
  dispatch?.(state.tr.delete(from, from + link.size).scrollIntoView());
  return true;
};

/** A link typed by hand becomes a chip when the closing `]]` is complete; until then it is raw text. */
const TYPED = /\[\[([^[\]|\n￼]+)(?:\|([^[\]\n￼]+))?\]$/;

export const finishTypedLink = (): Plugin =>
  new Plugin({
    props: {
      handleTextInput(view, from, to, text) {
        if (text !== ']' || from !== to) return false;
        const $pos = view.state.doc.resolve(from);
        if (!$pos.parent.isTextblock || $pos.parent.type.spec.code === true) return false;
        const before = $pos.parent.textBetween(0, $pos.parentOffset, undefined, '￼');
        const found = TYPED.exec(before);
        if (found === null) return false;
        const node = view.state.schema.nodes.wikilink!.create({ id: found[1], shown: found[2] ?? null });
        const start = $pos.start() + found.index;
        const tr = view.state.tr.replaceWith(start, from, node);
        view.dispatch(tr.setSelection(TextSelection.create(tr.doc, start + 1)).scrollIntoView());
        return true;
      },
    },
  });
