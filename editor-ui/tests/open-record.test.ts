// openRecord (P11 §5.8): the tree tab of a record comes from `codex_tabs`, not from a table in
// the UI; a tab's rows load once; when calls overlap, the last one wins.

import { beforeEach, expect, test, vi } from 'vitest';

import type { RecordPage, RecordRef, TreeNode, TreeTabInfo } from '../src/lib/ipc/bindings';

/** A synthetic record reference. */
function ref(id: string, kind: string): RecordRef {
  return { id, kind, path: `${id}.md`, title: `Title of ${id}`, status: null, never: false, locked: [] };
}

/** A synthetic page of the record `id` of `kind`. */
function page(id: string, kind: string): RecordPage {
  return {
    header: { record: ref(id, kind), parent: null, links_in: 0, children: 0 },
    body: [],
    lore: [],
    pictures: [],
    front_matter: [],
    links_in: [],
    used_by: [],
    findings: [],
  };
}

/**
 * A synthetic table in which the kind `widget` sits in Things and `lamp` in Where: no UI table
 * could know that, so a tab switch to it comes from the command's data.
 */
const TABLE: TreeTabInfo[] = [
  { tab: 'where', kinds: ['lamp'], folder: 'world/', id_first: [], root: 'lamp' },
  { tab: 'who', kinds: [], folder: 'people/', id_first: [], root: null },
  { tab: 'lore', kinds: [], folder: 'lore/', id_first: [], root: null },
  { tab: 'things', kinds: ['widget'], folder: 'things/', id_first: ['widget'], root: null },
];

const ipc = vi.hoisted(() => ({
  trees: 0,
  /** Pending record_view answers by id. */
  waiting: new Map<string, () => void>(),
  hold: false,
}));

vi.mock('../src/lib/ipc/bindings', () => ({
  commands: {
    codexTabs: () => Promise.resolve(TABLE),
    codexTree: (tab: string) => {
      ipc.trees += 1;
      const rows: TreeNode[] =
        tab === 'things'
          ? [{ type: 'group', label: 'Things', children: [] }]
          : [{ type: 'record', record: ref('lamp-1', 'lamp'), tag: null, children: [] }];
      return Promise.resolve({ status: 'ok', data: rows });
    },
    recordView: (id: string) => {
      const kind = id.startsWith('widget') ? 'widget' : 'lamp';
      const answer = { status: 'ok', data: page(id, kind) };
      if (!ipc.hold) {
        return Promise.resolve(answer);
      }
      return new Promise((resolve) => ipc.waiting.set(id, () => resolve(answer)));
    },
  },
}));

const { codexTabs, openRecord } = await import('../src/parts/codex/tabs.svelte');
const { tree } = await import('../src/parts/codex/tree.svelte');
const { isIdFirst, treeTabOf } = await import('../src/parts/codex/kinds.svelte');

beforeEach(() => {
  codexTabs.records = [];
  codexTabs.active = -1;
  codexTabs.tree = 'where';
  tree.nodes = {};
  ipc.trees = 0;
  ipc.hold = false;
  ipc.waiting.clear();
});

test('openRecord switches the tree tab from the command data', async () => {
  await openRecord('widget-1');
  expect(codexTabs.tree).toBe('things');
  expect(codexTabs.records.map((tab) => tab.id)).toEqual(['widget-1']);
  expect(treeTabOf('widget')).toBe('things');
  expect(treeTabOf('lamp')).toBe('where');
  expect(treeTabOf('floor')).toBeNull();
  expect(isIdFirst('widget')).toBe(true);
  expect(isIdFirst('lamp')).toBe(false);
});

test('a loaded tab is not loaded again', async () => {
  await openRecord('lamp-1');
  await openRecord('lamp-2');
  expect(codexTabs.tree).toBe('where');
  expect(ipc.trees).toBe(1);
});

test('when calls overlap, the last one wins', async () => {
  ipc.hold = true;
  const first = openRecord('lamp-1');
  const second = openRecord('widget-1');
  ipc.waiting.get('widget-1')?.();
  await second;
  ipc.waiting.get('lamp-1')?.();
  await first;
  expect(codexTabs.records.map((tab) => tab.id)).toEqual(['widget-1']);
  expect(codexTabs.tree).toBe('things');
});

test('a record still loading when another project opens adds no tab', async () => {
  const { closeAllRecords } = await import('../src/parts/codex/freshness');
  ipc.hold = true;
  const opening = openRecord('lamp-1');
  closeAllRecords();
  ipc.waiting.get('lamp-1')?.();
  await opening;
  expect(codexTabs.records).toEqual([]);
});
