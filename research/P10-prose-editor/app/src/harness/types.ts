export type Kind = 'tower' | 'band' | 'floor' | 'location' | 'people' | 'faction' | 'npc'
  | 'thing' | 'topic' | 'truth' | 'belief' | 'clue' | 'equation' | 'script' | 'dialogue'
  | 'template' | 'concept' | 'concept-version' | 'map' | 'map-version';
export type BeatKey = 'at' | 'with' | 'says' | 'reveals' | 'when';
export const BEAT_KEY_ORDER: readonly BeatKey[] = ['at', 'with', 'says', 'reveals', 'when'];
export type LinkPlace = { in: 'prose' } | { in: 'beat'; key: BeatKey };
export interface LinkInfo { id: string; kind: Kind; title: string }
export interface Finding { rule: string; severity: 'error' | 'warn' | 'info'; message: string;
  at: { line: number } | { beat: number } }
export type Mode = 'write' | 'source';
export interface Anchor { find: string; occurrence?: number /* 1-based, default 1 */;
  offset?: number /* added to the match start, default 0 */ }
export interface TextChange { from: number; to: number; insert: string }
export type ScriptedEdit = { id: string } & (
  | { kind: 'insert'; at: Anchor; text: string }
  | { kind: 'delete'; at: Anchor; length: number; backward?: boolean }
  | { kind: 'beat-param'; beat: number; key: BeatKey; value: string }
  | { kind: 'note-state'; note: string; state: string }
  | { kind: 'link-insert'; at: Anchor; query: string; pick: string }
  | { kind: 'link-delete'; link: Anchor /* points at the link's "[[" */ }
  | { kind: 'select-all-type'; text: string }
  | { kind: 'paste'; at: Anchor; plain: string; html?: string }
  | { kind: 'external'; local: { at: Anchor; text: string }; external: { at: Anchor; text: string } }
  | { kind: 'undo-all'; edits: Array<{ id: string } & ({ kind: 'insert'; at: Anchor; text: string }
      | { kind: 'delete'; at: Anchor; length: number; backward?: boolean })> });
export interface EditorContext {
  resolve(id: string): LinkInfo | undefined;
  allowedKinds(place: LinkPlace): Kind[];
  search(query: string, place: LinkPlace): LinkInfo[];
  findings: Finding[];
  developerWords: string[];
  keymap: Keymap;
  save(): void;
}
export interface Keymap { save: string; undo: string; redo: string; selectAll: string } // 'Mod-s' etc.
export interface ProseAdapter {
  mount(target: HTMLElement, text: string, ctx: EditorContext): void;
  text(): string;
  apply(edit: ScriptedEdit): void;          // through the editor's own commands
  setMode(mode: Mode): void;
  focus(): void;
  caretTo(offset: number): void;
  caret(): number;
  applyExternal(changes: TextChange[]): void; // minimal edit, keeps caret and local undo
  undo(): void;
  canUndo(): boolean;
  markSaved(): void;                         // clears the field undo history (CE-34.3)
  updateContext(ctx: EditorContext): void;   // index or findings changed; no re-mount
  lastSaveLevel?(): 'span' | 'block';        // best-effort variants only
  destroy(): void;
}
export interface VariantInfo { name: string /* e.g. 'codemirror', 'tiptap:stock' */;
  component: string; version: string; wysiwyg: boolean; column?: 'stock' | 'best-effort';
  baseline?: boolean; packages: string[] /* direct npm dependencies of this variant */ }
export interface Comparison { identical: boolean; firstDiff?: { line: number; column: number };
  extraBytes: number; withinBlock?: boolean; hunk?: string }
