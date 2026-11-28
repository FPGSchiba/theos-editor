// The pages of the open records (P11 §5.8): what `record_view` answered for each open tab, keyed
// by the tab's file (two files may carry one id), and why the last record could not be opened.
// Nothing here is stored on a record; a page is the index's view at the time it was loaded.

import type { RecordPage } from '../../lib/ipc/bindings';

/** The loaded pages by the key of their file (`recordKey`), and the last failure to open one. */
export interface Pages {
  byKey: Record<string, RecordPage>;
  error: string | null;
}

/** The pages of the open records. */
export const pages = $state<Pages>({ byKey: {}, error: null });

/** Keeps `page` as the page of the file `key`. */
export function storePage(key: string, page: RecordPage): void {
  pages.byKey[key] = page;
  pages.error = null;
}

/** Drops the page of the file `key`, whose tab closed. */
export function forgetPage(key: string): void {
  delete pages.byKey[key];
}

/** The page of the file `key`, or `null` when none is loaded. */
export function pageOf(key: string | null): RecordPage | null {
  return key === null ? null : (pages.byKey[key] ?? null);
}
