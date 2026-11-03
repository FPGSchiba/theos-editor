import type { Node } from 'prosemirror-model';
import type { Transaction } from 'prosemirror-state';
import { stockBody, type BodyModel } from '../prosemirror/body.ts';
import { joinFrontMatter } from '../prosemirror/frontmatter.ts';
import { tokenize } from '../prosemirror/markdown.ts';
import { blocksOf, childrenOf, runAround, type Block } from './blocks.ts';
import { changeOf, diffOf, joinChanges, type Change } from './changes.ts';
import { changedBlocks, type SourceEdit } from './edit.ts';
import { reserialize } from './fallback.ts';
import { RunPositions } from './offsets.ts';
import { spanEdit } from './spanedit.ts';
import { Verifier } from './verify.ts';

const copy = (blocks: readonly Block[]): Block[] => blocks.map((block) => ({ ...block }));

/**
 * Block ranges for a rewritten run that does not parse into one token per block (a document no
 * Markdown stands for, such as an attribute block holding a line that is no attribute line): the
 * neighbours keep theirs, the first rewritten block gets the whole written text and the others an
 * empty range behind it. They are `opaque`, so a later change rewrites them together.
 */
function guessed(edit: SourceEdit, old: readonly Block[], grown: number, changed: number, endAfter: number): Block[] {
  const shift = edit.insert.length - (edit.to - edit.from);
  const end = edit.from + edit.insert.length;
  const blocks: Block[] = [];
  for (let index = edit.first; index < edit.last + grown; index++) {
    if (index < changed) blocks.push({ ...old[index]! });
    else if (index >= endAfter) blocks.push({ ...old[index - grown]!, from: old[index - grown]!.from + shift, to: old[index - grown]!.to + shift });
    else blocks.push({ from: index === changed ? edit.from : end, to: end, serialized: true, opaque: true });
  }
  return blocks;
}

/**
 * The best-effort body: the source text is the saved truth. Untouched blocks keep their bytes;
 * every change of the document is mapped onto the source as a span edit, verified by parsing the
 * run of blocks around it again, and where that fails the changed blocks alone are written by
 * the stock serializer (see NOTES.md). The offsets of the harness refer to the same text.
 */
export class SourceBody implements BodyModel {
  private file = '';
  private bodyStart = 0;
  /** The document `file` stands for. */
  private doc: Node | undefined;
  private blocks: Block[] = [];
  /** The state as loaded (or as the last external change or mode switch left it). */
  private loaded: { file: string; doc: Node; blocks: Block[] } | undefined;
  /** Transactions not yet mapped onto the source. */
  private pending: Transaction[] = [];
  private map: RunPositions | undefined;
  /** Set when the blocks of the source cannot be paired with the document: then the stock body writes the text. */
  private stock: BodyModel | undefined;

  reset(file: string, bodyStart: number, doc: Node): void {
    this.file = file;
    this.bodyStart = bodyStart;
    this.doc = doc;
    this.pending = [];
    this.map = undefined;
    this.stock = undefined;
    const body = file.slice(bodyStart);
    const blocks = blocksOf(body, bodyStart, tokenize(body), childrenOf(doc, 0, doc.childCount));
    if (blocks === undefined) {
      this.giveUp(doc);
      this.blocks = [];
    } else {
      blocks.forEach((block, index) => {
        block.origin = { node: doc.child(index), text: file.slice(block.from, block.to) };
      });
      this.blocks = blocks;
    }
    this.loaded = { file, doc, blocks: copy(this.blocks) };
  }

  observe(tr: Transaction): void {
    this.pending.push(tr);
  }

  save(doc: Node, frontMatter: string): string {
    this.sync(doc);
    if (this.stock !== undefined) return doc.eq(this.loaded!.doc) ? this.loaded!.file : this.stock.save(doc, frontMatter);
    return joinFrontMatter(frontMatter, this.file.slice(this.bodyStart));
  }

  source(doc: Node): string {
    this.sync(doc);
    return this.stock?.source(doc) ?? this.file;
  }

  start(): number {
    return this.bodyStart;
  }

  toPos(doc: Node, offset: number): number {
    this.sync(doc);
    return this.stock?.toPos(doc, offset) ?? this.positions().toPos(offset);
  }

  toOffset(doc: Node, pos: number): number {
    this.sync(doc);
    return this.stock?.toOffset(doc, pos) ?? this.positions().toOffset(pos);
  }

  /** `block` while a block of the saved text was written by the serializer since the load, else `span`. */
  level(): 'span' | 'block' {
    if (this.stock !== undefined) return this.doc === undefined || this.doc.eq(this.loaded!.doc) ? 'span' : 'block';
    return this.blocks.some((block) => block.serialized) ? 'block' : 'span';
  }

  // ── Mapping the changes ──────────────────────────────────────────────────

  /** Map every pending transaction onto the source, so that `file` stands for `doc`. */
  private sync(doc: Node): void {
    const pending = this.pending;
    this.pending = [];
    if (this.stock !== undefined) {
      this.doc = doc;
      return;
    }
    try {
      this.mapAll(pending, doc);
    } catch (error) {
      // A defect of the mapping must not leave a source that stands for no document: the stock body takes over, and the error shows.
      this.giveUp(doc);
      throw error;
    }
  }

  private mapAll(pending: readonly Transaction[], doc: Node): void {
    let current = this.doc!;
    let group: Change | undefined;
    for (const tr of pending) {
      if (tr.before !== current) {
        // The state was replaced in between without a transaction: take the difference first.
        if (group !== undefined) this.applyChange(group, current);
        group = undefined;
        this.applyDiff(tr.before);
        current = tr.before;
      }
      const change = changeOf(tr);
      if (change !== undefined) {
        const joined = group === undefined ? undefined : joinChanges(group, tr, change);
        if (joined !== undefined) group = joined;
        else {
          if (group !== undefined) this.applyChange(group, current);
          group = change;
        }
      }
      current = tr.doc;
    }
    if (group !== undefined) this.applyChange(group, current);
    if (current !== doc) this.applyDiff(doc);
  }

  private applyDiff(after: Node): void {
    if (this.stock !== undefined) return;
    const change = diffOf(this.doc!, after);
    if (change !== undefined) this.applyChange(change, after);
    else {
      this.doc = after;
      this.map = undefined;
    }
  }

  /** One change, from the document `file` stands for to `after`. */
  private applyChange(change: Change, after: Node): void {
    if (this.stock !== undefined) return;
    const before = this.doc!;
    this.map = undefined;
    if (after === before) return;
    if (after.eq(before)) {
      this.doc = after;
      return;
    }
    const loaded = this.loaded!;
    if (after.content.size === loaded.doc.content.size && after.eq(loaded.doc)) {
      // Back to the loaded document (undo to the start): the loaded text comes back whole.
      this.file = loaded.file;
      this.blocks = copy(loaded.blocks);
      this.doc = after;
      return;
    }
    const blocks = changedBlocks(before, after);
    const state = { file: this.file, bodyStart: this.bodyStart, doc: before, blocks: this.blocks };
    const edit = spanEdit(state, after, change, blocks) ?? reserialize(state, after, blocks);
    this.applyEdit(edit, after);
  }

  private applyEdit(edit: SourceEdit, after: Node): void {
    const { changed, endBefore, endAfter } = edit.blocks;
    const before = this.doc!;
    const grown = after.childCount - before.childCount;
    const nodes = childrenOf(after, edit.first, edit.last + grown);
    const old = this.blocks;
    const fresh = blocksOf(edit.text, edit.runFrom, tokenize(edit.text), nodes) ?? guessed(edit, old, grown, changed, endAfter);
    fresh.forEach((block, offset) => {
      const index = edit.first + offset;
      // The old block this one continues: an unchanged neighbour, or a changed block at the same place.
      const twin = index < changed ? old[index] : index >= endAfter ? old[index - grown] : index < endBefore ? old[index] : undefined;
      if (twin?.origin !== undefined) block.origin = twin.origin;
      const rewritten = edit.serialized && index >= changed && index < endAfter;
      block.serialized = rewritten || (twin?.serialized ?? edit.serialized);
    });
    const shift = edit.insert.length - (edit.to - edit.from);
    this.file = this.file.slice(0, edit.from) + edit.insert + this.file.slice(edit.to);
    this.blocks = [
      ...old.slice(0, edit.first),
      ...fresh,
      ...old.slice(edit.last).map((block) => ({ ...block, from: block.from + shift, to: block.to + shift })),
    ];
    this.doc = after;
    this.restore(edit.first, edit.first + fresh.length);
  }

  /**
   * A changed block that is equal to its block as loaded gets its loaded text back (an undo also
   * undoes a fallback), as long as its run still parses to the document with that text.
   */
  private restore(start: number, end: number): void {
    const doc = this.doc!;
    for (let index = start; index < end; index++) {
      const block = this.blocks[index]!;
      const origin = block.origin;
      if (origin === undefined || !doc.child(index).eq(origin.node)) continue;
      const current = this.file.slice(block.from, block.to);
      if (current === origin.text && block.opaque !== true) {
        block.serialized = false;
        continue;
      }
      const [first, last] = runAround(this.file, this.blocks, index, index + 1);
      const runFrom = this.blocks[first]!.from;
      const text = this.file.slice(runFrom, block.from) + origin.text + this.file.slice(block.to, this.blocks[last - 1]!.to);
      const nodes = childrenOf(doc, first, last);
      if (!new Verifier().check(text, nodes, 'strict')) continue;
      const fresh = blocksOf(text, runFrom, tokenize(text), nodes);
      if (fresh === undefined) continue;
      fresh.forEach((one, offset) => {
        const was = this.blocks[first + offset]!;
        if (was.origin !== undefined) one.origin = was.origin;
        one.serialized = first + offset === index ? false : was.serialized;
      });
      const shift = origin.text.length - current.length;
      this.file = this.file.slice(0, block.from) + origin.text + this.file.slice(block.to);
      this.blocks.splice(first, last - first, ...fresh);
      for (let next = first + fresh.length; next < this.blocks.length; next++) {
        this.blocks[next]!.from += shift;
        this.blocks[next]!.to += shift;
      }
    }
  }

  /** From here on the stock body writes the text (the loaded source and its document do not pair). */
  private giveUp(doc: Node): void {
    this.stock = stockBody();
    this.stock.reset(this.file, this.bodyStart, doc);
    this.doc = doc;
  }

  private positions(): RunPositions {
    this.map ??= new RunPositions(this.file, this.doc!, this.blocks);
    return this.map;
  }
}
