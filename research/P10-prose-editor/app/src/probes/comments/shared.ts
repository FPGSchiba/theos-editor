import { readFileSync } from 'node:fs';

/** A comment on a text range. `from` and `to` are UTF-16 offsets into the whole file (front matter included), `from < to`. */
export interface Comment {
  id: string;
  from: number;
  to: number;
}

/**
 * What both probes expose, so one set of behaviours runs on both editors. Every offset is a file
 * offset. The comments live outside the file: none of these calls except `edit` and `external`
 * changes the text.
 */
export interface CommentSession {
  /** The file text as the editor would save it. */
  text(): string;
  /** Add a comment (an undoable action). */
  add(comment: Comment): void;
  /** Remove a comment by id (an undoable action). */
  remove(id: string): void;
  /** The comments as file offsets, ordered by start. */
  list(): Comment[];
  /** Put comments back, e.g. into a fresh session (not an undoable action). */
  restore(comments: Comment[]): void;
  /** A user edit through the editor's own transaction: replace file offsets `from..to` by `insert`. */
  edit(from: number, to: number, insert: string): void;
  /** A change from outside the editor: same replacement, kept out of the undo history. */
  external(from: number, to: number, insert: string): void;
  undo(): void;
  redo(): void;
}

/** The corpus file both probes read. */
export const FILE = readFileSync(new URL('../../../../corpus/records/lore/topics/synthetic-links-explaining.md', import.meta.url), 'utf8').replace(/\r\n?/g, '\n');

/** Range (a): plain prose. */
export const PROSE = 'The binders call them ancestors';
/** Range (a2): plain prose that does not start its paragraph. */
export const PROSE_MID = 'call them ancestors';
/** Range (b): from "record" to behind the closing brackets of the link. */
export const LINK_RANGE = 'record: [[fates-shrine]]';

/** The file offsets of `quote` in the corpus file. */
export function rangeOf(quote: string): { from: number; to: number } {
  const from = FILE.indexOf(quote);
  if (from === -1) throw new Error(`not in the corpus file: ${quote}`);
  return { from, to: from + quote.length };
}
