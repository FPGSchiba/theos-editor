// The find field's state (P11 §5.10): a 120 ms debounce, limit 20, and when answers arrive out
// of order only the last query's hits are shown.

import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import type { Hit } from '../src/lib/ipc/bindings';

/** A synthetic hit. */
function hit(id: string): Hit {
  return {
    id,
    kind: 'clue',
    path: `${id}.md`,
    title: `Title of ${id}`,
    status: null,
    never: false,
    locked: [],
    score: null,
    matches: [],
  };
}

const ipc = vi.hoisted(() => ({
  calls: [] as { query: string; limit: number }[],
  waiting: new Map<string, (hits: string[]) => void>(),
}));

vi.mock('../src/lib/ipc/bindings', () => ({
  commands: {
    codexSearch: (query: string, limit: number) => {
      ipc.calls.push({ query, limit });
      return new Promise((resolve) =>
        ipc.waiting.set(query, (ids) =>
          resolve({
            status: 'ok',
            data: { hits: ids.map((id) => ({ id })), truncated: false, note: null },
          }),
        ),
      );
    },
  },
}));

const { find, setQuery, closeFind, markedLine } = await import('../src/lib/state/find.svelte');

beforeEach(() => {
  vi.useFakeTimers();
  ipc.calls = [];
  ipc.waiting.clear();
  setQuery('');
});

afterEach(() => {
  vi.useRealTimers();
});

// AC: CE-33.3
test('a query is sent 120 ms after the last keystroke, with limit 20', async () => {
  setQuery(':c');
  vi.advanceTimersByTime(119);
  expect(ipc.calls).toEqual([]);
  setQuery(':cl');
  vi.advanceTimersByTime(119);
  expect(ipc.calls).toEqual([]);
  vi.advanceTimersByTime(1);
  expect(ipc.calls).toEqual([{ query: ':cl', limit: 20 }]);
});

// AC: CE-33.3
test('when answers arrive out of order, only the last query is shown', async () => {
  setQuery('a');
  vi.advanceTimersByTime(120);
  setQuery('ab');
  vi.advanceTimersByTime(120);
  expect(ipc.calls.map((c) => c.query)).toEqual(['a', 'ab']);
  ipc.waiting.get('ab')?.(['second']);
  await vi.advanceTimersByTimeAsync(0);
  ipc.waiting.get('a')?.(['first']);
  await vi.advanceTimersByTimeAsync(0);
  expect(find.hits.map((h) => h.id)).toEqual(['second']);
});

// AC: CE-33.3
test('closing drops a waiting query and an answer in flight', async () => {
  setQuery('x');
  vi.advanceTimersByTime(120);
  closeFind();
  ipc.waiting.get('x')?.(['late']);
  await vi.advanceTimersByTimeAsync(0);
  expect(find.hits).toEqual([]);
  expect(find.open).toBe(false);
  setQuery('y');
  closeFind();
  vi.advanceTimersByTime(500);
  expect(ipc.calls.map((c) => c.query)).toEqual(['x']);
});

/** A hit whose first match is `match` at `column_number` of `text`. */
function matching(text: string, match: string, column_number: number): Hit {
  return {
    ...hit('a'),
    matches: [{ column: 'body', line: 1, column_number, match, text }],
  };
}

// AC: CE-33.3
test('the column picks the occurrence that is marked', () => {
  expect(markedLine(matching('ra ra', 'ra', 4))).toEqual({ before: 'ra ', matched: 'ra', after: '' });
  expect(markedLine(matching('ra ra', 'ra', 1))).toEqual({ before: '', matched: 'ra', after: ' ra' });
});

// AC: CE-33.3
test('a surrogate pair before the match counts as one column', () => {
  expect(markedLine(matching('𝔸 ra ra', 'ra', 6))).toEqual({
    before: '𝔸 ra ',
    matched: 'ra',
    after: '',
  });
});

// AC: CE-33.3
test('a wrong column falls back to the first occurrence of the match', () => {
  expect(markedLine(matching('xx ra ra', 'ra', 2))).toEqual({
    before: 'xx ',
    matched: 'ra',
    after: ' ra',
  });
  expect(markedLine(matching('xx ra', 'ra', 0))).toEqual({ before: 'xx ', matched: 'ra', after: '' });
  expect(markedLine(matching('xx', 'ra', 1))).toEqual({ before: 'xx', matched: '', after: '' });
});

// AC: CE-33.3
test('the list opens on the first answer, not on the keystroke', async () => {
  setQuery('q');
  expect(find.open).toBe(false);
  vi.advanceTimersByTime(120);
  expect(find.open).toBe(false);
  ipc.waiting.get('q')?.(['one']);
  await vi.advanceTimersByTimeAsync(0);
  expect(find.open).toBe(true);
  expect(find.hits.map((h) => h.id)).toEqual(['one']);
});

// AC: CE-33.3
test('hits of an older text are gone, so none can be opened before the new answer', async () => {
  setQuery('q');
  vi.advanceTimersByTime(120);
  ipc.waiting.get('q')?.(['one']);
  await vi.advanceTimersByTimeAsync(0);
  find.selected = 0;
  setQuery('qu');
  expect(find.hits).toEqual([]);
  expect(find.selected).toBe(-1);
  expect(find.open).toBe(false);
});
