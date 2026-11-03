import type { Node } from 'prosemirror-model';
import { serializeBody } from '../prosemirror/markdown.ts';
import { childrenOf, runAround } from './blocks.ts';
import { docOf, type ChangedBlocks, type SourceEdit, type SourceState } from './edit.ts';
import { leadSeparator } from './spans.ts';

/**
 * The fallback when a change cannot be mapped as a span edit: the changed top-level blocks alone
 * are written again by the stock serializer and put in place of their old source lines. The
 * blocks around them and the blank lines between keep their bytes.
 */
export function reserialize(state: SourceState, after: Node, blocks: ChangedBlocks): SourceEdit {
  const { file, bodyStart, blocks: old } = state;
  const n = state.doc.childCount;
  let { changed, endBefore, endAfter } = blocks;
  // Blocks whose ranges are a guess share their text: they are written again together.
  if (endBefore > changed) {
    while (changed > 0 && old[changed]!.opaque === true && old[changed - 1]!.opaque === true) changed--;
    while (endBefore < n && old[endBefore - 1]!.opaque === true && old[endBefore]!.opaque === true) {
      endBefore++;
      endAfter++;
    }
  }
  const nodes = childrenOf(after, changed, endAfter);
  const written = nodes.length === 0 ? '' : serializeBody(docOf(nodes));
  let from: number;
  let to: number;
  let insert: string;
  if (endBefore > changed) {
    // Blocks replaced: the new ones take their lines, the newline of the last line included.
    from = old[changed]!.from;
    to = old[endBefore - 1]!.to;
    // Blocks that had no lines get one when text follows; otherwise the newline of the last line stays.
    const lines = file.slice(from, to);
    const newline = lines === '' ? to < file.length : lines.endsWith('\n');
    insert = written === '' ? '' : written + (newline ? '\n' : '');
    if (nodes.length === 0) {
      // Blocks removed: the blank line behind them goes too (in front of them for the last ones).
      if (endBefore < n) to = old[endBefore]!.from;
      else if (changed > 0) from = old[changed - 1]!.to;
    }
  } else if (changed < n) {
    // Blocks added in front of block `changed`.
    from = old[changed]!.from;
    to = from;
    insert = `${written}\n\n`;
  } else {
    // Blocks added at the end.
    from = file.length;
    to = from;
    insert = leadSeparator(file, bodyStart, from) + written;
  }
  const [first, last] = runAround(file, old, endBefore > changed ? changed : Math.max(changed - 1, 0), Math.max(endBefore, changed + 1));
  const runFrom = Math.min(old[first]!.from, from);
  const runTo = Math.max(old[last - 1]!.to, to);
  const text = file.slice(runFrom, from) + insert + file.slice(to, runTo);
  return { from, to, insert, first, last, runFrom, text, serialized: true, blocks: { changed, endBefore, endAfter } };
}
