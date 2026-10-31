import type { EditorView } from '@codemirror/view';

/**
 * The state of a note callout lives in the last segment of its title line:
 * `> [!note] n3 · [[f04-terrain-v2]] · open`, `· dropped`, or `· resolved by [[version]]`.
 */

/** The separator between the segments of a title line. */
export const SEPARATOR = ' · ';
/** A segment that is a state; any other last segment (a version, nothing) means the note has none yet. */
const STATE = /^(?:open|dropped|resolved by \[\[[^\]]*\]\])$/;
const RESOLVED = /^resolved by \[\[([^\]|]*)(?:\|[^\]]*)?\]\]$/;

export interface NoteState {
  state: 'open' | 'dropped' | 'resolved';
  /** The version that resolves the note. */
  resolvedBy?: string;
}

/** The state a title segment stands for, or `undefined` when it is not one. */
export function readState(segment: string | undefined): NoteState | undefined {
  if (segment === undefined || !STATE.test(segment)) return undefined;
  if (segment === 'open' || segment === 'dropped') return { state: segment };
  return { state: 'resolved', resolvedBy: RESOLVED.exec(segment)?.[1]?.trim() ?? '' };
}

const escapeRegExp = (text: string): string => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/**
 * `note-state`: one transaction that replaces the last segment of the note's title line when it is a
 * state, and appends the state as a new segment when the title has none. The caret ends behind it,
 * on the title line, which then shows raw as it does for anyone typing there.
 */
export function setNoteState(view: EditorView, note: string, state: string): void {
  const { doc } = view.state;
  const title = new RegExp(`^> \\[!note\\] ${escapeRegExp(note)}(?:${escapeRegExp(SEPARATOR)}.*)?$`);
  for (let number = 1; number <= doc.lines; number++) {
    const line = doc.line(number);
    if (!title.test(line.text)) continue;
    const segments = line.text.split(SEPARATOR);
    const last = segments.length > 1 ? segments[segments.length - 1] : undefined;
    const replaces = readState(last) !== undefined;
    const from = replaces ? line.to - (last?.length ?? 0) : line.to;
    const insert = replaces ? state : `${SEPARATOR}${state}`;
    view.dispatch({
      changes: { from, to: line.to, insert },
      selection: { anchor: from + insert.length },
      userEvent: 'input.type',
      scrollIntoView: true,
    });
    return;
  }
  throw new Error(`note-state: there is no note ${JSON.stringify(note)}`);
}
