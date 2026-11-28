// The Codex after a sync (P11 §5.9): a record deleted outside the editor closes its tab and the
// status bar says so until the next sync that closes none; an open record that changed loads its
// page again. The tree reloads by
// itself when the project view changes (`keepFor` in tree.svelte.ts).

import { commands, type SyncView } from '../../lib/ipc/bindings';
import { call, failureText } from '../../lib/ipc/client';
import { hush, say } from '../../lib/state/notice.svelte';
import { pages, storePage } from './pages.svelte';
import { closeRecordTab, codexTabs, dropOpening } from './tabs.svelte';
import { tree } from './tree.svelte';

/** The status bar's words for records deleted outside the editor. */
export function deletedText(ids: readonly string[]): string {
  return `${ids.join(', ')} ${ids.length === 1 ? 'was' : 'were'} deleted outside the editor`;
}

/** The ids of the open record tabs. */
function openIds(): Set<string> {
  return new Set(codexTabs.records.map((tab) => tab.id));
}

/**
 * Closes the tabs of removed records, says so in the status bar (a sync that closes none clears
 * that), and loads the pages of changed open records again.
 */
export async function refreshRecords(sync: SyncView): Promise<void> {
  const open = openIds();
  const gone = sync.removed.filter((id) => open.has(id));
  for (const id of gone) {
    closeRecordTab(id);
  }
  if (gone.length > 0) {
    say(deletedText(gone));
  } else {
    hush();
  }
  const stale = sync.changed.filter((id) => open.has(id) && !gone.includes(id));
  await Promise.all(stale.map(reloadPage));
}

/** Loads the page of the open record `id` again; its tab takes the new title and kind. */
async function reloadPage(id: string): Promise<void> {
  try {
    const page = await call(commands.recordView(id));
    const tab = codexTabs.records.find((entry) => entry.id === id);
    if (tab === undefined) {
      return;
    }
    storePage(id, page);
    tab.title = page.header.record.title ?? id;
    tab.kind = page.header.record.kind;
  } catch (failure) {
    pages.error = `${id} could not be reloaded: ${failureText(failure)}`;
  }
}

/**
 * Forgets the open records and the folded rows, and drops the records still loading: another
 * project opened.
 */
export function closeAllRecords(): void {
  dropOpening();
  codexTabs.records = [];
  codexTabs.active = -1;
  pages.byId = {};
  pages.error = null;
  tree.folded = [];
}
