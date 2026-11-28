// The rows of the command palette (P11 §5.11): every command, in the order of `commands`, with
// its keys from the keymap as the OS writes them. A command without keys still has a row.

import { commands } from './commands';
import { keysOf, label, type CommandId, type Os } from './keymap';

/** A row of the palette. */
export interface PaletteRow {
  id: CommandId;
  label: string;
  /** The command's keys as `os` writes them, or `''`. */
  keys: string;
}

/** Every command as a palette row for `os`. */
export function paletteRows(os: Os): PaletteRow[] {
  return (Object.keys(commands) as CommandId[]).map((id) => ({
    id,
    label: commands[id].label,
    keys: keysOf(id)
      .map((keys) => label(keys, os))
      .join('  '),
  }));
}
