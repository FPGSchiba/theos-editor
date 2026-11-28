<!-- A link chip (`tu-link`, screen 01): the target's kind glyph, its id for lore kinds, and its
     title or the link's shown text. A link that names no record is a red chip with ✕, and a
     screen reader hears "broken link". A truth with target `never` carries the mark.
     Plain props, so a CodeMirror widget can mount it. -->
<script lang="ts">
  import KindGlyph from '../../components/KindGlyph.svelte';
  import Spoken from '../../components/Spoken.svelte';
  import type { RecordRef } from '../../lib/ipc/bindings';
  import { isIdFirst } from './kinds.svelte';

  interface Props {
    /** The id the link names. */
    id: string;
    /** The text after `|`, if any. */
    shown?: string | null;
    /** The record the id names; `null` when no record has it. */
    target: RecordRef | null;
    /** Opens the target; without it the chip is not a button. */
    onopen?: ((id: string) => void) | undefined;
  }

  let { id, shown = null, target, onopen }: Props = $props();
  const title = $derived(shown ?? target?.title ?? id);
</script>

{#snippet content(record: RecordRef)}
  {#if record.kind !== null}<KindGlyph kind={record.kind} />{/if}{#if isIdFirst(record.kind)}<span
      class="tu-link__id">{record.id}</span
    >{/if}<span class="tu-link__title">{title}</span>{#if record.never}<span class="never"
      >· never</span
    >{/if}
{/snippet}

{#if target === null}
  <span class="tu-link is-broken is-reject" title="{id} names no record"
    ><Spoken>broken link:</Spoken><span class="tu-link__id" aria-hidden="true">✕</span><span class="tu-link__title">{title}</span
    ></span
  >
{:else if onopen === undefined}
  <span class="tu-link" data-kind={target.kind} data-never={target.never ? 'true' : undefined}
    >{@render content(target)}</span
  >
{:else}
  <button
    class="tu-link"
    type="button"
    data-kind={target.kind}
    data-never={target.never ? 'true' : undefined}
    onclick={() => onopen(target.id)}>{@render content(target)}</button
  >
{/if}

<style>
  button.tu-link {
    cursor: pointer;
  }

  .never {
    color: var(--sig-reject);
    font-family: var(--font-mono);
  }
</style>
