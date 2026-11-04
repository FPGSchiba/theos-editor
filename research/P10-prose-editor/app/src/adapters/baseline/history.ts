import type { TextChange } from '../../harness/types.ts';

/**
 * One local edit, kept so that it can be taken back. `inserted` is what the edit put at `from`
 * (it sits at `from` in the text as long as the step is on the undo stack); `removed` is what
 * it replaced.
 */
export interface Step {
  from: number;
  removed: string;
  inserted: string;
}

/** The step that turns `before` into `after`: the text between the common prefix and suffix. */
export function diffStep(before: string, after: string): Step | undefined {
  if (before === after) return undefined;
  const room = Math.min(before.length, after.length);
  let start = 0;
  while (start < room && before[start] === after[start]) start++;
  let end = 0;
  while (end < room - start && before[before.length - 1 - end] === after[after.length - 1 - end]) end++;
  return {
    from: start,
    removed: before.slice(start, before.length - end),
    inserted: after.slice(start, after.length - end),
  };
}

/** Where a position moves when `change` happens: unchanged at the change, shifted behind it. */
export function mapPosition(position: number, change: TextChange): number {
  if (position <= change.from) return position;
  if (position >= change.to) return position + change.insert.length - (change.to - change.from);
  return change.from + change.insert.length;
}

/**
 * The undo history of the baseline: an undo and a redo stack of local steps, nothing coalesced.
 * An external change is rebased through the undo stack, so undo still removes only the local
 * edit (spec §4.6 and the integration checks); it drops the redo stack.
 */
export class History {
  private undoStack: Step[] = [];
  private redoStack: Step[] = [];

  /** A new local edit: it can be undone, and what was undone before can no longer be redone. */
  record(step: Step): void {
    this.undoStack.push(step);
    this.redoStack = [];
  }

  canUndo(): boolean {
    return this.undoStack.length > 0;
  }

  canRedo(): boolean {
    return this.redoStack.length > 0;
  }

  /** Take back the latest local step in `text`; returns the new text and the caret. */
  undo(text: string): { text: string; caret: number } | undefined {
    const step = this.undoStack.pop();
    if (step === undefined) return undefined;
    this.redoStack.push({ from: step.from, removed: step.inserted, inserted: step.removed });
    return this.apply(text, step.from, step.inserted.length, step.removed);
  }

  redo(text: string): { text: string; caret: number } | undefined {
    const step = this.redoStack.pop();
    if (step === undefined) return undefined;
    this.undoStack.push({ from: step.from, removed: step.inserted, inserted: step.removed });
    return this.apply(text, step.from, step.inserted.length, step.removed);
  }

  /**
   * An external change, given in the coordinates of the text as it is now. Every step on the
   * undo stack is moved so that it still points at its own text; a step the change overlaps,
   * and every step below it, can no longer be placed and is dropped.
   */
  rebase(external: TextChange): void {
    this.redoStack = [];
    let change = external;
    for (let index = this.undoStack.length - 1; index >= 0; index--) {
      const step = this.undoStack[index]!;
      const end = step.from + step.inserted.length;
      const delta = change.insert.length - (change.to - change.from);
      if (change.to <= step.from) {
        step.from += delta;
      } else if (change.from >= end) {
        // Undoing this step moves everything behind it, so the change moves with it.
        const shift = step.inserted.length - step.removed.length;
        change = { from: change.from - shift, to: change.to - shift, insert: change.insert };
      } else {
        this.undoStack.splice(0, index + 1);
        return;
      }
    }
  }

  /** Forget everything: the field was saved (CE-34.3). */
  clear(): void {
    this.undoStack = [];
    this.redoStack = [];
  }

  private apply(text: string, from: number, length: number, insert: string): { text: string; caret: number } {
    return { text: text.slice(0, from) + insert + text.slice(from + length), caret: from + insert.length };
  }
}
