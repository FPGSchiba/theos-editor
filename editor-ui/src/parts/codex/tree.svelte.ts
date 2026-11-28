// The rows of the Codex tree (P11 §5.8): each tab's nodes as `codex_tree` answered, and the rows
// the user folded. Every row starts unfolded; opening a record unfolds the rows above it.

import { call, failureText } from '../../lib/ipc/client';
import { commands, type TreeNode, type TreeTab } from '../../lib/ipc/bindings';
import { recordKey } from '../../lib/record-key';

/** The tree's state. */
export interface TreeState {
  /** The nodes of each loaded tab. */
  nodes: Partial<Record<TreeTab, TreeNode[]>>;
  /** Why a tab could not be loaded. */
  errors: Partial<Record<TreeTab, string>>;
  /** The keys of the folded rows (`nodeKey`). */
  folded: string[];
}

/** The tree's state. */
export const tree = $state<TreeState>({ nodes: {}, errors: {}, folded: [] });

/**
 * The key of a row: the record's file (`recordKey`), or `group:` and the group's label. Two files
 * with one id are two rows, and each folds on its own.
 */
export function nodeKey(node: TreeNode): string {
  return node.type === 'record' ? recordKey(node.record) : `group:${node.label}`;
}

/** The loads that run, by tab, so a second call waits for the first. */
const running: Partial<Record<TreeTab, Promise<TreeNode[] | null>>> = {};

/** Loads the rows of `tab`; `null`, with the reason in `errors`, when that fails. */
export function loadTree(tab: TreeTab): Promise<TreeNode[] | null> {
  const load =
    running[tab] ??
    (async () => {
      try {
        const nodes = await call(commands.codexTree(tab));
        tree.nodes[tab] = nodes;
        delete tree.errors[tab];
        return nodes;
      } catch (failure) {
        tree.errors[tab] = failureText(failure);
        return null;
      } finally {
        delete running[tab];
      }
    })();
  running[tab] = load;
  return load;
}

/** The project view the loaded rows belong to. */
let shownView: unknown = null;

/** Drops the loaded rows when `view` is not the view they were loaded for (a reload). */
export function keepFor(view: unknown): void {
  if (view !== shownView) {
    shownView = view;
    tree.nodes = {};
    tree.errors = {};
  }
}

/** Folds the row `key` when it is unfolded, else unfolds it. */
export function toggleFold(key: string): void {
  const at = tree.folded.indexOf(key);
  if (at >= 0) {
    tree.folded.splice(at, 1);
  } else {
    tree.folded.push(key);
  }
}

/** The keys of the rows above the row `key` in `nodes`, outermost first; `null` if absent. */
function ancestors(nodes: readonly TreeNode[], key: string): string[] | null {
  for (const node of nodes) {
    if (node.type === 'record' && nodeKey(node) === key) {
      return [];
    }
    const below = ancestors(node.children, key);
    if (below !== null) {
      return [nodeKey(node), ...below];
    }
  }
  return null;
}

/** Unfolds the rows above the record row `key` (`recordKey`) in `nodes`. */
export function expandTo(nodes: readonly TreeNode[], key: string): void {
  const above = ancestors(nodes, key) ?? [];
  tree.folded = tree.folded.filter((key) => !above.includes(key));
}
