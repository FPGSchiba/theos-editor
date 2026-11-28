<!-- The record window's body (P11 §5.8, screen 01): the active tab's record page, its head in
     the Instrument register and its body in the manuscript register, then the lore it carries and
     its pictures. Empty sections are not drawn. Another file's page starts at its top, also one
     with the same id (the block is keyed by the file, `recordKey`). -->
<script lang="ts">
  import { recordKey, type RecordTarget } from '../../lib/record-key';
  import EmptyWindow from '../placeholder/EmptyWindow.svelte';
  import Body from './Body.svelte';
  import LoreTable from './LoreTable.svelte';
  import { pageOf, pages } from './pages.svelte';
  import Pictures from './Pictures.svelte';
  import RecordHeader from './RecordHeader.svelte';
  import { activeKey, openRecord } from './tabs.svelte';

  const page = $derived(pageOf(activeKey()));
  const open = (record: RecordTarget): void => void openRecord(record);
</script>

{#if page === null}
  <EmptyWindow mark="◈" text="The record you open. Pick one in the tree." />
{:else}
  {#key recordKey(page.header.record)}
    <div class="tu-window__body tu-manuscript">
      <div class="tu-instrument"><RecordHeader header={page.header} /></div>
      <div class="content">
        {#if page.body.length > 0}
          <div class="tu-prose"><Body nodes={page.body} onopen={open} /></div>
        {/if}
        {#if page.lore.length > 0}
          <h2 class="ms-h2">Lore this {page.header.record.kind ?? 'record'} carries</h2>
          <LoreTable rows={page.lore} onopen={open} />
        {/if}
        {#if page.pictures.length > 0}
          <h2 class="ms-h2">Pictures</h2>
          <Pictures rows={page.pictures} />
        {/if}
      </div>
    </div>
  {/key}
{/if}
{#if pages.error !== null}
  <p class="error inst-body" role="alert">{pages.error}</p>
{/if}

<style>
  .content {
    display: flex;
    flex-direction: column;
    gap: var(--sp-20);
    padding: var(--sp-20) var(--sp-24) var(--sp-32);
  }

  h2 {
    margin: 0;
    color: var(--text);
  }

  .error {
    margin: 0;
    padding: var(--sp-6) var(--sp-10);
    border-top: 1px solid var(--border);
    color: var(--sig-reject);
  }
</style>
