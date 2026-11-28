// The Codex after a sync (P11 §5.9): a removed open record closes its tab and the status bar says
// so; the next sync that closes no tab clears the notice.

import { beforeEach, expect, test } from 'vitest';

import type { SyncView } from '../src/lib/ipc/bindings';
import { notice } from '../src/lib/state/notice.svelte';
import { deletedText, refreshRecords } from '../src/parts/codex/freshness';
import { codexTabs, openRecordTab } from '../src/parts/codex/tabs.svelte';

/** A sync that removed `removed` and changed nothing. */
const synced = (removed: string[]): SyncView => ({ project: {} as SyncView['project'], changed: [], removed });

beforeEach(() => {
  codexTabs.records = [];
  codexTabs.active = -1;
  notice.text = null;
});

test('a removed open record closes its tab and the notice clears on the next sync', async () => {
  openRecordTab('C-032', 'Synthetic clue C-032');
  openRecordTab('hearth', 'Hearth');
  await refreshRecords(synced(['C-032', 'C-031']));
  expect(codexTabs.records.map((tab) => tab.id)).toEqual(['hearth']);
  expect(notice.text).toBe('C-032 was deleted outside the editor');
  await refreshRecords(synced(['T-012']));
  expect(notice.text).toBeNull();
});

test('several deleted records are named together', () => {
  expect(deletedText(['C-031', 'C-032'])).toBe('C-031, C-032 were deleted outside the editor');
});
