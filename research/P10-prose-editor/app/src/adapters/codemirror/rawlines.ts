import type { EditorState } from '@codemirror/state';

/**
 * Live preview shows raw Markdown on every line a selection touches (the whole line, so a delete
 * or a selection never takes a marker the author did not see) and the rendered look elsewhere.
 * The result tells whether a span `from`..`to` shares a line with a selection.
 */
export function touchedLines(state: EditorState): (from: number, to: number) => boolean {
  const { doc } = state;
  const raw = state.selection.ranges.map((range) => [doc.lineAt(range.from).from, doc.lineAt(range.to).to] as const);
  return (from, to) => raw.some(([start, end]) => start <= to && end >= from);
}
