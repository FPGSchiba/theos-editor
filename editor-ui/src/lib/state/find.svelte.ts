// The toolbar's find field (P11 §5.10): the query, and the hits of the last query. A query runs
// 120 ms after the last keystroke; when answers arrive out of order, only the last query's is
// shown. The syntax (`#id`, `:kind`, `@people`) is the backend's; the field sends the text as is.

import { commands, type Hit } from '../ipc/bindings';
import { call, failureText } from '../ipc/client';

/** The wait after the last keystroke before a query is sent, in milliseconds. */
export const DEBOUNCE_MS = 120;

/** The most hits asked for. */
export const HIT_LIMIT = 20;

/** What the find field shows. */
export interface FindState {
  /** The text of the field. */
  query: string;
  /** Whether the results list is open. */
  open: boolean;
  /** The hits of the last answered query. */
  hits: Hit[];
  /** Whether more records match than `hits` holds. */
  truncated: boolean;
  /** The line of the backend for the user, or the failure text. */
  note: string | null;
  /** The index of the highlighted hit; -1 without one. */
  selected: number;
}

/** The find field's state. */
export const find = $state<FindState>({
  query: '',
  open: false,
  hits: [],
  truncated: false,
  note: null,
  selected: -1,
});

let timer: ReturnType<typeof setTimeout> | undefined;
/** The number of the last query; an answer to an older one is dropped. */
let latest = 0;

/** Empties the results. */
function clearResults(): void {
  find.hits = [];
  find.truncated = false;
  find.note = null;
  find.selected = -1;
}

/** Sends `query` and shows its answer, unless a newer query was made meanwhile. */
async function run(query: string, ticket: number): Promise<void> {
  let hits: Hit[] = [];
  let truncated = false;
  let note: string | null;
  try {
    const result = await call(commands.codexSearch(query, HIT_LIMIT));
    ({ hits, truncated, note } = result);
  } catch (failure) {
    note = `Search failed: ${failureText(failure)}`;
  }
  if (ticket !== latest) {
    return;
  }
  find.open = true;
  find.hits = hits;
  find.truncated = truncated;
  find.note = note;
  find.selected = -1;
}

/** Sets the field's text and, after the debounce, searches for it; an empty text clears. */
export function setQuery(query: string): void {
  find.query = query;
  find.open = false;
  clearTimeout(timer);
  const ticket = ++latest;
  if (query.trim() === '') {
    clearResults();
    return;
  }
  // The hits of an older text no longer match the field: none stay to be opened.
  clearResults();
  timer = setTimeout(() => void run(query, ticket), DEBOUNCE_MS);
}

/** Closes the results list; a query still waiting is dropped. */
export function closeFind(): void {
  clearTimeout(timer);
  latest += 1;
  find.open = false;
}

/** Empties the field and its hits, and drops a query still waiting: another project opened. */
export function resetFind(): void {
  closeFind();
  find.query = '';
  clearResults();
}

/** Opens the results list again for the text in the field, as when the field gets the focus. */
export function reopenFind(): void {
  if (find.query.trim() !== '' && !find.open) {
    setQuery(find.query);
  }
}

/** Moves the highlight `step` hits on, wrapping; none is highlighted until the first move. */
export function moveSelection(step: number): void {
  const count = find.hits.length;
  if (count === 0) {
    return;
  }
  // From no highlight, ArrowDown goes to the first hit and ArrowUp to the last.
  const from = find.selected < 0 ? (step > 0 ? -1 : 0) : find.selected;
  find.selected = (from + step + count) % count;
}

/** The text of `hit`'s first match split around the matched slice, or `null` without a match. */
export function markedLine(hit: Hit): { before: string; matched: string; after: string } | null {
  const first = hit.matches[0];
  if (first === undefined) {
    return null;
  }
  const chars = Array.from(first.text);
  const matched = Array.from(first.match);
  let start = Math.max(0, first.column_number - 1);
  if (chars.slice(start, start + matched.length).join('') !== first.match) {
    const at = first.text.indexOf(first.match);
    if (at < 0) {
      return { before: first.text, matched: '', after: '' };
    }
    start = Array.from(first.text.slice(0, at)).length;
  }
  return {
    before: chars.slice(0, start).join(''),
    matched: first.match,
    after: chars.slice(start + matched.length).join(''),
  };
}
