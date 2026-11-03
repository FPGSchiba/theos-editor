import type { Node } from 'prosemirror-model';
import { schema } from '../prosemirror/schema.ts';
import type { Block } from './blocks.ts';

/** What a change is mapped against: the source text, the document it stands for, and its blocks. */
export interface SourceState {
  /** The whole file, front matter included. */
  file: string;
  bodyStart: number;
  doc: Node;
  blocks: readonly Block[];
}

/**
 * A change written into the source: `insert` replaces `[from, to)` of the file. The blocks
 * `[first, last)` are the run the change falls in; `text` is that run's source after the change,
 * starting at offset `runFrom` of the new file.
 */
export interface SourceEdit {
  from: number;
  to: number;
  insert: string;
  first: number;
  last: number;
  runFrom: number;
  text: string;
  /** The changed blocks were written by the stock serializer (the fallback), not span-edited. */
  serialized: boolean;
  /** The top-level children the edit replaced. */
  blocks: ChangedBlocks;
}

/**
 * The top-level children a change replaced, from the equal children at both ends: before
 * `[changed, endBefore)`, after `[changed, endAfter)`.
 */
export interface ChangedBlocks {
  changed: number;
  endBefore: number;
  endAfter: number;
}

export function changedBlocks(before: Node, after: Node): ChangedBlocks {
  const n = before.childCount;
  const m = after.childCount;
  let start = 0;
  while (start < n && start < m && before.child(start).eq(after.child(start))) start++;
  let end = 0;
  while (end < n - start && end < m - start && before.child(n - 1 - end).eq(after.child(m - 1 - end))) end++;
  return { changed: start, endBefore: n - end, endAfter: m - end };
}

/** A document of these top-level blocks. */
export const docOf = (nodes: readonly Node[]): Node => schema.topNodeType.create(null, nodes);
