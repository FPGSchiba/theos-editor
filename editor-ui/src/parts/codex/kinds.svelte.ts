// What the Codex knows of kinds (P11 §5.8), from `codex_tabs`, the table the tree is built from:
// the kinds of each tree tab, their folder, the kinds shown id first and the kind at the top of a
// nested tab. The UI keeps no copy of that table; it loads it once.

import { failureText } from '../../lib/ipc/client';
import { commands, type TreeTab, type TreeTabInfo } from '../../lib/ipc/bindings';

/** The tabs as `codex_tabs` answered, and why they could not be loaded. */
export interface Kinds {
  tabs: TreeTabInfo[];
  error: string | null;
}

/** The tree's table. */
export const kinds = $state<Kinds>({ tabs: [], error: null });

/** The load that runs or ran, so the table is asked for once. */
let loading: Promise<void> | null = null;

/** Loads the table once; later calls wait for the same load. A failure may be retried. */
export function loadKinds(): Promise<void> {
  loading ??= (async () => {
    try {
      kinds.tabs = await commands.codexTabs();
      kinds.error = null;
    } catch (failure) {
      kinds.error = failureText(failure);
      loading = null;
    }
  })();
  return loading;
}

/** The table entry of `tab`, once loaded. */
export function tabInfo(tab: TreeTab): TreeTabInfo | undefined {
  return kinds.tabs.find((entry) => entry.tab === tab);
}

/** The tree tab that shows records of `kind`, or `null` for a kind the tree does not show. */
export function treeTabOf(kind: string | null): TreeTab | null {
  return kinds.tabs.find((entry) => kind !== null && entry.kinds.includes(kind))?.tab ?? null;
}

/** Whether rows and chips of `kind` show the id before the title. */
export function isIdFirst(kind: string | null): boolean {
  return kind !== null && kinds.tabs.some((entry) => entry.id_first.includes(kind));
}

/** Whether `kind` is the kind at the top of a nested tab (the tower, people). */
export function isRootKind(kind: string | null): boolean {
  return kind !== null && kinds.tabs.some((entry) => entry.root === kind);
}
