<!-- The Codex tree's body (P11 §5.8, screen 01): the rows of the active tree tab, the Where tab's
     footer, and what an empty tab will show and how to add a record. -->
<script lang="ts">
  import Note from '../../components/Note.svelte';
  import type { TreeNode } from '../../lib/ipc/bindings';
  import type { RecordTarget } from '../../lib/record-key';
  import { project } from '../../lib/state/project.svelte';
  import { isRootKind, kinds as table, loadKinds, tabInfo } from './kinds.svelte';
  import { activeKey, codexTabs, openRecord, TREE_TABS } from './tabs.svelte';
  import TreeRow from './TreeRow.svelte';
  import { keepFor, loadTree, nodeKey, toggleFold, tree } from './tree.svelte';

  const title = $derived(TREE_TABS.find((tab) => tab.tab === codexTabs.tree)?.title);
  const entry = $derived(tabInfo(codexTabs.tree));
  const error = $derived(tree.errors[codexTabs.tree] ?? table.error ?? undefined);

  /** Whether `node` sits at the top of a nested tab without being of its top kind. */
  const stray = (node: TreeNode): boolean =>
    entry !== undefined && entry.root !== null && node.type === 'record' && !isRootKind(node.record.kind);

  /** The tab's top rows: the real roots first, then the records outside the tree. */
  const nodes = $derived.by(() => {
    const top = tree.nodes[codexTabs.tree];
    return top === undefined
      ? undefined
      : [...top.filter((node) => !stray(node)), ...top.filter(stray)];
  });

  /** The kinds of the tab as a phrase: `a, b or c`. */
  const kinds = $derived.by(() => {
    const all = entry?.kinds ?? [];
    return all.length < 2 ? all.join('') : `${all.slice(0, -1).join(', ')} or ${all.at(-1) ?? ''}`;
  });

  $effect(() => {
    keepFor(project.view);
  });

  $effect(() => {
    const tab = codexTabs.tree;
    if (project.view !== null) {
      void loadKinds();
      if (tree.nodes[tab] === undefined && tree.errors[tab] === undefined) {
        void loadTree(tab);
      }
    }
  });

  const open = (record: RecordTarget): void => void openRecord(record);
</script>

<div class="tu-window__body">
  {#if error !== undefined}
    <Note alert>The tree could not be loaded: {error}</Note>
  {:else if nodes !== undefined && nodes.length === 0}
    <div class="tu-empty">
      <span class="tu-empty__mark" aria-hidden="true">◈</span>
      <p class="inst-body">
        Records whose kind is {kinds} appear here. Add one with <code>theos new</code>, or as a file
        under <code>{entry?.folder}</code>.
      </p>
    </div>
  {:else if nodes !== undefined}
    <div class="tu-tree" role="tree" aria-label={title}>
      {#each nodes as node (nodeKey(node))}
        <TreeRow
          {node}
          stray={stray(node)}
          selected={activeKey()}
          folded={tree.folded}
          ontoggle={toggleFold}
          onopen={open}
        />
      {/each}
    </div>
  {/if}
  {#if codexTabs.tree === 'where'}
    <div class="tree-footer tu-mono data-sm">
      <span>Where: one parent per place</span>
      <span>Who: people → faction → NPC</span>
      <span>folders follow this tree; the id is the identity</span>
    </div>
  {/if}
</div>

<style>
  .tu-window__body {
    display: flex;
    flex-direction: column;
  }

  .tu-empty p {
    margin: 0;
    max-width: 44ch;
  }

  .tree-footer {
    margin-top: auto;
    padding: var(--sp-8) var(--sp-10);
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    color: var(--textDim);
  }
</style>
