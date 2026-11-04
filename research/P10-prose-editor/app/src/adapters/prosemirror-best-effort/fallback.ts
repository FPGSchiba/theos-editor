import type { Node } from 'prosemirror-model';
import { serializeBody } from '../prosemirror/markdown.ts';
import { childrenOf, isEmptyParagraph, runAround } from './blocks.ts';
import { docOf, type ChangedBlocks, type SourceEdit, type SourceState } from './edit.ts';
import { keepFinalNewline, leadSeparator } from './spans.ts';
import { Verifier } from './verify.ts';

/**
 * The text of the blocks `nodes` as the stock serializer writes them, without a newline at the end.
 * Empty paragraphs, which the serializer writes as nothing, own a blank line each (separated by a
 * blank line), so the newline of their line is part of the text.
 */
function written(nodes: readonly Node[]): { text: string; ownsLine: boolean } {
  if (nodes.length > 0 && nodes.every(isEmptyParagraph)) return { text: '\n'.repeat(2 * nodes.length - 1), ownsLine: true };
  return { text: nodes.length === 0 ? '' : serializeBody(docOf(nodes)), ownsLine: false };
}

/**
 * The changed blocks `[changed, endBefore)` written again in place of their old lines. Blocks keep
 * whole lines and the blank lines between blocks stay outside them: replaced blocks end where the
 * old ones ended (the newline of their last line included), inserted blocks get a blank line to
 * the block behind them, and at the end of the file the final newline stays (`keepFinalNewline`).
 */
function rewrite(state: SourceState, after: Node, blocks: ChangedBlocks): SourceEdit {
  const { file, bodyStart, blocks: old } = state;
  const n = state.doc.childCount;
  const { changed, endBefore, endAfter } = blocks;
  const nodes = childrenOf(after, changed, endAfter);
  const { text: own, ownsLine } = written(nodes);
  let from: number;
  let to: number;
  let lead = '';
  let body: string;
  let trail = '';
  if (endBefore > changed) {
    from = old[changed]!.from;
    to = old[endBefore - 1]!.to;
    const lines = file.slice(from, to);
    const newline = lines === '' ? to < file.length : lines.endsWith('\n');
    body = own === '' || ownsLine ? own : own + (newline ? '\n' : '');
    if (nodes.length === 0) {
      // Blocks removed: the blank line behind them goes too (in front of them for the last ones).
      if (endBefore < n) to = old[endBefore]!.from;
      else if (changed > 0) from = old[changed - 1]!.to;
    }
  } else if (changed < n) {
    // Blocks added in front of block `changed`, with a blank line to it.
    from = old[changed]!.from;
    to = from;
    body = ownsLine ? own : `${own}\n`;
    trail = '\n';
  } else {
    // Blocks added at the end, behind a blank line.
    from = file.length;
    to = from;
    lead = leadSeparator(file, bodyStart, from);
    body = ownsLine ? own : keepFinalNewline(file, bodyStart, from, to, lead + own).slice(lead.length);
  }
  const insert = lead + body + trail;
  const [first, last] = runAround(file, old, endBefore > changed ? changed : Math.max(changed - 1, 0), Math.max(endBefore, changed + 1));
  const runFrom = Math.min(old[first]!.from, from);
  const runTo = Math.max(old[last - 1]!.to, to);
  const text = file.slice(runFrom, from) + insert + file.slice(to, runTo);
  const start = from + lead.length;
  return { from, to, insert, first, last, runFrom, text, serialized: true, blocks, wrote: [start, start + body.length] };
}

/**
 * The fallback when a change cannot be mapped as a span edit: the changed top-level blocks alone
 * are written again by the stock serializer. The result is parsed again like a span edit; when it
 * does not stand for the document (the serializer's text runs into a neighbour), the whole run of
 * blocks around it is written again. When neither stands for it (no Markdown holds the document),
 * the smaller rewrite is kept and its blocks get guessed ranges.
 */
export function reserialize(state: SourceState, after: Node, blocks: ChangedBlocks): SourceEdit {
  const old = state.blocks;
  const n = state.doc.childCount;
  const grown = after.childCount - n;
  let { changed, endBefore, endAfter } = blocks;
  // Blocks whose ranges are a guess share their text: they are written again together.
  if (endBefore > changed) {
    while (changed > 0 && old[changed]!.opaque === true && old[changed - 1]!.opaque === true) changed--;
    while (endBefore < n && old[endBefore - 1]!.opaque === true && old[endBefore]!.opaque === true) {
      endBefore++;
      endAfter++;
    }
  }
  const stands = (edit: SourceEdit): boolean => new Verifier().check(edit.text, childrenOf(after, edit.first, edit.last + grown), 'loose');
  const narrow = rewrite(state, after, { changed, endBefore, endAfter });
  if (stands(narrow) || (narrow.first === changed && narrow.last === endBefore)) return narrow;
  const wide = rewrite(state, after, { changed: narrow.first, endBefore: narrow.last, endAfter: narrow.last + grown });
  return stands(wide) ? wide : narrow;
}
