import { expect, test } from 'vitest';

import type { RailEntry } from './ipc/bindings';
import { activeEntry, railOrder } from './rail';

const entry = (id: string, order: number): RailEntry => ({
  id,
  label: id,
  glyph: '◇',
  order,
  phase: null,
});

const RAIL = [entry('settings', 100), entry('codex', 10), entry('graph', 30)];

test('the rail is sorted by order', () => {
  expect(railOrder(RAIL).map((e) => e.id)).toEqual(['codex', 'graph', 'settings']);
});

test('the active entry is the one with the active id', () => {
  expect(activeEntry(RAIL, 'graph')?.id).toBe('graph');
});

// AC: CE-32.1
test('an active id the rail does not have falls back to the first entry', () => {
  expect(activeEntry(RAIL, 'atlas')?.id).toBe('codex');
  expect(activeEntry([], 'codex')).toBeNull();
});
