import type { Node } from 'prosemirror-model';
import type { MarkdownToken } from '../prosemirror/markdown.ts';

/**
 * The top-level blocks of the source text. Every top-level child of the document has one: the
 * source lines markdown-it gives its top-level token (`map`), the newline of the last line
 * included. What lies between two blocks (blank lines) is a gap and belongs to neither.
 */
export interface Block {
  /** Offsets into the whole file: the start of the first line and the end of the last one (behind its newline). */
  from: number;
  to: number;
  /** The block as it was loaded (or as the last external change or mode switch left it): node and text. */
  origin?: { node: Node; text: string };
  /** The text of the block was written by the stock serializer, not kept or span-edited. */
  serialized: boolean;
  /** The range is a guess: the rewritten run it is in did not parse into one token per block. */
  opaque?: boolean;
}

/** The tokens that stand for a top-level child of the document: level 0, opening or self-contained, with a line range. */
export function topTokens(tokens: MarkdownToken[]): MarkdownToken[] {
  return tokens.filter((token) => token.level === 0 && token.nesting !== -1 && token.block && token.map !== null);
}

export function lineStarts(text: string): number[] {
  const starts = [0];
  for (let at = text.indexOf('\n'); at !== -1; at = text.indexOf('\n', at + 1)) starts.push(at + 1);
  return starts;
}

/** A paragraph without content (Enter at the end of a paragraph makes one): no Markdown holds it, and markdown-it gives it no token. */
export const isEmptyParagraph = (node: Node): boolean => node.type.name === 'paragraph' && node.content.size === 0;

/**
 * The blocks of `text` (which starts at offset `base` of the file) for the document children
 * `nodes`, or undefined when markdown-it and the document do not pair. A block is the lines of its
 * token's `map`, never the blank lines around it. An empty paragraph has no token: it owns a blank
 * line of the gap it stands in, behind the one blank line that separates it from the block before
 * (an empty body: an empty range at the start).
 */
export function blocksOf(text: string, base: number, tokens: MarkdownToken[], nodes: readonly Node[]): Block[] | undefined {
  const top = topTokens(tokens);
  const starts = lineStarts(text);
  const at = (line: number): number => starts[line] ?? text.length;
  const ranged = (token: MarkdownToken): Block => {
    const from = at(token.map![0]);
    return { from: base + from, to: base + withoutBlankLines(text, from, at(token.map![1])), serialized: false };
  };
  if (top.length === nodes.length) return top.map(ranged);
  if (top.length !== nodes.filter((node) => !isEmptyParagraph(node)).length) return undefined;
  const blocks: Block[] = [];
  let next = 0;
  nodes.forEach((node, index) => {
    if (!isEmptyParagraph(node)) {
      blocks.push(ranged(top[next++]!));
      return;
    }
    const gapFrom = index === 0 ? 0 : blocks[index - 1]!.to - base;
    const gapTo = next < top.length ? at(top[next]!.map![0]) : text.length;
    const [from, to] = blankLine(text, gapFrom, gapTo, index > 0);
    blocks.push({ from: base + from, to: base + to, serialized: false });
  });
  return blocks;
}

/** The blank line of `[from, to)` an empty paragraph owns: the second one behind a block (the first separates), else the first. */
function blankLine(text: string, from: number, to: number, behindBlock: boolean): [number, number] {
  const lines: number[] = [];
  for (let start = from; start < to;) {
    const end = text.indexOf('\n', start);
    if (end === -1 || end >= to || !/^[ \t]*$/.test(text.slice(start, end))) break;
    lines.push(start);
    start = end + 1;
  }
  const line = lines[behindBlock && lines.length > 1 ? 1 : 0];
  return line === undefined ? [from, from] : [line, text.indexOf('\n', line) + 1];
}

/** The end of `[from, to)` of `text` without the blank lines at its end (markdown-it counts the one behind a list to the list). */
function withoutBlankLines(text: string, from: number, to: number): number {
  let end = to;
  while (end > from) {
    const contentEnd = text[end - 1] === '\n' ? end - 1 : end;
    const lineStart = text.lastIndexOf('\n', contentEnd - 1) + 1;
    if (lineStart <= from || !/^[ \t]*$/.test(text.slice(lineStart, contentEnd))) break;
    end = lineStart;
  }
  return end;
}

/** The children of `doc` from `start` up to `end`. */
export function childrenOf(doc: Node, start: number, end: number): Node[] {
  const nodes: Node[] = [];
  for (let index = start; index < end; index++) nodes.push(doc.child(index));
  return nodes;
}

/** The document position in front of child `index`. */
export function positionOf(doc: Node, index: number): number {
  let pos = 0;
  for (let at = 0; at < index; at++) pos += doc.child(at).nodeSize;
  return pos;
}

/**
 * Whether the blocks `index - 1` and `index` have to be parsed together: they are not separated
 * by a blank line (the gap is empty), or something that is no block lies between them (a
 * reference definition), which may change how either parses.
 */
export function joined(source: string, blocks: readonly Block[], index: number): boolean {
  const gap = source.slice(blocks[index - 1]!.to, blocks[index]!.from);
  return gap === '' || /\S/.test(gap);
}

/**
 * The run of blocks that has to be parsed as one: `[start, end)` widened over every neighbour
 * that is joined to it. Blocks separated by blank lines parse independently in markdown-it, except
 * for reference definitions and list continuation by indentation (see NOTES.md).
 */
export function runAround(source: string, blocks: readonly Block[], start: number, end: number): [number, number] {
  let first = Math.max(0, Math.min(start, blocks.length - 1));
  let last = Math.min(blocks.length, Math.max(end, first + 1));
  while (first > 0 && joined(source, blocks, first)) first--;
  while (last < blocks.length && joined(source, blocks, last)) last++;
  return [first, last];
}
