import { splitFrontMatter } from '../../harness/frontmatter.ts';
import { editOffset, expectedAfter, toLf } from '../../harness/oracle.ts';
import type {
  Anchor, EditorContext, Mode, ProseAdapter, ScriptedEdit, TextChange,
} from '../../harness/types.ts';
import { diffStep, History, mapPosition } from './history.ts';

/** Does the key event match a spec like `Mod-s` or `Mod-Shift-z` (Mod is Ctrl, or Cmd on macOS)? */
function matches(event: KeyboardEvent, spec: string): boolean {
  const parts = spec.split('-');
  const key = parts.pop() ?? '';
  const wants = (name: string) => parts.includes(name);
  return (
    event.key.toLowerCase() === key.toLowerCase()
    && (event.ctrlKey || event.metaKey) === wants('Mod')
    && event.shiftKey === wants('Shift')
    && event.altKey === wants('Alt')
  );
}

/**
 * The change an edit stands for, found with the oracle's own logic: the text the oracle expects,
 * compared with the text now, from the offset where the oracle says the edit starts.
 */
function changeFor(before: string, edit: ScriptedEdit): TextChange | undefined {
  const after = expectedAfter(before, edit);
  if (after === before) return undefined;
  const from = Math.min(editOffset(before, edit) ?? 0, before.length, after.length);
  const room = Math.min(before.length, after.length) - from;
  let end = 0;
  while (end < room && before[before.length - 1 - end] === after[after.length - 1 - end]) end++;
  return { from, to: before.length - end, insert: after.slice(from, after.length - end) };
}

/**
 * The baseline: a plain `<textarea>` that shows the whole file raw. It keeps bytes by
 * construction (a textarea only turns CRLF into LF), so a suite that fails on it points at the
 * harness, not at a prototype. `apply` takes the oracle's own change for every edit kind.
 */
class BaselineAdapter implements ProseAdapter {
  private area: HTMLTextAreaElement | undefined;
  private ctx: EditorContext | undefined;
  private history = new History();
  private last = '';
  private mode: Mode = 'write';

  mount(target: HTMLElement, text: string, ctx: EditorContext): void {
    const area = document.createElement('textarea');
    area.className = 'hx-baseline';
    area.dataset.mode = this.mode;
    area.value = text; // a textarea turns CRLF into LF by itself
    area.addEventListener('input', this.onInput);
    area.addEventListener('keydown', this.onKeydown);
    area.addEventListener('paste', this.onPaste);
    target.append(area);
    this.area = area;
    this.ctx = ctx;
    this.last = area.value;
  }

  text(): string {
    return this.field().value;
  }

  apply(edit: ScriptedEdit): void {
    switch (edit.kind) {
      case 'external': {
        const before = this.text();
        expectedAfter(before, edit); // throws when the two anchors collide
        const insert = (part: { at: Anchor; text: string }): ScriptedEdit => (
          { id: edit.id, kind: 'insert', at: part.at, text: part.text }
        );
        const local = changeFor(before, insert(edit.local));
        const external = changeFor(before, insert(edit.external));
        if (local !== undefined) this.change(local);
        // Both anchors resolve against the original; the local insert may have moved the other.
        if (external !== undefined && local !== undefined) {
          this.applyExternal([{
            from: mapPosition(external.from, local), to: mapPosition(external.to, local), insert: external.insert,
          }]);
        } else if (external !== undefined) {
          this.applyExternal([external]);
        }
        return;
      }
      case 'undo-all': {
        // Each edit is its own undo step; then everything is taken back, as the user would.
        for (const step of edit.edits) this.apply(step);
        for (let n = 0; n < 50 && this.canUndo(); n++) this.undo();
        return;
      }
      default: {
        const change = changeFor(this.text(), edit);
        if (change !== undefined) this.change(change);
      }
    }
  }

  setMode(mode: Mode): void {
    // The textarea shows the whole file raw in both modes.
    this.mode = mode;
    if (this.area !== undefined) this.area.dataset.mode = mode;
  }

  focus(): void {
    this.field().focus();
  }

  caretTo(offset: number): void {
    this.field().setSelectionRange(offset, offset);
  }

  caret(): number {
    const area = this.field();
    return area.selectionDirection === 'backward' ? area.selectionStart : area.selectionEnd;
  }

  applyExternal(changes: TextChange[]): void {
    const area = this.field();
    let start = area.selectionStart;
    let end = area.selectionEnd;
    // Last change first, so the offsets of the earlier ones stay valid.
    for (const change of [...changes].sort((a, b) => b.from - a.from)) {
      area.setRangeText(change.insert, change.from, change.to, 'preserve');
      start = mapPosition(start, change);
      end = mapPosition(end, change);
      this.history.rebase(change);
    }
    area.setSelectionRange(start, end, area.selectionDirection);
    this.last = area.value;
  }

  undo(): void {
    this.restore(this.history.undo(this.text()));
  }

  canUndo(): boolean {
    return this.history.canUndo();
  }

  markSaved(): void {
    this.history.clear();
  }

  updateContext(ctx: EditorContext): void {
    this.ctx = ctx;
  }

  destroy(): void {
    const area = this.area;
    if (area === undefined) return;
    area.removeEventListener('input', this.onInput);
    area.removeEventListener('keydown', this.onKeydown);
    area.removeEventListener('paste', this.onPaste);
    area.remove();
    this.area = undefined;
    this.ctx = undefined;
  }

  private field(): HTMLTextAreaElement {
    if (this.area === undefined) throw new Error('baseline: the adapter is not mounted');
    return this.area;
  }

  /** A local change made by the harness or a paste: into the text, onto the undo stack. */
  private change(change: TextChange): void {
    const area = this.field();
    const removed = area.value.slice(change.from, change.to);
    area.setRangeText(change.insert, change.from, change.to, 'end');
    this.history.record({ from: change.from, removed, inserted: change.insert });
    this.last = area.value;
  }

  private restore(result: { text: string; caret: number } | undefined): void {
    if (result === undefined) return;
    const area = this.field();
    area.value = result.text;
    area.setSelectionRange(result.caret, result.caret);
    this.last = area.value;
  }

  /** Typing, deleting, drag and drop: whatever the browser did to the value becomes one step. */
  private readonly onInput = (): void => {
    const area = this.field();
    const step = diffStep(this.last, area.value);
    if (step !== undefined) this.history.record(step);
    this.last = area.value;
  };

  private readonly onKeydown = (event: KeyboardEvent): void => {
    const area = this.field();
    const keymap = this.ctx?.keymap;
    if (keymap === undefined) return;
    const bodyStart = splitFrontMatter(area.value).frontMatter.length;
    if (matches(event, keymap.save)) {
      event.preventDefault();
      this.ctx?.save();
    } else if (matches(event, keymap.undo)) {
      event.preventDefault();
      this.undo();
    } else if (matches(event, keymap.redo)) {
      event.preventDefault();
      this.restore(this.history.redo(this.text()));
    } else if (matches(event, keymap.selectAll)) {
      // Select-all takes the body only; the front matter is not the author's text.
      event.preventDefault();
      area.setSelectionRange(bodyStart, area.value.length);
    } else if (
      event.key === 'Backspace' && area.selectionStart === area.selectionEnd && area.selectionStart <= bodyStart
    ) {
      // Backspace never reaches into the front matter (oracle, `delete` with `backward`).
      event.preventDefault();
    }
  };

  /** A paste inserts its plain text with LF line ends; rich HTML is ignored. */
  private readonly onPaste = (event: ClipboardEvent): void => {
    const plain = event.clipboardData?.getData('text/plain');
    if (plain === undefined) return;
    event.preventDefault();
    const area = this.field();
    this.change({ from: area.selectionStart, to: area.selectionEnd, insert: toLf(plain) });
  };
}

export function createBaselineAdapter(): ProseAdapter {
  return new BaselineAdapter();
}
