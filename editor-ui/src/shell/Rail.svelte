<!-- The rail (`tu-rail`, P11 §5.3): one button per entry, in rail order; each switches the whole
     layout. Settings sits alone at the bottom (Theos UI ActivityRail). -->
<script lang="ts">
  import type { RailEntry } from '../lib/ipc/bindings';
  import { railOrder } from '../lib/rail';

  interface Props {
    /** The entries, in any order; drawn by `order`. */
    entries: readonly RailEntry[];
    /** The id of the active entry. */
    active: string;
    /** Called with the id of the entry the user picks. */
    onselect: (id: string) => void;
  }

  /** The entry that sits at the bottom, after the spacer. */
  const BOTTOM = 'settings';

  let { entries, active, onselect }: Props = $props();
  const sorted = $derived(railOrder(entries));
  const top = $derived(sorted.filter((entry) => entry.id !== BOTTOM));
  const bottom = $derived(sorted.filter((entry) => entry.id === BOTTOM));
</script>

{#snippet item(entry: RailEntry)}
  <button
    class="tu-rail__item"
    class:is-active={entry.id === active}
    type="button"
    aria-current={entry.id === active ? 'true' : undefined}
    onclick={() => onselect(entry.id)}
    ><span class="tu-rail__glyph" aria-hidden="true">{entry.glyph}</span><span
      class="tu-rail__label">{entry.label}</span
    ></button
  >
{/snippet}

<nav class="tu-rail" aria-label="Layout presets">
  {#each top as entry (entry.id)}{@render item(entry)}{/each}
  <div class="tu-rail__spacer"></div>
  {#each bottom as entry (entry.id)}{@render item(entry)}{/each}
</nav>

<style>
  /* Screen 01's reset of a button that is a rail item. */
  button.tu-rail__item {
    width: 100%;
    padding: 0;
    background: transparent;
    border: 0;
    border-left: 2px solid transparent;
  }

  button.tu-rail__item.is-active {
    background: color-mix(in srgb, var(--sig-attention) 6%, transparent);
    border-left-color: var(--sig-attention);
  }
</style>
