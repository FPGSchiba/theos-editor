// The rail's entries in order, and the entry that is active (P11 §5.3, §5.6).

import type { RailEntry } from './ipc/bindings';

/** The entries sorted by `order`. */
export function railOrder(entries: readonly RailEntry[]): RailEntry[] {
  return [...entries].sort((a, b) => a.order - b.order);
}

/**
 * The entry whose id is `active`; when the rail has no such entry, its first entry; `null` for an
 * empty rail.
 */
export function activeEntry(entries: readonly RailEntry[], active: string): RailEntry | null {
  return entries.find((entry) => entry.id === active) ?? railOrder(entries)[0] ?? null;
}
