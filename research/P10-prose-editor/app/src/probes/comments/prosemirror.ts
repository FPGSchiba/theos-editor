import { isHistoryTransaction, history, redo, redoDepth, undo, undoDepth } from 'prosemirror-history';
import { EditorState, Plugin, PluginKey, type Transaction } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';
import { stockBody, type BodyModel } from '../../adapters/prosemirror/body.ts';
import { splitFrontMatter } from '../../adapters/prosemirror/frontmatter.ts';
import { parseBody } from '../../adapters/prosemirror/markdown.ts';
import type { Comment, CommentSession } from './shared.ts';

/**
 * Comments on text ranges in plain ProseMirror (the stock column of the adapter). The comments
 * are inline decorations (class `tu-comment`, id in the spec) in plugin state, mapped through
 * `tr.mapping`; the file offsets are converted to document positions and back by the adapter's
 * `stockBody()` (its `PositionMap` and `Tracker`). Edge rule: the decorations are not inclusive,
 * so text typed at the start or at the end of a range stays outside it.
 *
 * Two things ProseMirror does not do for plugin state, and what this probe does about them:
 * - A comment whose text was deleted entirely is dropped by the mapping. The plugin keeps it in a
 *   "grave" (position and length where it vanished, mapped through later changes); a history
 *   transaction that puts back text of the same length at that spot brings the comment back.
 * - `prosemirror-history` has no hook for plugin state, so an undo of an add or a remove has
 *   nothing to undo. The session keeps its own small stack of comment actions and uses it when no
 *   document edit was made since the action.
 */

interface Grave {
  id: string;
  pos: number;
  length: number;
}

interface CommentState {
  set: DecorationSet;
  grave: Grave[];
}

/** What a transaction asks of the plugin, in document positions. */
interface CommentMeta {
  add?: Comment[];
  remove?: string;
}

const key = new PluginKey<CommentState>('tu-comments');

const decoration = (c: Comment): Decoration => Decoration.inline(c.from, c.to, { class: 'tu-comment' }, { id: c.id });
const idOf = (d: Decoration): string => (d.spec as { id: string }).id;

/** The comments in document positions. */
export function listPositions(state: EditorState): Comment[] {
  const set = key.getState(state)?.set ?? DecorationSet.empty;
  return set.find().map((d) => ({ id: idOf(d), from: d.from, to: d.to })).sort((a, b) => a.from - b.from);
}

/** The plugin: the decoration set, its mapping, and the grave of dropped comments. */
export function commentPlugin(): Plugin<CommentState> {
  return new Plugin<CommentState>({
    key,
    state: {
      init: () => ({ set: DecorationSet.empty, grave: [] }),
      apply(tr, value) {
        let { set, grave } = value;
        if (tr.docChanged) {
          const mapped = set.map(tr.mapping, tr.doc);
          const kept = new Set(mapped.find().map(idOf));
          const revived: Comment[] = [];
          if (isHistoryTransaction(tr)) {
            for (const g of grave) {
              const from = tr.mapping.map(g.pos, -1);
              const to = tr.mapping.map(g.pos, 1);
              if (to - from === g.length) revived.push({ id: g.id, from, to });
            }
          }
          const revivedIds = new Set(revived.map((c) => c.id));
          grave = grave.filter((g) => !revivedIds.has(g.id)).map((g) => ({ ...g, pos: tr.mapping.map(g.pos) }));
          for (const d of set.find()) if (!kept.has(idOf(d))) grave.push({ id: idOf(d), pos: tr.mapping.map(d.from), length: d.to - d.from });
          set = mapped.add(tr.doc, revived.map(decoration));
        }
        const meta = tr.getMeta(key) as CommentMeta | undefined;
        if (meta?.add !== undefined) set = set.add(tr.doc, meta.add.map(decoration));
        if (meta?.remove !== undefined) set = set.remove(set.find().filter((d) => idOf(d) === meta.remove));
        return { set, grave };
      },
    },
    props: { decorations: (state) => key.getState(state)?.set },
  });
}

/** A comment action the session can take back: `depth` is the undo depth of the document when it happened. */
interface Action {
  depth: number;
  kind: 'add' | 'remove';
  comment: Comment;
}

/** A comment session on a ProseMirror state of the file's body, driven without a view. */
export class ProseMirrorComments implements CommentSession {
  state: EditorState;
  private readonly body: BodyModel = stockBody();
  private readonly frontMatter: string;
  private done: Action[] = [];
  private undone: Action[] = [];

  constructor(text: string) {
    const { frontMatter, body } = splitFrontMatter(text);
    this.frontMatter = frontMatter;
    const doc = parseBody(body);
    this.body.reset(text, frontMatter.length, doc);
    this.state = EditorState.create({ doc, plugins: [history(), commentPlugin()] });
  }

  /** The file as the editor saves it: front matter plus the serialized body. */
  text(): string {
    return this.body.save(this.state.doc, this.frontMatter);
  }

  /** The file text the offsets refer to (the position source of the adapter, not what is saved). */
  sourceText(): string {
    return this.body.source(this.state.doc);
  }

  add(comment: Comment): void {
    this.apply(this.state.tr.setMeta(key, { add: [this.inPositions(comment)] } satisfies CommentMeta));
    this.done.push({ depth: undoDepth(this.state), kind: 'add', comment });
    this.undone = [];
  }

  remove(id: string): void {
    const comment = this.list().find((c) => c.id === id);
    if (comment === undefined) return;
    this.apply(this.state.tr.setMeta(key, { remove: id } satisfies CommentMeta));
    this.done.push({ depth: undoDepth(this.state), kind: 'remove', comment });
    this.undone = [];
  }

  list(): Comment[] {
    return listPositions(this.state).map((c) => ({ id: c.id, from: this.body.toOffset(this.state.doc, c.from), to: this.body.toOffset(this.state.doc, c.to) }));
  }

  restore(comments: Comment[]): void {
    this.apply(this.state.tr.setMeta(key, { add: comments.map((c) => this.inPositions(c)) } satisfies CommentMeta));
  }

  edit(from: number, to: number, insert: string): void {
    this.apply(this.replacement(from, to, insert));
    this.undone = [];
  }

  external(from: number, to: number, insert: string): void {
    this.apply(this.replacement(from, to, insert).setMeta('addToHistory', false));
  }

  undo(): void {
    const last = this.done.at(-1);
    if (last !== undefined && last.depth === undoDepth(this.state)) {
      this.done.pop();
      this.invert(last);
      this.undone.push(last);
    } else undo(this.state, (tr) => this.apply(tr));
  }

  redo(): void {
    const last = this.undone.at(-1);
    if (last !== undefined && last.depth === undoDepth(this.state)) {
      this.undone.pop();
      this.invert({ ...last, kind: last.kind === 'add' ? 'remove' : 'add' });
      this.done.push(last);
    } else if (redoDepth(this.state) > 0) redo(this.state, (tr) => this.apply(tr));
  }

  private apply(tr: Transaction): void {
    this.state = this.state.apply(tr);
  }

  private inPositions(c: Comment): Comment {
    return { id: c.id, from: this.body.toPos(this.state.doc, c.from), to: this.body.toPos(this.state.doc, c.to) };
  }

  private replacement(from: number, to: number, insert: string): Transaction {
    const start = this.body.toPos(this.state.doc, from);
    const end = this.body.toPos(this.state.doc, to);
    return this.state.tr.insertText(insert, start, end);
  }

  /** Do the opposite of `action` (an add is removed, a remove is added again). */
  private invert(action: Action): void {
    const meta: CommentMeta = action.kind === 'add' ? { remove: action.comment.id } : { add: [this.inPositions(action.comment)] };
    this.apply(this.state.tr.setMeta(key, meta));
  }
}
