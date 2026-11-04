import type { Keymap } from './types.ts';

/** The one keymap of the harness; every prototype reads its shortcuts from `ctx.keymap`. */
export const keymap: Keymap = { save: 'Mod-s', undo: 'Mod-z', redo: 'Mod-Shift-z', selectAll: 'Mod-a' };
