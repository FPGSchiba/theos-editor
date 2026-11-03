import type { Node } from 'prosemirror-model';
import { sourceLeaf } from './links.ts';
import { tokenize } from './markdown.ts';

/**
 * Source offsets and ProseMirror positions. The harness speaks offsets into the whole file (LF
 * form, front matter included); ProseMirror speaks positions in a tree that kept no source. The
 * bridge between them is the "position source": the file text the offsets refer to, kept apart
 * from the saved text (`text()` never reads it).
 *
 * `PositionMap` pairs every textblock of a document with the line range markdown-it gives its
 * block token (`map`), and aligns the text of the block with its source lines character by
 * character: the markers (`## `, `> `, `- `, `**`, `\`) are the source characters no text
 * character is matched to. `Tracker` keeps the position source up to date when the document
 * changes, by splicing the changed text into it, as the oracle does with the file.
 *
 * Limits: the alignment is greedy, so a text character without a source twin (an entity such as
 * `&amp;`) is placed at the cursor; blocks that markdown-it and the schema do not pair one to
 * one (an empty list item) make the whole map fall back to proportional guesses.
 */

interface Block {
  node: Node;
  /** Document position of the first content character. */
  start: number;
  size: number;
  /** Offsets into the position source: the first line of the block and the start of the line after it. */
  from: number;
  to: number;
  fenced: boolean;
  aligned?: number[];
  /** For each position of the block's content, how many text characters of the source stand before it. */
  chars?: number[];
  firstContent?: number;
}

/** What precedes the text on a source line: quote marks, list markers, an ATX heading's hashes. */
const PREFIX = /^(?:[ \t]*>[ \t]?)*[ \t]*(?:(?:[-*+]|\d{1,9}[.)])[ \t]+)*(?:#{1,6}(?:[ \t]+|$))?/;

const BLOCK_TOKENS = new Set(['paragraph_open', 'heading_open', 'fence', 'code_block', 'callout_title_open', 'callout_caption_open', 'beat_heading_open', 'beat_attrs_open']);

/** After the line prefix, the title of a callout starts behind `[!type] ` and a caption behind the dash. */
const AFTER_PREFIX: Record<string, RegExp> = { callout_title: /^\[![a-z]+\][ \t]?/i, callout_caption: /^—[ \t]?/ };

/**
 * The next place of `char` in `text`. A soft line break is a space in the stock tree, so a space
 * also matches the end of a source line.
 */
function find(text: string, char: string, from: number): number {
  const at = text.indexOf(char, from);
  if (char !== ' ') return at;
  const newline = text.indexOf('\n', from);
  return at === -1 || (newline !== -1 && newline < at) ? newline : at;
}

function lineStartsOf(text: string): number[] {
  const starts = [0];
  for (let at = text.indexOf('\n'); at !== -1; at = text.indexOf('\n', at + 1)) starts.push(at + 1);
  return starts;
}

/** The map of one document against one position source. */
export class PositionMap {
  private readonly blocks: Block[] = [];
  /** False when markdown-it and the document disagree about the blocks; then only guesses are possible. */
  readonly exact: boolean;

  constructor(private readonly source: string, private readonly bodyStart: number, private readonly doc: Node) {
    const body = source.slice(bodyStart);
    const lines = lineStartsOf(body);
    const tokens = tokenize(body).filter((token) => BLOCK_TOKENS.has(token.type) && token.map !== null);
    const textblocks: Array<{ node: Node; start: number }> = [];
    doc.descendants((node, pos) => {
      if (node.isTextblock) textblocks.push({ node, start: pos + 1 });
      return !node.isTextblock;
    });
    this.exact = tokens.length === textblocks.length;
    if (!this.exact) return;
    tokens.forEach((token, index) => {
      const block = textblocks[index]!;
      const [first, end] = token.map!;
      this.blocks.push({
        node: block.node,
        start: block.start,
        size: block.node.content.size,
        from: bodyStart + (lines[first] ?? body.length),
        to: bodyStart + (lines[end] ?? body.length),
        fenced: token.type === 'fence' || token.type === 'code_block',
      });
    });
  }

  /** The source offset of document position `pos` (the point before the character at `pos`). */
  toOffset(pos: number): number {
    if (!this.exact || this.blocks.length === 0) return this.guessOffset(pos);
    const index = this.blockAt(pos);
    const block = this.blocks[index]!;
    if (pos < block.start) return block.from + this.firstContent(block);
    return this.offsetIn(block, this.charsBefore(block, Math.min(pos, block.start + block.size) - block.start));
  }

  /** The document position of source offset `offset`. */
  toPos(offset: number): number {
    if (!this.exact || this.blocks.length === 0) return this.guessPos(offset);
    let low = 0;
    let high = this.blocks.length - 1;
    let found = -1;
    while (low <= high) {
      const mid = (low + high) >> 1;
      if (this.blocks[mid]!.from <= offset) {
        found = mid;
        low = mid + 1;
      } else high = mid - 1;
    }
    if (found === -1) return this.blocks[0]!.start;
    const block = this.blocks[found]!;
    const aligned = this.align(block);
    const inBlock = offset - block.from;
    let first = 0;
    let last = aligned.length;
    while (first < last) {
      const mid = (first + last) >> 1;
      if (aligned[mid]! >= inBlock) last = mid;
      else first = mid + 1;
    }
    return block.start + this.positionOf(block, first);
  }

  /** The nearest block at or after `pos` that contains it, else the next one, else the last. */
  private blockAt(pos: number): number {
    let low = 0;
    let high = this.blocks.length - 1;
    let found = -1;
    while (low <= high) {
      const mid = (low + high) >> 1;
      if (this.blocks[mid]!.start <= pos) {
        found = mid;
        low = mid + 1;
      } else high = mid - 1;
    }
    if (found === -1) return 0;
    const block = this.blocks[found]!;
    if (pos <= block.start + block.size) return found;
    return Math.min(found + 1, this.blocks.length - 1);
  }

  private offsetIn(block: Block, index: number): number {
    const aligned = this.align(block);
    if (index < aligned.length) return block.from + aligned[index]!;
    if (aligned.length === 0) return block.from + this.firstContent(block);
    return block.from + aligned[aligned.length - 1]! + 1;
  }

  /** Text characters before each position: a link is one position and as many characters as its syntax. */
  private charTable(block: Block): number[] {
    if (block.chars !== undefined) return block.chars;
    const table = [0];
    let count = 0;
    block.node.forEach((child) => {
      if (child.isText) for (let i = 0; i < child.nodeSize; i++) table.push(++count);
      else {
        count += sourceLeaf(child).length;
        table.push(count);
      }
    });
    block.chars = table;
    return table;
  }

  private charsBefore(block: Block, index: number): number {
    return this.charTable(block)[index] ?? index;
  }

  /** The position (relative to the block start) at character `char`: inside a link it is the one before it. */
  private positionOf(block: Block, char: number): number {
    const table = this.charTable(block);
    let low = 0;
    let high = table.length - 1;
    while (low < high) {
      const mid = (low + high + 1) >> 1;
      if (table[mid]! <= char) low = mid;
      else high = mid - 1;
    }
    return low;
  }

  private firstContent(block: Block): number {
    this.align(block);
    return block.firstContent ?? 0;
  }

  /** For each character of the block's text, its index in the block's source lines. */
  private align(block: Block): number[] {
    if (block.aligned !== undefined) return block.aligned;
    const text = this.source.slice(block.from, block.to);
    const chars = block.node.textBetween(0, block.size, '', sourceLeaf);
    const starts = lineStartsOf(text);
    const contents = new Map<number, number>();
    const extra = AFTER_PREFIX[block.node.type.name];
    const contentStart = (line: number): number => {
      let known = contents.get(line);
      if (known === undefined) {
        const begin = starts[line]!;
        const end = starts[line + 1] === undefined ? text.length : starts[line + 1]! - 1;
        if (block.fenced) known = begin;
        else {
          const line = text.slice(begin, end);
          const prefix = PREFIX.exec(line)?.[0].length ?? 0;
          known = begin + prefix + (extra?.exec(line.slice(prefix))?.[0].length ?? 0);
        }
        contents.set(line, known);
      }
      return known;
    };
    // A fenced block's first line is the fence, not text.
    let cursor = block.fenced ? (starts[1] ?? text.length) : 0;
    block.firstContent = block.fenced ? cursor : contentStart(0);
    const aligned = new Array<number>(chars.length);
    let line = 0;
    for (let i = 0; i < chars.length; i++) {
      let at = -1;
      for (;;) {
        at = find(text, chars[i]!, cursor);
        if (at === -1) break;
        while (line + 1 < starts.length && starts[line + 1]! <= at) line++;
        const content = contentStart(line);
        if (at >= content) break;
        cursor = content;
      }
      if (at === -1) {
        aligned[i] = Math.min(cursor, text.length);
      } else {
        aligned[i] = at;
        cursor = at + 1;
      }
    }
    block.aligned = aligned;
    return aligned;
  }

  private guessOffset(pos: number): number {
    const size = Math.max(this.doc.content.size, 1);
    return this.bodyStart + Math.round(((this.source.length - this.bodyStart) * Math.min(pos, size)) / size);
  }

  private guessPos(offset: number): number {
    const length = Math.max(this.source.length - this.bodyStart, 1);
    const size = this.doc.content.size;
    return Math.min(Math.max(Math.round((Math.max(offset - this.bodyStart, 0) / length) * size), 1), Math.max(size - 1, 1));
  }
}

/** Keeps the position source and its map in step with a document that changes. */
export class Tracker {
  private map: PositionMap | undefined;

  constructor(private source: string, private bodyStart: number, private base: Node) {}

  /** The file the offsets refer to (not what `text()` saves). */
  text(): string {
    return this.source;
  }

  /** Where the body starts in the position source. */
  start(): number {
    return this.bodyStart;
  }

  /** Make the position source follow `doc`: the changed range is spliced in as plain text. */
  sync(doc: Node): void {
    if (doc === this.base) return;
    const start = this.base.content.findDiffStart(doc.content);
    if (start === null) {
      this.base = doc;
      this.map = undefined;
      return;
    }
    const end = this.base.content.findDiffEnd(doc.content);
    let endOld = end?.a ?? start;
    let endNew = end?.b ?? start;
    const overlap = start - Math.min(endOld, endNew);
    if (overlap > 0) {
      endOld += overlap;
      endNew += overlap;
    }
    const map = this.current();
    const from = map.toOffset(start);
    const to = Math.max(map.toOffset(endOld), from);
    this.source = this.source.slice(0, from) + doc.textBetween(start, endNew, '\n\n', sourceLeaf) + this.source.slice(to);
    this.base = doc;
    this.map = undefined;
  }

  /** The file changed from outside: `source` is the new position source and `doc` what it parses to. */
  reset(source: string, bodyStart: number, doc: Node): void {
    this.source = source;
    this.bodyStart = bodyStart;
    this.base = doc;
    this.map = undefined;
  }

  toPos(offset: number): number {
    return this.current().toPos(offset);
  }

  toOffset(pos: number): number {
    return this.current().toOffset(pos);
  }

  private current(): PositionMap {
    this.map ??= new PositionMap(this.source, this.bodyStart, this.base);
    return this.map;
  }
}
