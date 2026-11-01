import { baseKeymap } from 'prosemirror-commands';
import { keymap } from 'prosemirror-keymap';
import type { Command, Plugin } from 'prosemirror-state';
import type { Keymap } from '../../harness/types.ts';
import { redo, undo } from './history.ts';

/**
 * The shortcuts of the host (`ctx.keymap`, `Mod-` notation) go straight into prosemirror-keymap;
 * the base commands (Enter, Backspace, arrows) come behind them.
 */
export function keymapPlugins(keys: Keymap, save: () => void, selectBody: Command): Plugin[] {
  return [
    keymap({
      [keys.save]: () => {
        save();
        return true;
      },
      [keys.undo]: undo,
      [keys.redo]: redo,
      [keys.selectAll]: selectBody,
    }),
    keymap(baseKeymap),
  ];
}
