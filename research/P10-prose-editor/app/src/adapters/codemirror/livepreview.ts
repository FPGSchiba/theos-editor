import { syntaxTree } from '@codemirror/language';
import type { Range } from '@codemirror/state';
import { Decoration, ViewPlugin, type DecorationSet, type EditorView, type ViewUpdate } from '@codemirror/view';
import { touchedLines } from './rawlines.ts';

/**
 * Live preview of headings and emphasis: the text is styled, the markers (`##`, `*`, `_`) are
 * hidden, except on the lines the caret or a selection touches, where the raw Markdown shows.
 * The document is never changed; only what is drawn.
 */

const HEADING = /^(?:ATXHeading|SetextHeading)([1-6])$/;
const hidden = Decoration.replace({});
/** The heading looks come from the design system's manuscript type (`tokens.css`). */
const headingLine = ['ms-title', 'ms-h2', 'ms-h3', 'ms-h3', 'ms-h3', 'ms-h3'].map((name) => Decoration.line({ class: name }));
const emphasis = Decoration.mark({ class: 'tu-em' });
const strong = Decoration.mark({ class: 'tu-strong' });

function build(view: EditorView): DecorationSet {
  const { state } = view;
  const { doc } = state;
  const isRaw = touchedLines(state);
  const found: Array<Range<Decoration>> = [];
  const tree = syntaxTree(state);
  for (const { from, to } of view.visibleRanges) {
    tree.iterate({
      from,
      to,
      enter(node) {
        const heading = HEADING.exec(node.name);
        if (heading !== null) {
          found.push(headingLine[Number(heading[1]) - 1]!.range(doc.lineAt(node.from).from));
        } else if (node.name === 'Emphasis') {
          found.push(emphasis.range(node.from, node.to));
        } else if (node.name === 'StrongEmphasis') {
          found.push(strong.range(node.from, node.to));
        } else if (node.name === 'EmphasisMark' && !isRaw(node.from, node.to)) {
          found.push(hidden.range(node.from, node.to));
        } else if (node.name === 'HeaderMark' && node.node.parent?.name.startsWith('ATX') === true && !isRaw(node.from, node.to)) {
          // The space after the hashes goes with them.
          found.push(hidden.range(node.from, node.to + (doc.sliceString(node.to, node.to + 1) === ' ' ? 1 : 0)));
        }
      },
    });
  }
  return Decoration.set(found, true);
}

export const livePreview = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = build(view);
    }

    update(update: ViewUpdate): void {
      if (
        update.docChanged || update.selectionSet || update.viewportChanged
        || syntaxTree(update.startState) !== syntaxTree(update.state)
      ) {
        this.decorations = build(update.view);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);
