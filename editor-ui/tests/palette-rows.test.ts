// The palette lists every command (P11 §5.11), with its keys from the keymap; a command without
// keys still has a row.

import { expect, test } from 'vitest';

import { commands } from '../src/lib/commands';
import { keymap } from '../src/lib/keymap';
import { paletteRows } from '../src/lib/palette-rows';

// AC: CE-32.2
test('every command in commands appears in the palette', () => {
  for (const os of ['windows', 'macos', 'linux'] as const) {
    const rows = paletteRows(os);
    expect(rows.map((row) => row.id)).toEqual(Object.keys(commands));
    expect(rows.map((row) => row.label)).toEqual(Object.values(commands).map((c) => c.label));
  }
});

// AC: CE-32.2
test('a row shows every combination of its command', () => {
  const palette = paletteRows('windows').find((row) => row.id === 'palette.open');
  expect(palette?.keys).toBe('Ctrl+K  Ctrl+Shift+P');
  const mac = paletteRows('macos').find((row) => row.id === 'palette.open');
  expect(mac?.keys).toBe('⌘K  ⇧⌘P');
});

test('every keymap command has a row', () => {
  const ids = new Set(paletteRows('windows').map((row) => row.id));
  for (const { command } of keymap) {
    expect(ids.has(command), command).toBe(true);
  }
});
