import {
  Annotation, ChangeSet, EditorSelection, EditorState, StateField, Transaction, type ChangeSpec, type Extension, type Text,
} from '@codemirror/state';
import { Decoration, EditorView, type DecorationSet } from '@codemirror/view';

/**
 * Front matter stays inside the document, so every offset of the harness is a document offset.
 * Write mode hides it behind a replace decoration; both modes protect its bytes with a
 * transaction filter. History runs with `filter: false`, but it only inverts changes that
 * passed the filter, so undo cannot reach the front matter either.
 */

/** A change that does not come from the author (the file changed on disk): the filter lets it pass. */
export const fromOutside = Annotation.define<boolean>();

/** Where the body starts: the end of the closing `---` line and its newline; 0 without front matter. */
export function frontMatterLength(doc: Text): number {
  if (doc.lines < 2 || doc.line(1).text !== '---') return 0;
  for (let number = 2; number <= doc.lines; number++) {
    const line = doc.line(number);
    if (line.text === '---') return Math.min(line.to + 1, doc.length);
  }
  return 0;
}

/** The body start, kept up to date; only a change that reaches the front matter makes it look again. */
export const bodyStart = StateField.define<number>({
  create: (state) => frontMatterLength(state.doc),
  update(end, tr) {
    if (!tr.docChanged) return end;
    let reaches = end === 0;
    tr.changes.iterChangedRanges((from) => { if (from < end) reaches = true; });
    return reaches ? frontMatterLength(tr.newDoc) : end;
  },
});

/** No change of the author may alter bytes before the body: inside is dropped, a span across it starts at the body. */
export const protectFrontMatter: Extension = EditorState.transactionFilter.of((tr) => {
  const end = tr.startState.field(bodyStart);
  if (end === 0 || !tr.docChanged || tr.annotation(fromOutside) === true) return tr;
  const kept: ChangeSpec[] = [];
  let cut = false;
  tr.changes.iterChanges((from, to, _fromB, _toB, insert) => {
    if (from >= end) {
      kept.push({ from, to, insert });
      return;
    }
    cut = true;
    if (to > end) kept.push({ from: end, to, insert });
  });
  if (!cut) return tr;
  if (kept.length === 0) return [];
  const changes = ChangeSet.of(kept, tr.startState.doc.length);
  // The caret goes behind what was typed, as it would have if the change had not been cut.
  return [{
    changes,
    selection: EditorSelection.cursor(changes.mapPos(end, 1)),
    userEvent: tr.annotation(Transaction.userEvent),
  }];
});

const hiddenRange = (state: EditorState): DecorationSet => {
  const end = state.field(bodyStart);
  return end > 0 ? Decoration.set(Decoration.replace({}).range(0, end)) : Decoration.none;
};

/** Write mode: no trace of the front matter, the caret cannot step into it. */
export const hideFrontMatter: Extension = [
  EditorView.decorations.compute([bodyStart], hiddenRange),
  EditorView.atomicRanges.of((view) => hiddenRange(view.state)),
  EditorState.transactionFilter.of((tr) => {
    const end = tr.changes.mapPos(tr.startState.field(bodyStart), 1);
    const selection = tr.newSelection;
    if (selection.ranges.every((range) => range.from >= end)) return tr;
    const ranges = selection.ranges.map((range) =>
      EditorSelection.range(Math.max(range.anchor, end), Math.max(range.head, end)));
    return [tr, { selection: EditorSelection.create(ranges, selection.mainIndex), sequential: true }];
  }),
];

/** Source mode: the front matter is shown raw, set apart by a line class. */
export const showFrontMatter: Extension = EditorView.decorations.compute([bodyStart], (state) => {
  const end = state.field(bodyStart);
  if (end === 0) return Decoration.none;
  const mark = Decoration.line({ class: 'cm-frontmatter' });
  const last = state.doc.lineAt(end - 1).number;
  const lines = [];
  for (let number = 1; number <= last; number++) lines.push(mark.range(state.doc.line(number).from));
  return Decoration.set(lines);
});

/** The body start for a caret that is placed at `offset` in Write mode. */
export const clampToBody = (state: EditorState, offset: number): number => Math.max(offset, state.field(bodyStart));
