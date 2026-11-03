import { selectAll } from 'prosemirror-commands';
import { EditorState, TextSelection, type Plugin, type Transaction } from 'prosemirror-state';
import { EditorView } from 'prosemirror-view';
import type { EditorContext, Mode, ProseAdapter, ScriptedEdit, TextChange } from '../../harness/types.ts';
import { BeatAttrsView, BeatHeadingView } from './beatview.ts';
import { stockBody, type BodyModel } from './body.ts';
import { beatRaw } from './beatraw.ts';
import { CalloutView } from './calloutview.ts';
import { calloutRaw } from './calloutraw.ts';
import { devWords } from './devwords.ts';
import { applyEdit, type EditTarget } from './edits.ts';
import { applyChanges, externalTransaction } from './external.ts';
import { splitFrontMatter } from './frontmatter.ts';
import { canUndo, history, undo, withoutHistory } from './history.ts';
import { keymapPlugins } from './keymap.ts';
import { finishTypedLink } from './linkkeys.ts';
import { LinkView } from './linkview.ts';
import { livePreview } from './livepreview.ts';
import { parseBody } from './markdown.ts';
import { popoverPlugin } from './popover.ts';
import { PositionMap } from './positions.ts';
import { schema } from './schema.ts';
import { guard, selectSourceBody, sourceDoc, sourceState } from './source.ts';
import { installTheme } from './theme.ts';
import { typingPlugin } from './typing.ts';

const toLf = (text: string): string => text.replace(/\r\n?/g, '\n');

/**
 * Prototype B: plain ProseMirror with the stock `prosemirror-markdown` parser. What Write mode
 * saves comes from the body model (`body.ts`): in the stock column the body is a tree that kept no
 * source, so `text()` is the front matter plus what the serializer writes. Write mode keeps the
 * front matter outside the document; Source mode is the same view on a one-block document that
 * holds the whole file, front matter protected.
 */
export class ProseMirrorAdapter implements ProseAdapter, EditTarget {
  view!: EditorView;
  private ctx: EditorContext | undefined;
  private mode: Mode = 'write';
  private frontMatter = '';
  /** Source mode: how many characters at the start of the file are the protected front matter. */
  private protectedEnd = 0;
  private historyPlugin: Plugin = history();
  /** The chips on the page, so a new index can redraw them without a new editor. */
  private readonly chips = new Set<LinkView>();
  /** The callouts on the page, so a new index can redraw their heads. */
  private readonly callouts = new Set<CalloutView>();
  /** The beat rows on the page, redrawn by a new index or new findings. */
  private readonly beatRows = new Set<BeatAttrsView>();

  /** `body`: what Write mode saves and what the offsets refer to; the stock column by default. */
  constructor(private readonly body: BodyModel = stockBody()) {}

  mount(target: HTMLElement, text: string, ctx: EditorContext): void {
    installTheme();
    this.ctx = ctx;
    const file = toLf(text);
    const { frontMatter, body } = splitFrontMatter(file);
    this.frontMatter = frontMatter;
    const doc = parseBody(body);
    this.body.reset(file, frontMatter.length, doc);
    this.view = new EditorView(target, {
      state: this.stateOf(doc),
      // Only a body model that maps the changes wants the transactions; the stock view keeps the default dispatch.
      ...(this.body.observe === undefined ? {} : { dispatchTransaction: (tr: Transaction) => this.dispatch(tr) }),
      nodeViews: {
        wikilink: (node) => new LinkView(node, () => this.ctx!, this.chips),
        callout: (node, view, getPos) => new CalloutView(node, view, getPos, () => this.ctx!, this.callouts),
        beat_heading: (node, view, getPos) => new BeatHeadingView(node, view, getPos),
        beat_attrs: (node, view, getPos) => new BeatAttrsView(node, view, getPos, () => this.ctx!, this.beatRows),
      },
      attributes: (state) => (state.doc.type === schema.nodes.doc
        ? { class: 'pm-write', spellcheck: 'true' }
        : { class: 'pm-source-mode', spellcheck: 'false' }),
    });
  }

  text(): string {
    if (this.mode === 'source') return this.view.state.doc.textContent;
    return this.body.save(this.view.state.doc, this.frontMatter);
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
      this.body.reset(file, frontMatter.length, doc);
      this.mode = 'write';
      const state = this.stateOf(doc);
      const pos = this.body.toPos(doc, Math.max(offset, frontMatter.length));
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
    return this.body.toOffset(this.view.state.doc, head);
  }

  applyExternal(changes: TextChange[]): void {
    const { view } = this;
    if (this.mode === 'source') {
      const next = sourceDoc(applyChanges(view.state.doc.textContent, changes));
      const tr = externalTransaction(view.state, next);
      if (tr !== undefined) view.dispatch(tr);
      return;
    }
    const file = applyChanges(this.body.source(view.state.doc), changes);
    const { frontMatter, body } = splitFrontMatter(file);
    const tr = externalTransaction(view.state, parseBody(body));
    if (tr !== undefined) view.dispatch(tr);
    this.frontMatter = frontMatter;
    this.body.reset(file, frontMatter.length, view.state.doc);
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
    for (const chip of this.chips) chip.refresh();
    for (const callout of this.callouts) callout.refresh();
    for (const row of this.beatRows) row.refresh();
    // An open popover searches again with the new index.
    this.view.dispatch(this.view.state.tr.setMeta('p10-context', true));
  }

  destroy(): void {
    this.view?.destroy();
    this.ctx = undefined;
  }

  // ── EditTarget ───────────────────────────────────────────────────────────

  sourceText(): string {
    if (this.mode === 'source') return this.view.state.doc.textContent;
    return this.body.source(this.view.state.doc);
  }

  toPos(offset: number): number {
    if (this.mode === 'source') return Math.min(Math.max(offset, 0), this.view.state.doc.textContent.length) + 1;
    return this.body.toPos(this.view.state.doc, offset);
  }

  bodyStart(): number {
    return this.mode === 'source' ? this.protectedEnd : this.body.start();
  }

  writing(): boolean {
    return this.mode === 'write';
  }

  /** The dispatch of a view whose body model observes: every Write-mode transaction that changed the document goes to it. */
  private dispatch(tr: Transaction): void {
    const { state, transactions } = this.view.state.applyTransaction(tr);
    if (this.mode === 'write') for (const applied of transactions) if (applied.docChanged) this.body.observe?.(applied);
    this.view.updateState(state);
  }

  // ── Plugins ──────────────────────────────────────────────────────────────

  /** The plugins of the current mode; the history plugin is the same instance until `markSaved`. */
  private plugins(): Plugin[] {
    const write = this.mode === 'write';
    const selectBody = write ? selectAll : selectSourceBody(this.protectedEnd);
    return [
      this.historyPlugin,
      ...(write ? calloutRaw() : []),
      ...(write ? beatRaw() : []),
      ...keymapPlugins(this.ctx!.keymap, () => this.ctx?.save(), selectBody),
      ...(write ? [finishTypedLink(), typingPlugin(), popoverPlugin(() => this.ctx!), livePreview(), devWords(() => this.ctx?.developerWords ?? [])] : [guard(this.protectedEnd)]),
    ];
  }

  private stateOf(doc: EditorState['doc']): EditorState {
    return EditorState.create({ doc, plugins: this.plugins() });
  }
}

export function createProseMirrorAdapter(): ProseAdapter {
  return new ProseMirrorAdapter();
}
