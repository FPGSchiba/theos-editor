import type { Node } from 'prosemirror-model';
import { PositionMap } from '../prosemirror/positions.ts';
import { childrenOf, positionOf, runAround, type Block } from './blocks.ts';
import { docOf } from './edit.ts';

interface RunMap {
  map: PositionMap;
  /** Document position in front of the run's first block, and source offset of its first line. */
  pos: number;
  from: number;
}

/**
 * Offsets and positions on the source text: the stock `PositionMap`, built per run of blocks
 * (the blocks that are parsed together) on demand, so a query reads one run and not the whole file,
 * and a run whose blocks do not pair with its tokens only makes guesses inside itself.
 */
export class RunPositions {
  private readonly runs = new Map<number, RunMap>();

  constructor(private readonly file: string, private readonly doc: Node, private readonly blocks: readonly Block[]) {}

  /** The source offset of document position `pos`. */
  toOffset(pos: number): number {
    const index = Math.min(this.doc.resolve(Math.min(Math.max(pos, 0), this.doc.content.size)).index(0), this.blocks.length - 1);
    const run = this.runOf(index);
    return run.from + run.map.toOffset(pos - run.pos);
  }

  /** The document position of source offset `offset`: in the last block that starts at or before it (the first for the front matter). */
  toPos(offset: number): number {
    let low = 0;
    let high = this.blocks.length - 1;
    let found = 0;
    while (low <= high) {
      const mid = (low + high) >> 1;
      if (this.blocks[mid]!.from <= offset) {
        found = mid;
        low = mid + 1;
      } else high = mid - 1;
    }
    const run = this.runOf(found);
    return run.pos + run.map.toPos(offset - run.from);
  }

  private runOf(index: number): RunMap {
    const [first, last] = runAround(this.file, this.blocks, index, index + 1);
    let run = this.runs.get(first);
    if (run === undefined) {
      const from = this.blocks[first]!.from;
      const text = this.file.slice(from, this.blocks[last - 1]!.to);
      run = { map: new PositionMap(text, 0, docOf(childrenOf(this.doc, first, last))), pos: positionOf(this.doc, first), from };
      this.runs.set(first, run);
    }
    return run;
  }
}
