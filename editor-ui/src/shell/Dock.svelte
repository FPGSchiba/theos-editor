<!-- Draws a dock tree (P11 §5.6): a row or column of windows and nested splits, each with its
     flex size. -->
<script lang="ts">
  import Window from '../components/Window.svelte';
  import { isSplit, type Split } from '../parts/layout';
  import Dock from './Dock.svelte';

  interface Props {
    /** The tree to draw. */
    split: Split;
  }

  let { split }: Props = $props();
</script>

<div class="split" class:column={split.dir === 'column'}>
  {#each split.items as item, index (index)}
    <div class="item" style:flex={item.flex}>
      {#if isSplit(item.child)}
        <Dock split={item.child} />
      {:else}
        <Window spec={item.child} />
      {/if}
    </div>
  {/each}
</div>

<style>
  .split {
    flex: 1;
    display: flex;
    gap: var(--sp-4);
    min-width: 0;
    min-height: 0;
  }

  .column {
    flex-direction: column;
  }

  .item {
    display: flex;
    min-width: 0;
    min-height: 0;
  }

  .item > :global(.tu-window) {
    flex: 1;
  }
</style>
