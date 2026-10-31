import { EditorView } from '@codemirror/view';

/**
 * CodeMirror draws only the lines around what is on screen, and it works that out from line heights
 * it has not measured yet: the first frame holds about twenty lines, and a callout page is taller than
 * the estimate, so the lines below the fold are not in the DOM at all. A record is short, and a person
 * (or a check that reads the page) expects all of it to be there, so a short document is drawn in full.
 * A long one keeps the viewport, which is what makes it fast.
 */
const WHOLE_LIMIT = 20_000;

export function drawWhole(view: EditorView): void {
  const { doc } = view.state;
  if (doc.length > WHOLE_LIMIT) return;
  // Reading the layout makes the first measure now, not a frame from now; asking for the end of the
  // document widens the viewport to it. The negative margin keeps the page where it is.
  view.lineBlockAtHeight(0);
  view.dispatch({ effects: EditorView.scrollIntoView(doc.length, { y: 'nearest', yMargin: -100_000 }) });
  view.lineBlockAtHeight(0);
}
