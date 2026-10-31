import { Language, defineLanguageFacet } from '@codemirror/language';
import { EditorState } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { parser } from '@lezer/markdown';
import { resolveAnchor } from '../../harness/anchor.ts';
import type { EditorContext, Mode, ProseAdapter, ScriptedEdit, TextChange } from '../../harness/types.ts';
import { applyEdit, selectBody } from './edits.ts';
import { applyExternal } from './external.ts';
import { bodyStart, protectFrontMatter } from './frontmatter.ts';
import { canUndo, clearHistory, historyExtension, undo } from './history.ts';
import { keymapExtension, rebind } from './keymap.ts';
import { modeExtension, switchMode } from './modes.ts';
import { spellcheck } from './spellcheck.ts';
import { theme } from './theme.ts';

const markdown = new Language(defineLanguageFacet(), parser, [], 'markdown');

/**
 * Prototype A: CodeMirror 6. The file is the document, front matter included, so offsets and
 * saved bytes are the editor's own; the base adds the modes, live preview, history and keymap.
 */
class CodeMirrorAdapter implements ProseAdapter {
  private view: EditorView | undefined;
  private ctx: EditorContext | undefined;

  mount(target: HTMLElement, text: string, ctx: EditorContext): void {
    this.ctx = ctx;
    this.view = new EditorView({
      parent: target,
      state: EditorState.create({
        doc: text,
        extensions: [
          markdown.extension,
          theme,
          spellcheck,
          bodyStart,
          protectFrontMatter,
          historyExtension,
          keymapExtension(ctx.keymap, this.save, selectBody),
          modeExtension('write'),
        ],
      }),
    });
  }

  text(): string {
    return this.editor().state.doc.toString();
  }

  apply(edit: ScriptedEdit): void {
    const view = this.editor();
    switch (edit.kind) {
      case 'external': {
        // Both anchors resolve against the original; the local insert may move the external one.
        const text = this.text();
        const local = resolveAnchor(text, edit.local.at);
        const external = resolveAnchor(text, edit.external.at);
        if (local === external) throw new Error('external: the local and the external anchor resolve to the same offset');
        applyEdit(view, { id: edit.id, kind: 'insert', at: edit.local.at, text: edit.local.text });
        const at = external > local ? external + edit.local.text.length : external;
        applyExternal(view, [{ from: at, to: at, insert: edit.external.text }]);
        return;
      }
      case 'undo-all':
        // Each edit is its own step; then everything is taken back, as the user would.
        for (const step of edit.edits) this.apply(step);
        for (let n = 0; n < 50 && this.canUndo(); n++) this.undo();
        return;
      default:
        applyEdit(view, edit);
    }
  }

  setMode(mode: Mode): void {
    switchMode(this.editor(), mode);
  }

  focus(): void {
    this.editor().focus();
  }

  caretTo(offset: number): void {
    const view = this.editor();
    const at = Math.min(Math.max(offset, 0), view.state.doc.length);
    view.dispatch({ selection: { anchor: at }, userEvent: 'select', scrollIntoView: true });
  }

  caret(): number {
    return this.editor().state.selection.main.head;
  }

  applyExternal(changes: TextChange[]): void {
    applyExternal(this.editor(), changes);
  }

  undo(): void {
    undo(this.editor());
  }

  canUndo(): boolean {
    return canUndo(this.editor().state);
  }

  markSaved(): void {
    clearHistory(this.editor());
  }

  updateContext(ctx: EditorContext): void {
    const keysChanged = JSON.stringify(ctx.keymap) !== JSON.stringify(this.ctx?.keymap);
    this.ctx = ctx;
    if (keysChanged) this.editor().dispatch({ effects: rebind(ctx.keymap, this.save, selectBody) });
  }

  destroy(): void {
    this.view?.destroy();
    this.view = undefined;
    this.ctx = undefined;
  }

  /** The host's save, read when the key is pressed so that a new context is the one asked. */
  private readonly save = (): void => this.ctx?.save();

  private editor(): EditorView {
    if (this.view === undefined) throw new Error('codemirror: the adapter is not mounted');
    return this.view;
  }
}

export function createCodeMirrorAdapter(): ProseAdapter {
  return new CodeMirrorAdapter();
}
