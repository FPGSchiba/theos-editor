<!-- The head of a record page (`tu-rechead`, screen 01): glyph, id, kind and parent, the status
     chip, the title, and the meta line with the path, the links in and the children. -->
<script lang="ts">
  import KindGlyph from '../../components/KindGlyph.svelte';
  import type { Header } from '../../lib/ipc/bindings';
  import { count } from '../../lib/counts';

  interface Props {
    /** The record's header. */
    header: Header;
  }

  let { header }: Props = $props();
  const record = $derived(header.record);
  const kind = $derived(
    [record.kind ?? 'record', header.parent === null ? '' : `in ${header.parent.title ?? header.parent.id}`]
      .filter((part) => part !== '')
      .join(' · '),
  );
</script>

<header class="tu-rechead">
  <div class="tu-rechead__top">
    {#if record.kind !== null}<span class="tu-rechead__glyph"><KindGlyph kind={record.kind} /></span>{/if}
    <span class="tu-rechead__id">{record.id}</span>
    <span class="tu-rechead__kind">{kind}</span>
    {#if record.status !== null}
      <span class="tu-rechead__right"
        ><span class="tu-status" data-status={record.status}>{record.status}</span></span
      >
    {/if}
  </div>
  <h1 class="tu-rechead__title">{record.title ?? record.id}</h1>
  <div class="tu-rechead__meta">
    {#if record.path !== null}<span>{record.path}</span>{/if}
    <span>{count(header.links_in, 'link in', 'links in')}</span>
    {#if header.children > 0}<span>{count(header.children, 'child', 'children')}</span>{/if}
  </div>
</header>

<style>
  .tu-status {
    text-transform: capitalize;
  }
</style>
