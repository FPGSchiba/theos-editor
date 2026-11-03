import type { Node } from 'prosemirror-model';
import type { Transaction } from 'prosemirror-state';
import { joinFrontMatter } from './frontmatter.ts';
import { serializeBody } from './markdown.ts';
import { Tracker } from './positions.ts';

/**
 * The seam between the Write-mode editor and its save: how the file text is produced from the
 * document, and which text the offsets of the harness refer to. The stock column writes the tree
 * with the stock serializer and keeps a position source beside it (`stockBody`); the best-effort
 * column (`../prosemirror-best-effort/`) keeps the source text itself and maps the changes onto it.
 */
export interface BodyModel {
  /** A file was loaded or replaced: its LF text, where its body starts, and the document of the body. */
  reset(file: string, bodyStart: number, doc: Node): void;
  /** When defined, the adapter hands it every transaction that changed the Write-mode document, in order. */
  observe?(tr: Transaction): void;
  /** The whole file as Write mode saves it for the document `doc`. */
  save(doc: Node, frontMatter: string): string;
  /** The file text the offsets refer to, for the document `doc`. */
  source(doc: Node): string;
  /** Where the body starts in `source`. */
  start(): number;
  /** The document position of a source offset. */
  toPos(doc: Node, offset: number): number;
  /** The source offset of a document position. */
  toOffset(doc: Node, pos: number): number;
}

/**
 * The stock column: the body is what the stock serializer writes; the offsets refer to the
 * `Tracker`'s position source, which is never saved.
 */
export function stockBody(): BodyModel {
  let tracker: Tracker | undefined;
  const synced = (doc: Node): Tracker => {
    if (tracker === undefined) throw new Error('prosemirror: the body model has no file yet');
    tracker.sync(doc);
    return tracker;
  };
  return {
    reset(file, bodyStart, doc) {
      tracker = new Tracker(file, bodyStart, doc);
    },
    save: (doc, frontMatter) => joinFrontMatter(frontMatter, serializeBody(doc)),
    source: (doc) => synced(doc).text(),
    start: () => tracker?.start() ?? 0,
    toPos: (doc, offset) => synced(doc).toPos(offset),
    toOffset: (doc, pos) => synced(doc).toOffset(pos),
  };
}
