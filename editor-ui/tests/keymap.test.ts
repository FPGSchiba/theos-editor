// The one keymap (P11 §5.11): `Mod` is Ctrl on Windows and Linux and Cmd on macOS, Ctrl+Tab is
// Ctrl everywhere, no two commands share keys on one OS, and every command with keys exists.

import { expect, test } from 'vitest';

import { commands } from '../src/lib/commands';
import {
  asOs,
  keyLabel,
  keymap,
  label,
  match,
  type KeyPress,
  type Os,
} from '../src/lib/keymap';

const OSES: readonly Os[] = ['windows', 'macos', 'linux'];

/** A key press with no modifier but those named. */
function press(key: string, code: string, mods: Partial<KeyPress> = {}): KeyPress {
  return { key, code, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...mods };
}

// AC: CE-32.2
test('Mod is Control on windows and linux and Meta on macos', () => {
  const ctrlK = press('k', 'KeyK', { ctrlKey: true });
  const metaK = press('k', 'KeyK', { metaKey: true });
  expect(match(ctrlK, 'windows')).toBe('palette.open');
  expect(match(ctrlK, 'linux')).toBe('palette.open');
  expect(match(ctrlK, 'macos')).toBeNull();
  expect(match(metaK, 'macos')).toBe('palette.open');
  expect(match(metaK, 'windows')).toBeNull();
  expect(match(metaK, 'linux')).toBeNull();
});

// AC: CE-32.2
test('Ctrl+Tab is Control on all three', () => {
  for (const os of OSES) {
    expect(match(press('Tab', 'Tab', { ctrlKey: true }), os)).toBe('tab.next');
    expect(match(press('Tab', 'Tab', { ctrlKey: true, shiftKey: true }), os)).toBe('tab.previous');
    expect(match(press('Tab', 'Tab', { metaKey: true }), os)).toBeNull();
  }
});

// AC: CE-32.2
test('no two commands share keys on any OS', () => {
  for (const os of OSES) {
    const seen = new Map<string, string>();
    for (const { command, keys } of keymap) {
      for (const combo of keys) {
        const written = label(combo, os).toLowerCase();
        expect(seen.get(written), `${written} on ${os}`).toBeUndefined();
        seen.set(written, command);
      }
    }
  }
});

// AC: CE-32.2
test('every command with keys is in commands', () => {
  for (const { command } of keymap) {
    expect(commands[command], command).toBeDefined();
  }
});

// AC: CE-32.2
test('labels are Ctrl+K on windows and linux and ⌘K on macos', () => {
  expect(label('Mod+K', 'windows')).toBe('Ctrl+K');
  expect(label('Mod+K', 'linux')).toBe('Ctrl+K');
  expect(label('Mod+K', 'macos')).toBe('⌘K');
  expect(label('Mod+Shift+P', 'windows')).toBe('Ctrl+Shift+P');
  expect(label('Mod+Shift+P', 'macos')).toBe('⇧⌘P');
  expect(label('Ctrl+Tab', 'macos')).toBe('⌃Tab');
  expect(label('Ctrl+Shift+Tab', 'macos')).toBe('⌃⇧Tab');
  expect(keyLabel('file.save', 'windows')).toBe('Ctrl+S');
  expect(keyLabel('project.open', 'macos')).toBe('⌘O');
});

test('digits match by their key and, under Shift, by their code', () => {
  const two = press('2', 'Digit2', { ctrlKey: true });
  expect(match(two, 'windows')).toBe('rail.2');
  const shiftThree = press('§', 'Digit3', { ctrlKey: true, shiftKey: true });
  expect(match(shiftThree, 'windows')).toBe('tree.lore');
});

// AC: CE-32.2
test('a letter matches by the key it produced, not by its physical key', () => {
  // Dvorak: the physical P key gives `r`, so Ctrl on it is Reload, not Find record.
  expect(match(press('r', 'KeyP', { ctrlKey: true }), 'windows')).toBe('project.reload');
  // Dvorak: the physical W key gives `,`, which is no command.
  expect(match(press(',', 'KeyW', { ctrlKey: true }), 'windows')).toBeNull();
  // A non-Latin layout produces no Latin letter, so the physical key counts.
  expect(match(press('л', 'KeyK', { ctrlKey: true }), 'windows')).toBe('palette.open');
  // AZERTY: the digit row writes `&` for 1; digits count by their place on the row.
  expect(match(press('&', 'Digit1', { ctrlKey: true }), 'windows')).toBe('rail.1');
});

test('an unknown modifier or a missing key is an error, not a silent Shift', () => {
  expect(() => label('Hyper+K', 'windows')).toThrow(/unknown modifier `Hyper`/);
  expect(() => label('Mod+', 'windows')).toThrow(/names no key/);
});

test('an extra modifier is no match', () => {
  expect(match(press('k', 'KeyK', { ctrlKey: true, altKey: true }), 'windows')).toBeNull();
  expect(match(press('k', 'KeyK'), 'windows')).toBeNull();
  expect(match(press('Control', 'ControlLeft', { ctrlKey: true }), 'windows')).toBeNull();
});

test('an unknown OS is read as windows', () => {
  expect(asOs('macos')).toBe('macos');
  expect(asOs('linux')).toBe('linux');
  expect(asOs('freebsd')).toBe('windows');
});
