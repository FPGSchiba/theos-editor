import type { Node } from 'prosemirror-model';
import { Plugin } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';

/**
 * Live preview of headings and emphasis. The tree renders them (`<h2>`, `<em>`, `<strong>`);
 * the Markdown markers of the blocks and runs the selection touches are added back as widget
 * decorations (`## ` before the heading, `*` around emphasis), so the caret line reads raw and
 * every other line is rendered. The document is never changed.
 */

const MARKERS: Record<string, string> = { em: '*', strong: '**', code: '`' };

function marker(text: string): () => HTMLElement {
  return () => {
    const span = document.createElement('span');
    span.className = 'pm-marker';
    span.textContent = text;
    return span;
  };
}

/** The runs of one mark in a textblock, as document positions [from, to]. */
function runsOf(block: Node, start: number, name: string): Array<[number, number]> {
  const runs: Array<[number, number]> = [];
  let open: [number, number] | undefined;
  block.forEach((child, offset) => {
    const marked = child.marks.some((mark) => mark.type.name === name);
    if (marked && open !== undefined) open[1] = start + offset + child.nodeSize;
    else if (marked) {
      open = [start + offset, start + offset + child.nodeSize];
      runs.push(open);
    } else open = undefined;
  });
  return runs;
}

function build(doc: Node, from: number, to: number): DecorationSet {
  const found: Decoration[] = [];
  doc.nodesBetween(from, to, (node, pos) => {
    if (!node.isTextblock) return true;
    const start = pos + 1;
    if (node.type.name === 'heading') {
      const level = Number(node.attrs.level);
      found.push(Decoration.widget(start, marker(`${'#'.repeat(level)} `), { side: -1, key: `h${level}@${start}` }));
    }
    for (const [name, text] of Object.entries(MARKERS)) {
      for (const [runFrom, runTo] of runsOf(node, start, name)) {
        if (runFrom > to || runTo < from) continue;
        found.push(Decoration.widget(runFrom, marker(text), { side: -1, key: `open-${name}@${runFrom}` }));
        found.push(Decoration.widget(runTo, marker(text), { side: 1, key: `close-${name}@${runTo}` }));
      }
    }
    return false;
  });
  return DecorationSet.create(doc, found);
}

export const livePreview = (): Plugin =>
  new Plugin({
    props: {
      decorations: (state) => build(state.doc, state.selection.from, state.selection.to),
    },
  });
