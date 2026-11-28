<!-- One front-matter value (P11 §5.8): text, a link chip (red when it dangles), a list of values,
     the read-only tier selector (`tu-tier`), or the rows of a map such as a lens. -->
<script lang="ts">
  import type { RecordTarget } from '../../lib/record-key';
  import type { FieldValue } from '../../lib/ipc/bindings';
  import FieldValueView from './FieldValue.svelte';
  import LinkChip from './LinkChip.svelte';

  interface Props {
    /** The value. */
    value: FieldValue;
    /** Opens a chip's record. */
    onopen?: ((record: RecordTarget) => void) | undefined;
  }

  let { value, onopen }: Props = $props();

  /** The tiers the selector shows. */
  const TIERS = ['T1', 'T2', 'T3'];
</script>

{#if value.type === 'text'}
  <span>{value.value}</span>
{:else if value.type === 'link'}
  <LinkChip id={value.value.id} shown={value.value.shown} target={value.value.target} {onopen} />
{:else if value.type === 'list'}
  {#each value.value as item, index (index)}
    <FieldValueView value={item} {onopen} />
  {/each}
{:else if value.type === 'tier'}
  <span class="tu-tier tu-tier--sm" role="img" aria-label="tier {value.value}">
    {#each TIERS as tier (tier)}
      <span class="tu-tier__opt" class:is-active={tier === value.value}
        ><span class="tu-tier__n">{tier}</span></span
      >
    {/each}
  </span>
{:else if value.type === 'map'}
  <span class="map">
    {#each value.value as row (row.key)}
      <span class="entry"><span class="tu-dim">{row.key}</span> <FieldValueView value={row.value} {onopen} /></span>
    {/each}
  </span>
{/if}

<style>
  .map {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }
</style>
