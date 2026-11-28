<!-- "Used by" (P11 §5.8, CE-33.2): the records that link to the open one, one row per source with
     its relations. It is the index's backlinks at the time of the view; nothing is stored on the
     record. -->
<script lang="ts">
  import Note from '../../components/Note.svelte';
  import PropRow from '../../components/PropRow.svelte';
  import { recordKey } from '../../lib/record-key';
  import { isIdFirst } from './kinds.svelte';
  import { pageOf } from './pages.svelte';
  import { activeRecord, openRecord } from './tabs.svelte';

  const page = $derived(pageOf(activeRecord()));
</script>

<div class="tu-window__body">
  {#if page === null}
    <Note>The records that use the open one, and how.</Note>
  {:else}
    {#each page.used_by as row (recordKey(row.source))}
      <PropRow
        kind={row.source.kind}
        value={isIdFirst(row.source.kind)
          ? `${row.source.id} ${row.source.title ?? ''}`
          : (row.source.title ?? row.source.id)}
        type={row.relations.join(' · ')}
        onclick={() => void openRecord(row.source.id)}
      />
    {:else}
      <Note>No record uses this one.</Note>
    {/each}
  {/if}
</div>

