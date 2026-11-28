// The one keymap (P11 §5.11): the only place a key combination is written. The palette, the
// toolbar's "Save Ctrl+S" and P12's CodeMirror keymap read this table.
//
// `Mod` is Ctrl on Windows and Linux and Cmd on macOS; `Ctrl` is Ctrl on every OS (Ctrl+Tab, as
// in browsers). The OS comes from `app_info().os`, not from the browser, so tests can set it.
// `tinykeys` reads its `$mod` from `navigator.platform` once, at load, and skips input fields, so
// `match` is our own: a combination is its modifiers and one key, and a press matches it when it
// has exactly those modifiers and the key (see `sameKey` for keyboard layouts).

/** The OS the app runs on, from `app_info().os`. */
export type Os = 'windows' | 'macos' | 'linux';

/** The rail entries that have a key: Mod+1 … Mod+9. */
export type RailSlot = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;

/** A command of the editor (spec §5.11's table). */
export type CommandId =
  | 'palette.open'
  | 'record.find'
  | 'project.open'
  | 'project.reload'
  | `rail.${RailSlot}`
  | 'tab.next'
  | 'tab.previous'
  | 'tab.close'
  | 'tree.where'
  | 'tree.who'
  | 'tree.lore'
  | 'tree.things'
  | 'file.save';

/** A command and its key combinations, such as `Mod+Shift+P`. */
export interface Binding {
  command: CommandId;
  keys: string[];
}

const RAIL_SLOTS: readonly RailSlot[] = [1, 2, 3, 4, 5, 6, 7, 8, 9];

/** Every command's keys, in the palette's order. */
export const keymap: ReadonlyArray<Binding> = [
  { command: 'palette.open', keys: ['Mod+K', 'Mod+Shift+P'] },
  { command: 'record.find', keys: ['Mod+P'] },
  { command: 'project.open', keys: ['Mod+O'] },
  { command: 'project.reload', keys: ['Mod+R'] },
  ...RAIL_SLOTS.map((slot): Binding => ({ command: `rail.${slot}`, keys: [`Mod+${slot}`] })),
  { command: 'tab.next', keys: ['Ctrl+Tab'] },
  { command: 'tab.previous', keys: ['Ctrl+Shift+Tab'] },
  { command: 'tab.close', keys: ['Mod+W'] },
  { command: 'tree.where', keys: ['Mod+Shift+1'] },
  { command: 'tree.who', keys: ['Mod+Shift+2'] },
  { command: 'tree.lore', keys: ['Mod+Shift+3'] },
  { command: 'tree.things', keys: ['Mod+Shift+4'] },
  { command: 'file.save', keys: ['Mod+S'] },
];

/** A modifier as `KeyboardEvent` names its flag. */
type Modifier = 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey';

/** What `match` reads of a key press; a `KeyboardEvent` is one. */
export type KeyPress = Pick<KeyboardEvent, 'key' | 'code' | Modifier>;

/** The modifiers in the order macOS writes them. */
const MODIFIERS: readonly Modifier[] = ['ctrlKey', 'altKey', 'shiftKey', 'metaKey'];
const MAC_SYMBOL: Record<Modifier, string> = { ctrlKey: '⌃', altKey: '⌥', shiftKey: '⇧', metaKey: '⌘' };
const WORD: Record<Modifier, string> = { ctrlKey: 'Ctrl', altKey: 'Alt', shiftKey: 'Shift', metaKey: 'Cmd' };

/** A combination read for `os`: its modifiers and its key. */
interface Combo {
  modifiers: ReadonlySet<Modifier>;
  key: string;
}

/** The modifier each written part stands for; `Mod` depends on the OS. */
function modifierOf(part: string, os: Os): Modifier {
  switch (part) {
    case 'Mod':
      return os === 'macos' ? 'metaKey' : 'ctrlKey';
    case 'Ctrl':
      return 'ctrlKey';
    case 'Alt':
      return 'altKey';
    case 'Shift':
      return 'shiftKey';
    default:
      throw new Error(`the keymap names an unknown modifier \`${part}\``);
  }
}

/** Reads `keys` (`Mod+Shift+P`) for `os`; throws on an unknown modifier or a missing key. */
function parse(keys: string, os: Os): Combo {
  const parts = keys.split('+');
  const key = parts.pop() ?? '';
  if (key === '') {
    throw new Error(`the keymap entry \`${keys}\` names no key`);
  }
  return { modifiers: new Set(parts.map((part) => modifierOf(part, os))), key };
}

/** `keys` as `os` writes them: `Ctrl+Shift+P` on Windows and Linux, `⇧⌘P` on macOS. */
export function label(keys: string, os: Os): string {
  const { modifiers, key } = parse(keys, os);
  const held = MODIFIERS.filter((m) => modifiers.has(m));
  return os === 'macos'
    ? held.map((m) => MAC_SYMBOL[m]).join('') + key
    : [...held.map((m) => WORD[m]), key].join('+');
}

/** The keys of `command`; none for a command without keys. */
export function keysOf(command: CommandId): readonly string[] {
  return keymap.find((binding) => binding.command === command)?.keys ?? [];
}

/** The first keys of `command` as `os` writes them (`Ctrl+O`, `⌘O`), or `''` without keys. */
export function keyLabel(command: CommandId, os: Os): string {
  const first = keysOf(command)[0];
  return first === undefined ? '' : label(first, os);
}

/** A key that is a digit, matched also by its place on the digit row (`Digit3`). */
const DIGIT = /^\d$/;
/** A key a Latin layout writes as text: letters, digits and punctuation. */
const ASCII = /^[!-~]$/;

/**
 * Whether the key of `press` is `key`. A digit is also its place on the digit row, which every
 * layout keeps (Shift+3 gives `§` or `#`, AZERTY gives `"` for 3). A letter counts by the text
 * the press produced, so Dvorak's `r` on the physical P is R and its `,` on the physical W is no
 * W; only a press that produced no Latin text (a Cyrillic `л`) counts by its physical key (`KeyK`).
 */
function sameKey(press: KeyPress, key: string): boolean {
  if (press.key.toLowerCase() === key.toLowerCase()) {
    return true;
  }
  if (DIGIT.test(key)) {
    return press.code === `Digit${key}`;
  }
  return !ASCII.test(press.key) && press.key.length === 1 && press.code === `Key${key.toUpperCase()}`;
}

/** Whether `press` is `combo`: exactly its modifiers, and its key. */
function pressed(press: KeyPress, combo: Combo): boolean {
  return MODIFIERS.every((m) => press[m] === combo.modifiers.has(m)) && sameKey(press, combo.key);
}

/** The command whose keys `press` is on `os`, or `null`. */
export function match(press: KeyPress, os: Os): CommandId | null {
  const found = keymap.find((binding) => binding.keys.some((keys) => pressed(press, parse(keys, os))));
  return found?.command ?? null;
}

/** `os` when it names a platform the keymap knows, else `windows`. */
export function asOs(os: string): Os {
  return os === 'macos' || os === 'linux' ? os : 'windows';
}
