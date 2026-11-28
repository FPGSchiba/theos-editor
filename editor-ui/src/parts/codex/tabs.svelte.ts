// The Codex's tabs (P11 §5.8, §5.11): the active tree tab, and the open record tabs of the record
// window. The commands Tree tab, Next tab, Previous tab and Close tab work on this state, and the
// Codex preset's windows draw it. `openRecord` opens a record from a tree row, a link chip, a
// "Used by" row or a search hit.

import { call, failureText } from '../../lib/ipc/client';
import { commands, type TreeTab } from '../../lib/ipc/bindings';
import { recordKey, type RecordTarget } from '../../lib/record-key';
import type { RecordTabTitle, TabSelection } from '../layout';
import { loadKinds, treeTabOf } from './kinds.svelte';
import { forgetPage, pages, storePage } from './pages.svelte';
import { expandTo, loadTree, tree } from './tree.svelte';

/** The tree tabs in order, with their titles. Their kinds come from `codex_tabs` (kinds.svelte.ts). */
export const TREE_TABS: ReadonlyArray<{ tab: TreeTab; title: string }> = [
  { tab: 'where', title: 'Where' },
  { tab: 'who', title: 'Who' },
  { tab: 'lore', title: 'Lore' },
  { tab: 'things', title: 'Things' },
];

/** An open record tab: one file, so two files with one id are two tabs. */
export interface RecordTab {
  /** The key of the record's file (`recordKey`): its path, else its id. */
  key: string;
  /** The record's id. */
  id: string;
  /** The record's file, project-relative, when known. */
  path: string | null;
  /** The record's title. */
  title: string;
  /** The record's kind, for the tab's glyph. */
  kind: string | null;
}

/** The Codex's tabs. */
export interface CodexTabs {
  /** The active tree tab. */
  tree: TreeTab;
  /** The open record tabs, in order. */
  records: RecordTab[];
  /** The index of the active record tab; -1 without one. */
  active: number;
}

/** The Codex's tabs. */
export const codexTabs = $state<CodexTabs>({ tree: 'where', records: [], active: -1 });

/** Makes `tab` the active tree tab. */
export function setTreeTab(tab: TreeTab): void {
  codexTabs.tree = tab;
}

/**
 * The key (`recordKey`) of the file in the active tab, or `null`: its page is `pageOf` that key,
 * and the tree marks that row, not every row with the id.
 */
export function activeKey(): string | null {
  return codexTabs.records[codexTabs.active]?.key ?? null;
}

/** Opens a tab for the record `id` in the file `path`, or makes that file's open tab active. */
export function openRecordTab(
  id: string,
  title: string,
  kind: string | null = null,
  path: string | null = null,
): void {
  const key = recordKey({ id, path });
  const open = codexTabs.records.findIndex((tab) => tab.key === key);
  if (open >= 0) {
    codexTabs.active = open;
    return;
  }
  codexTabs.records.push({ key, id, path, title, kind });
  codexTabs.active = codexTabs.records.length - 1;
}

/** The number of the last `openRecord` call; an answer to an older call is dropped. */
let latest = 0;

/** Drops the answers of every `openRecord` call still running: another project opened. */
export function dropOpening(): void {
  latest += 1;
}

/**
 * Opens the record `target` in a tab, or focuses its file's tab, after loading its page; its path,
 * when known, picks the file among those with the id. Shows the tree tab of its kind with its row
 * unfolded, loading that tab only when it is not loaded. A failure is kept in `pages.error`, and
 * no tab opens. When calls overlap, the last one wins.
 */
export async function openRecord(target: RecordTarget): Promise<void> {
  const { id } = target;
  const ticket = ++latest;
  let page;
  try {
    [page] = await Promise.all([call(commands.recordView(id, target.path)), loadKinds()]);
  } catch (failure) {
    if (ticket === latest) {
      pages.error = `${id} could not be opened: ${failureText(failure)}`;
    }
    return;
  }
  if (ticket !== latest) {
    return;
  }
  const { record } = page.header;
  storePage(recordKey(record), page);
  openRecordTab(record.id, record.title ?? record.id, record.kind, record.path);
  const tab = treeTabOf(record.kind);
  if (tab === null) {
    return;
  }
  codexTabs.tree = tab;
  const nodes = tree.nodes[tab] ?? (await loadTree(tab));
  if (nodes !== null && ticket === latest) {
    expandTo(nodes, recordKey(record));
  }
}

/** Moves to the record tab `step` places on, wrapping; nothing with fewer than two tabs. */
function moveTab(step: number): void {
  const count = codexTabs.records.length;
  if (count < 2) {
    return;
  }
  codexTabs.active = (codexTabs.active + step + count) % count;
}

/** Makes the next record tab active. */
export function nextTab(): void {
  moveTab(1);
}

/** Makes the previous record tab active. */
export function previousTab(): void {
  moveTab(-1);
}

/** Closes the active record tab; the tab after it, else before it, becomes active. */
export function closeTab(): void {
  const key = activeKey();
  if (key !== null) {
    closeRecordTab(key);
  }
}

/**
 * Closes the tab of the file `key` (`recordKey`), if one is open. The active tab stays active;
 * when it is the one closed, the tab after it, else before it, becomes active.
 */
export function closeRecordTab(key: string): void {
  const at = codexTabs.records.findIndex((tab) => tab.key === key);
  if (at < 0) {
    return;
  }
  codexTabs.records.splice(at, 1);
  if (at < codexTabs.active) {
    codexTabs.active -= 1;
  } else {
    codexTabs.active = Math.min(codexTabs.active, codexTabs.records.length - 1);
  }
  forgetPage(key);
}

/** The tree window's tab selection, which the window draws and sets. */
export const treeSelection: TabSelection = {
  get index() {
    return TREE_TABS.findIndex((entry) => entry.tab === codexTabs.tree);
  },
  select(index) {
    const entry = TREE_TABS[index];
    if (entry !== undefined) {
      codexTabs.tree = entry.tab;
    }
  },
};

/** The record window's tab selection: the active record tab. */
export const recordSelection: TabSelection = {
  get index() {
    return codexTabs.active;
  },
  select(index) {
    if (index >= 0 && index < codexTabs.records.length) {
      codexTabs.active = index;
    }
  },
};

/** The record window's tab titles: a glyph and id per open file, else the window's title. */
export function recordTabTitles(): readonly (RecordTabTitle | string)[] {
  return codexTabs.records.length === 0
    ? ['Record']
    : codexTabs.records.map((tab) => ({ title: tab.id, kind: tab.kind, key: tab.key }));
}

/** The side window's active tab: 0 for Front matter, 1 for Links in. */
const side = $state({ index: 0 });

/** The front-matter window's tab selection, which its body reads to know what to draw. */
export const sideSelection: TabSelection = {
  get index() {
    return side.index;
  },
  select(index) {
    side.index = index;
  },
};
