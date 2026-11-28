// The Codex after a sync (P11 §5.9): sync names record files by path, and tabs are keyed by path,
// so two files with one id are never confused. A removed open file closes its tab and the status
// bar says so; the next sync that closes no tab clears the notice. A moved file's tab follows it;
// a changed open file loads its page again, from that file only.

import { beforeEach, expect, test, vi } from 'vitest';

import type { RecordFile, RecordPage, SyncView } from '../src/lib/ipc/bindings';

/** A synthetic page of the record `id` in the file `path`. */
function page(id: string, path: string): RecordPage {
  return {
    header: {
      record: { id, kind: null, path, title: `Title of ${path}`, status: null, never: false, locked: [] },
      parent: null,
      links_in: 0,
      children: 0,
    },
    body: [],
    lore: [],
    pictures: [],
    front_matter: [],
    links_in: [],
    used_by: [],
    findings: [],
  };
}

const ipc = vi.hoisted(() => ({
  /** The record_view calls: id and path. */
  views: [] as Array<{ id: string; path: string | null }>,
  /** The file a record_view answers instead of the one asked for, by asked path. */
  answers: new Map<string, string>(),
}));

vi.mock('../src/lib/ipc/bindings', () => ({
  commands: {
    recordView: (id: string, path: string | null) => {
      ipc.views.push({ id, path });
      const asked = path ?? `${id}.md`;
      return Promise.resolve({ status: 'ok', data: page(id, ipc.answers.get(asked) ?? asked) });
    },
  },
}));

const { notice } = await import('../src/lib/state/notice.svelte');
const { deletedText, refreshRecords } = await import('../src/parts/codex/freshness');
const { pages, storePage } = await import('../src/parts/codex/pages.svelte');
const { codexTabs, openRecordTab } = await import('../src/parts/codex/tabs.svelte');

/** The file `path` carrying `id`. */
const file = (id: string, path: string): RecordFile => ({ id, path });

/** A sync with these lists; the ones left out are empty. */
function synced(lists: Partial<Omit<SyncView, 'project'>>): SyncView {
  return {
    project: {} as SyncView['project'],
    changed: [],
    removed: [],
    moved: [],
    ...lists,
  };
}

/** Opens the tab of `id` in the file `path`, with its page loaded. */
function open(id: string, path: string): void {
  openRecordTab(id, `Title of ${path}`, null, path);
  storePage(path, page(id, path));
}

const keys = (): string[] => codexTabs.records.map((tab) => tab.key);

beforeEach(() => {
  codexTabs.records = [];
  codexTabs.active = -1;
  pages.byKey = {};
  pages.error = null;
  notice.text = null;
  ipc.views = [];
  ipc.answers.clear();
});

test('a removed open file closes its tab and the notice clears on the next sync', async () => {
  open('C-032', 'lore/clues/C-032.md');
  open('hearth', 'world/hearth.md');
  await refreshRecords(
    synced({ removed: [file('C-032', 'lore/clues/C-032.md'), file('C-031', 'lore/clues/C-031.md')] }),
  );
  expect(keys()).toEqual(['world/hearth.md']);
  expect(notice.text).toBe('C-032 was deleted outside the editor');
  expect(pages.byKey['lore/clues/C-032.md']).toBeUndefined();
  await refreshRecords(synced({ removed: [file('T-012', 'lore/truths/T-012.md')] }));
  expect(notice.text).toBeNull();
});

test('several deleted records are named together', () => {
  expect(deletedText(['C-031', 'C-032'])).toBe('C-031, C-032 were deleted outside the editor');
});

test('one of two open files with one id deleted closes that tab only', async () => {
  open('twin', 'world/twin.md');
  open('twin', 'world/copy/twin.md');
  open('hearth', 'world/hearth.md');
  await refreshRecords(
    synced({ removed: [file('twin', 'world/twin.md')], changed: [file('twin', 'world/copy/twin.md')] }),
  );
  expect(keys()).toEqual(['world/copy/twin.md', 'world/hearth.md']);
  expect(new Set(keys()).size).toBe(keys().length);
  expect(notice.text).toBe('twin was deleted outside the editor');
  expect(ipc.views).toEqual([{ id: 'twin', path: 'world/copy/twin.md' }]);
  expect(pages.byKey['world/copy/twin.md']?.header.record.path).toBe('world/copy/twin.md');
  expect(pages.byKey['world/twin.md']).toBeUndefined();
});

test('a moved file keeps its tab, which takes the new path and loads from it', async () => {
  open('C-032', 'lore/clues/C-032.md');
  await refreshRecords(
    synced({
      moved: [{ id: 'C-032', from: 'lore/clues/C-032.md', to: 'lore/moved/C-032.md' }],
      changed: [file('C-032', 'lore/moved/C-032.md')],
    }),
  );
  expect(codexTabs.records).toEqual([
    {
      key: 'lore/moved/C-032.md',
      id: 'C-032',
      path: 'lore/moved/C-032.md',
      title: 'Title of lore/moved/C-032.md',
      kind: null,
    },
  ]);
  expect(codexTabs.active).toBe(0);
  expect(ipc.views).toEqual([{ id: 'C-032', path: 'lore/moved/C-032.md' }]);
  expect(Object.keys(pages.byKey)).toEqual(['lore/moved/C-032.md']);
  expect(notice.text).toBeNull();
});

test('a move onto a file another tab holds closes the moved tab and says so', async () => {
  open('twin', 'world/twin.md');
  open('twin', 'world/copy/twin.md');
  await refreshRecords(
    synced({ moved: [{ id: 'twin', from: 'world/twin.md', to: 'world/copy/twin.md' }] }),
  );
  expect(keys()).toEqual(['world/copy/twin.md']);
  expect(notice.text).toBe('twin was deleted outside the editor');
});

test('a reload that answers another file closes the tab instead of taking that key', async () => {
  open('twin', 'world/twin.md');
  open('twin', 'world/copy/twin.md');
  ipc.answers.set('world/twin.md', 'world/copy/twin.md');
  await refreshRecords(synced({ changed: [file('twin', 'world/twin.md')] }));
  expect(keys()).toEqual(['world/copy/twin.md']);
  expect(pages.byKey['world/copy/twin.md']?.header.record.title).toBe('Title of world/copy/twin.md');
  expect(notice.text).toBe('twin was deleted outside the editor');
});

test('a changed file reloads only its own tab, not every tab with its id', async () => {
  open('twin', 'world/twin.md');
  open('twin', 'world/copy/twin.md');
  await refreshRecords(synced({ changed: [file('twin', 'world/twin.md')] }));
  expect(ipc.views).toEqual([{ id: 'twin', path: 'world/twin.md' }]);
  expect(keys()).toEqual(['world/twin.md', 'world/copy/twin.md']);
  expect(notice.text).toBeNull();
});
