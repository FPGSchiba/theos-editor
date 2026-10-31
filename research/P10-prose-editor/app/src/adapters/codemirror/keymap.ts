import { deleteCharBackwardStrict, deleteCharForward, insertNewline, standardKeymap } from '@codemirror/commands';
import { Compartment, type Extension, type StateEffect } from '@codemirror/state';
import { keymap, type Command } from '@codemirror/view';
import type { Keymap } from '../../harness/types.ts';
import { redo, undo } from './history.ts';

/** The shortcuts of the host (`ctx.keymap`) are in a compartment: the host may hand in a new keymap. */
const slot = new Compartment();

/**
 * Plain prose editing: Enter and Backspace add and remove only what was typed (no indent copied,
 * no indent unit deleted), so bytes the author did not touch stay as they are.
 */
function bindings(keys: Keymap, save: () => void, selectBody: Command): Extension {
  return keymap.of([
    { key: keys.save, run: () => { save(); return true; }, preventDefault: true },
    { key: keys.undo, run: undo, preventDefault: true },
    { key: keys.redo, run: redo, preventDefault: true },
    { key: keys.selectAll, run: selectBody, preventDefault: true },
    { key: 'Enter', run: insertNewline, shift: insertNewline },
    { key: 'Backspace', run: deleteCharBackwardStrict, shift: deleteCharBackwardStrict, preventDefault: true },
    { key: 'Delete', run: deleteCharForward, preventDefault: true },
    ...standardKeymap,
  ]);
}

export const keymapExtension = (keys: Keymap, save: () => void, selectBody: Command): Extension =>
  slot.of(bindings(keys, save, selectBody));

/** The effect that swaps the shortcuts in a running editor. */
export const rebind = (keys: Keymap, save: () => void, selectBody: Command): StateEffect<unknown> =>
  slot.reconfigure(bindings(keys, save, selectBody));
