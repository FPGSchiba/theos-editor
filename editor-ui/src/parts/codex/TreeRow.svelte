<!-- One row of the Codex tree (`tu-tree__row`, Theos UI TreeRow) and the rows under it: a record's
     glyph, title (lore kinds show the id first) and tag, or a group's label. A row with rows under
     it folds. A truth with target `never` carries the mark. The selected row scrolls into view. -->
<script lang="ts">
  import KindGlyph from '../../components/KindGlyph.svelte';
  import type { TreeNode } from '../../lib/ipc/bindings';
  import { recordKey } from '../../lib/record-key';
  import { isIdFirst, isRootKind } from './kinds.svelte';
  import TreeRow from './TreeRow.svelte';
  import { nodeKey } from './tree.svelte';

  interface Props {
    /** The row's node. */
    node: TreeNode;
    /** How deep the row sits; the tab's top rows are 0. */
    depth?: number;
    /** Whether the row sits at the top without being of the tab's top kind (no root). */
    stray?: boolean;
    /** The key (`recordKey`) of the selected record's file, or `null`. */
    selected: string | null;
    /** The keys of the folded rows. */
    folded: readonly string[];
    /** Folds or unfolds the row `key`. */
    ontoggle: (key: string) => void;
    /** Opens the record `id`. */
    onopen: (id: string) => void;
  }

  let { node, depth = 0, stray = false, selected, folded, ontoggle, onopen }: Props = $props();
  const key = $derived(nodeKey(node));
  const open = $derived(!folded.includes(key));
  const parent = $derived(node.children.length > 0);
  const isSelected = $derived(node.type === 'record' && recordKey(node.record) === selected);
  let element: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (isSelected) {
      element?.scrollIntoView({ block: 'nearest' });
    }
  });

  /** The row's name: a lore record's id and title, another record's title, a group's label. */
  const name = $derived.by(() => {
    if (node.type === 'group') {
      return node.label;
    }
    const { id, title, kind } = node.record;
    if (title === null) {
      return id;
    }
    return isIdFirst(kind) ? `${id} ${title}` : title;
  });
</script>

<div
  class="tu-tree__row"
  class:is-root={node.type === 'group' || isRootKind(node.record.kind)}
  class:is-entity={node.type === 'record' && parent}
  bind:this={element}
  class:is-selected={isSelected}
  style:--depth={depth}
  role="treeitem"
  aria-level={depth + 1}
  aria-selected={isSelected}
  aria-expanded={parent ? open : undefined}
  data-never={node.type === 'record' && node.record.never ? 'true' : undefined}
>
  {#if parent}
    <button
      class="fold"
      type="button"
      aria-label={open ? `Fold ${name}` : `Unfold ${name}`}
      onclick={() => ontoggle(key)}>{open ? '▾' : '▸'}</button
    >
  {/if}
  {#if node.type === 'record'}
    <button class="open" type="button" onclick={() => onopen(node.record.id)}>
      <KindGlyph kind={node.record.kind ?? 'record'} /><span class="tu-tree__name">{name}</span>
    </button>
    {#if node.tag !== null || node.record.never || stray}
      <span class="tu-tree__tag"
        >{node.tag ?? ''}{#if node.record.never}<span class="never">· never</span>{/if}{#if stray}<span
            class="stray"
            title="Its parent is missing, names no record, or loops back to it">no root</span
          >{/if}</span
      >
    {/if}
  {:else}
    <button class="open" type="button" onclick={() => ontoggle(key)}>
      <span class="tu-tree__name">{name}</span>
    </button>
  {/if}
</div>
{#if open}
  {#each node.children as child (nodeKey(child))}
    <TreeRow node={child} depth={depth + 1} {selected} {folded} {ontoggle} {onopen} />
  {/each}
{/if}

<style>
  button {
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font-family: var(--font-mono);
    font-size: inherit;
    line-height: inherit;
    cursor: pointer;
  }

  .open {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-8);
    min-width: 0;
    flex: 1;
    text-align: left;
  }

  .fold {
    width: 1em;
    color: var(--textMute);
  }

  .never {
    color: var(--sig-reject);
  }

  .stray {
    color: var(--sig-warn);
  }
</style>
