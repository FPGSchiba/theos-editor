import { selectAll } from 'prosemirror-commands';
import { EditorState, TextSelection, type Plugin } from 'prosemirror-state';
import { EditorView } from 'prosemirror-view';
import type { EditorContext, Mode, ProseAdapter, ScriptedEdit, TextChange } from '../../harness/types.ts';
import { applyEdit, type EditTarget } from './edits.ts';
import { applyChanges, externalTransaction } from './external.ts';
import { joinFrontMatter, splitFrontMatter } from './frontmatter.ts';
import { canUndo, history, undo, withoutHistory } from './history.ts';
import { keymapPlugins } from './keymap.ts';
import { livePreview } from './livepreview.ts';
import { parseBody, serializeBody } from './markdown.ts';
import { PositionMap, Tracker } from './positions.ts';
import { schema } from './schema.ts';
import { guard, selectSourceBody, sourceDoc, sourceState } from './source.ts';
import { installTheme } from './theme.ts';

const toLf = (text: string): string => text.replace(/\r\n?/g, '\n');

/**
 * Prototype B, stock column: plain ProseMirror with the stock `prosemirror-markdown` parser and
 * serializer. The body is a tree that kept no source, so `text()` is the front matter plus what
 * the serializer writes. Write mode keeps the front matter outside the document; Source mode is
 * the same view on a one-block document that holds the whole file, front matter protected.
 */
class ProseMirrorAdapter implements ProseAdapter, EditTarget {
  view!: EditorView;
  private ctx: EditorContext | undefined;
  private mode: Mode = 'write';
  private frontMatter = '';
  private tracker!: Tracker;
  /** Source mode: how many characters at the start of the file are the protected front matter. */
  private protectedEnd = 0;
  private historyPlugin: Plugin = history();

  mount(target: HTMLElement, text: string, ctx: EditorContext): void {
    installTheme();
    this.ctx = ctx;
    const file = toLf(text);
    const { frontMatter, body } = splitFrontMatter(file);
    this.frontMatter = frontMatter;
    const doc = parseBody(body);
    this.tracker = new Tracker(file, frontMatter.length, doc);
    this.view = new EditorView(target, {
      state: this.stateOf(doc),
      attributes: (state) => (state.doc.type === schema.nodes.doc
        ? { class: 'pm-write', spellcheck: 'true' }
        : { class: 'pm-source-mode', spellcheck: 'false' }),
    });
  }

  text(): string {
    if (this.mode === 'source') return this.view.state.doc.textContent;
    return joinFrontMatter(this.frontMatter, serializeBody(this.view.state.doc));
  }

  apply(edit: ScriptedEdit): void {
    applyEdit(this, edit);
  }

  setMode(mode: Mode): void {
    if (mode === this.mode) return;
    const { view } = this;
    if (mode === 'source') {
      const text = this.text();
      // The caret goes where it stands in the serialized text, so the map is built on that.
      const map = new PositionMap(text, this.frontMatter.length, view.state.doc);
      const head = map.toOffset(view.state.selection.head);
      this.protectedEnd = this.frontMatter.length;
      this.mode = 'source';
      view.updateState(sourceState(text, this.plugins(), head));
    } else {
      const offset = view.state.selection.head - 1;
      const file = view.state.doc.textContent;
      const { frontMatter, body } = splitFrontMatter(file);
      this.frontMatter = frontMatter;
      const doc = parseBody(body);
      this.tracker.reset(file, frontMatter.length, doc);
      this.mode = 'write';
      const state = this.stateOf(doc);
      const pos = this.tracker.toPos(Math.max(offset, frontMatter.length));
      view.updateState(state.apply(state.tr.setSelection(TextSelection.near(state.doc.resolve(pos)))));
    }
  }

  focus(): void {
    this.view.focus();
  }

  caretTo(offset: number): void {
    const { view } = this;
    const pos = this.toPos(offset);
    const doc = view.state.doc;
    view.dispatch(view.state.tr.setSelection(TextSelection.near(doc.resolve(Math.min(pos, doc.content.size)))).scrollIntoView());
  }

  caret(): number {
    const head = this.view.state.selection.head;
    if (this.mode === 'source') return head - 1;
    this.tracker.sync(this.view.state.doc);
    return this.tracker.toOffset(head);
  }

  applyExternal(changes: TextChange[]): void {
    const { view } = this;
    if (this.mode === 'source') {
      const next = sourceDoc(applyChanges(view.state.doc.textContent, changes));
      const tr = externalTransaction(view.state, next);
      if (tr !== undefined) view.dispatch(tr);
      return;
    }
    this.tracker.sync(view.state.doc);
    const file = applyChanges(this.tracker.text(), changes);
    const { frontMatter, body } = splitFrontMatter(file);
    const tr = externalTransaction(view.state, parseBody(body));
    if (tr !== undefined) view.dispatch(tr);
    this.frontMatter = frontMatter;
    this.tracker.reset(file, frontMatter.length, view.state.doc);
  }

  undo(): void {
    undo(this.view.state, this.view.dispatch);
  }

  canUndo(): boolean {
    return canUndo(this.view.state);
  }

  markSaved(): void {
    this.historyPlugin = history();
    this.view.updateState(withoutHistory(this.view.state, this.plugins()));
  }

  updateContext(ctx: EditorContext): void {
    const keysChanged = JSON.stringify(ctx.keymap) !== JSON.stringify(this.ctx?.keymap);
    this.ctx = ctx;
    // The history and the other plugins keep their instances: only the keymap is swapped.
    if (keysChanged) this.view.updateState(this.view.state.reconfigure({ plugins: this.plugins() }));
  }

  destroy(): void {
    this.view?.destroy();
    this.ctx = undefined;
  }

  // ── EditTarget ───────────────────────────────────────────────────────────

  sourceText(): string {
    if (this.mode === 'source') return this.view.state.doc.textContent;
    this.tracker.sync(this.view.state.doc);
    return this.tracker.text();
  }

  toPos(offset: number): number {
    if (this.mode === 'source') return Math.min(Math.max(offset, 0), this.view.state.doc.textContent.length) + 1;
    this.tracker.sync(this.view.state.doc);
    return this.tracker.toPos(offset);
  }

  bodyStart(): number {
    return this.mode === 'source' ? this.protectedEnd : this.tracker.start();
  }

  writing(): boolean {
    return this.mode === 'write';
  }

  // ── Plugins ──────────────────────────────────────────────────────────────

  /** The plugins of the current mode; the history plugin is the same instance until `markSaved`. */
  private plugins(): Plugin[] {
    const write = this.mode === 'write';
    const selectBody = write ? selectAll : selectSourceBody(this.protectedEnd);
    return [
      this.historyPlugin,
      ...keymapPlugins(this.ctx!.keymap, () => this.ctx?.save(), selectBody),
      ...(write ? [livePreview()] : [guard(this.protectedEnd)]),
    ];
  }

  private stateOf(doc: EditorState['doc']): EditorState {
    return EditorState.create({ doc, plugins: this.plugins() });
  }
}

export function createProseMirrorAdapter(): ProseAdapter {
  return new ProseMirrorAdapter();
}
