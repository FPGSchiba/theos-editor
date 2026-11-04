import { closeHistory } from 'prosemirror-history';
import { TextSelection } from 'prosemirror-state';
import type { EditorView } from 'prosemirror-view';

/**
 * The state of a note callout lives in the last segment of its title line:
 * `> [!note] n3 · [[f04-terrain-v2]] · open`, `· dropped`, or `· resolved by [[version]]`.
 * In the tree the title is the text of the callout's first child (the part behind `[!note] `).
 */

/** The separator between the segments of a title line. */
export const SEPARATOR = ' · ';
/** A segment that is a state; any other last segment (a version, nothing) means the note has none yet. */
const STATE = /^(?:open|dropped|resolved by \[\[[^\]]*\]\])$/;
const RESOLVED = /^resolved by \[\[([^\]|]*)(?:\|[^\]]*)?\]\]$/;

export interface NoteParts {
  note: string;
  state: 'open' | 'dropped' | 'resolved';
  /** The segment between the id and the state that is a link, as written. */
  version?: string;
  resolvedBy?: string;
}

/** What a note's title says: id, state (`open` when it has none), version and resolving version. */
export function noteParts(title: string): NoteParts {
  const segments = title.split(SEPARATOR).map((segment) => segment.trim());
  const last = segments.length > 1 ? segments[segments.length - 1] : undefined;
  const isState = last !== undefined && STATE.test(last);
  const middle = segments.slice(1, isState ? -1 : undefined);
  const version = middle.find((segment) => segment.startsWith('[['));
  const resolvedBy = isState ? RESOLVED.exec(last)?.[1]?.trim() : undefined;
  return {
    note: segments[0] ?? '',
    state: !isState || last === 'open' ? 'open' : last === 'dropped' ? 'dropped' : 'resolved',
    ...(version === undefined ? {} : { version }),
    ...(resolvedBy === undefined || resolvedBy === '' ? {} : { resolvedBy }),
  };
}

const escapeRegExp = (text: string): string => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/**
 * `note-state`: one transaction that replaces the last segment of the note's title when it is a
 * state, and appends the state as a new segment when the title has none, as the oracle does. It
 * is its own undo step. The caret ends behind the state, in the title, which then shows raw.
 */
export function setNoteState(view: EditorView, note: string, state: string): void {
  const title = new RegExp(`^${escapeRegExp(note)}(?:${escapeRegExp(SEPARATOR)}.*)?$`);
  let found: { start: number; text: string } | undefined;
  view.state.doc.descendants((node, pos) => {
    if (found !== undefined) return false;
    if (node.type.name === 'callout' && node.attrs.type === 'note' && title.test(node.firstChild?.textContent ?? '')) {
      found = { start: pos + 2, text: node.firstChild!.textContent };
      return false;
    }
    return !node.isTextblock;
  });
  if (found === undefined) throw new Error(`note-state: there is no note ${JSON.stringify(note)}`);
  const segments = found.text.split(SEPARATOR);
  const last = segments.length > 1 ? segments[segments.length - 1]! : undefined;
  const replaces = last !== undefined && STATE.test(last);
  const to = found.start + found.text.length;
  const from = replaces ? to - last.length : to;
  const insert = replaces ? state : `${SEPARATOR}${state}`;
  const tr = closeHistory(view.state.tr.insertText(insert, from, to));
  view.dispatch(tr.setSelection(TextSelection.create(tr.doc, from + insert.length)).scrollIntoView());
  // Whatever is typed next starts a step of its own.
  view.dispatch(closeHistory(view.state.tr.setMeta('addToHistory', false)));
}
