<!-- The tab bar of a window (`tu-tabbar`): one title, or tabs of which one is active. A tab that
     names a record shows its kind glyph before its id, and is a tab even when it is the only one. -->
<script lang="ts">
  import { tabKey, tabText, type TabBadge, type TabTitle } from '../parts/layout';
  import KindGlyph from './KindGlyph.svelte';

  interface Props {
    /** The tab titles; one plain title shows as the window's title. */
    tabs: readonly TabTitle[];
    /** The index of the active tab. */
    active?: number;
    /** Called with the index of the tab the user picks. */
    onselect?: (index: number) => void;
  }

  let { tabs, active = 0, onselect }: Props = $props();
  /** The one title of a window with one tab that is not a record tab. */
  const only = $derived.by(() => {
    const first = tabs[0];
    return tabs.length === 1 && first !== undefined && (typeof first === 'string' || first.kind === undefined)
      ? first
      : null;
  });

  /** The badge of `tab`, if it has one. */
  const badgeOf = (tab: TabTitle): TabBadge | undefined =>
    typeof tab === 'string' ? undefined : tab.badge;
</script>

{#snippet badge(value: TabBadge | undefined)}
  {#if value !== undefined}<span class="tu-badge is-{value.signal}">{value.count}</span>{/if}
{/snippet}

<div class="tu-tabbar">
  {#if only !== null}
    <div class="tu-tabbar__title">
      {tabText(only)}
      {@render badge(badgeOf(only))}
    </div>
  {:else}
    <div class="tu-tabbar__tabs" role="tablist">
      {#each tabs as tab, index (tabKey(tab))}
        <button
          class="tu-tab"
          class:is-active={index === active}
          type="button"
          role="tab"
          aria-selected={index === active}
          onclick={() => onselect?.(index)}
          >{#if typeof tab !== 'string' && tab.kind !== undefined && tab.kind !== null}<KindGlyph
              kind={tab.kind}
            />{/if}{tabText(tab)}
          {@render badge(badgeOf(tab))}</button
        >
      {/each}
    </div>
  {/if}
</div>

<style>
  /* Screen 01's reset of a button that is a tab. */
  button.tu-tab {
    background: transparent;
    border: 0;
    border-right: 1px solid var(--border);
    border-top: 2px solid transparent;
  }

  button.tu-tab.is-active {
    background: var(--panel);
    border-top-color: var(--sig-attention);
  }
</style>
