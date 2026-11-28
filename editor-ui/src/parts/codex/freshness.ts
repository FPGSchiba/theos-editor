// The Codex after a sync (P11 §5.9): a record file deleted outside the editor closes its tab and
// the status bar says so until the next sync that closes none; a moved file's tab follows it; an
// open file that changed loads its page again. The tree reloads by itself when the project view
// changes (`keepFor` in tree.svelte.ts).
//
// Sync names files by path, and tabs are keyed by path (`recordKey`): every match below compares
// keys, never ids, since two files may carry one id. A tab is never given a key another tab holds.

import { commands, type FileMove, type SyncView } from '../../lib/ipc/bindings';
import { call, failureText } from '../../lib/ipc/client';
import { hush, say } from '../../lib/state/notice.svelte';
import { recordKey } from '../../lib/record-key';
import { forgetPage, pages, storePage } from './pages.svelte';
import { closeRecordTab, codexTabs, dropOpening, type RecordTab } from './tabs.svelte';
import { tree } from './tree.svelte';

/** The status bar's words for records deleted outside the editor. */
export function deletedText(ids: readonly string[]): string {
  return `${ids.join(', ')} ${ids.length === 1 ? 'was' : 'were'} deleted outside the editor`;
}

/** The open tab of the file `key`, if any. */
function tabOf(key: string): RecordTab | undefined {
  return codexTabs.records.find((tab) => tab.key === key);
}

/** Closes the tab of the file `key` and returns its id, or `null` when none is open. */
function closeFile(key: string): string | null {
  const tab = tabOf(key);
  if (tab === undefined) {
    return null;
  }
  closeRecordTab(key);
  return tab.id;
}

/**
 * Moves the tab of the moved file to its new path. When another tab already holds the new path,
 * the moved tab closes instead and its id is returned, to be named as deleted.
 */
function follow(move: FileMove): string | null {
  const tab = tabOf(move.from);
  if (tab === undefined) {
    return null;
  }
  if (tabOf(move.to) !== undefined) {
    return closeFile(move.from);
  }
  forgetPage(move.from);
  tab.key = move.to;
  tab.path = move.to;
  return null;
}

/**
 * Closes the tabs of removed files, moves the tabs of moved files, and loads the pages of changed
 * open files again. The status bar names the records whose tabs closed; a sync that closes none
 * clears it.
 */
export async function refreshRecords(sync: SyncView): Promise<void> {
  const closed = [
    ...sync.removed.map((file) => closeFile(recordKey(file))),
    ...sync.moved.map(follow),
  ];
  const changed = sync.changed.map(recordKey).filter((key) => tabOf(key) !== undefined);
  closed.push(...(await Promise.all(changed.map(reloadPage))));
  const ids = [...new Set(closed.filter((id): id is string => id !== null))];
  if (ids.length > 0) {
    say(deletedText(ids));
  } else {
    hush();
  }
}

/**
 * Loads the page of the open file `key` again; its tab takes the new title and kind. The page
 * comes from that file only: an answer for another file closes the tab, whose id is returned to be
 * named as deleted, and never moves it onto that file's key.
 */
async function reloadPage(key: string): Promise<string | null> {
  const opened = tabOf(key);
  if (opened === undefined) {
    return null;
  }
  const { id } = opened;
  try {
    const page = await call(commands.recordView(id, opened.path));
    const tab = tabOf(key);
    if (tab === undefined) {
      return null;
    }
    const { record } = page.header;
    if (recordKey(record) !== key) {
      return closeFile(key);
    }
    storePage(key, page);
    tab.title = record.title ?? id;
    tab.kind = record.kind;
  } catch (failure) {
    pages.error = `${id} could not be reloaded: ${failureText(failure)}`;
  }
  return null;
}

/**
 * Forgets the open records and the folded rows, and drops the records still loading: another
 * project opened.
 */
export function closeAllRecords(): void {
  dropOpening();
  codexTabs.records = [];
  codexTabs.active = -1;
  pages.byKey = {};
  pages.error = null;
  tree.folded = [];
}
