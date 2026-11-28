// The pages of the open records (P11 §5.8): what `record_view` answered for each open tab, and
// why the last record could not be opened. Nothing here is stored on a record; a page is the
// index's view at the time it was loaded.

import type { RecordPage } from '../../lib/ipc/bindings';

/** The loaded pages by record id, and the last failure to open one. */
export interface Pages {
  byId: Record<string, RecordPage>;
  error: string | null;
}

/** The pages of the open records. */
export const pages = $state<Pages>({ byId: {}, error: null });

/** Keeps `page` as the page of the record `id`. */
export function storePage(id: string, page: RecordPage): void {
  pages.byId[id] = page;
  pages.error = null;
}

/** Drops the page of the record `id`, whose tab closed. */
export function forgetPage(id: string): void {
  delete pages.byId[id];
}

/** The page of the record `id`, or `null` when none is loaded. */
export function pageOf(id: string | null): RecordPage | null {
  return id === null ? null : (pages.byId[id] ?? null);
}
