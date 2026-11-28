<!-- The record window's body (P11 §5.8, screen 01): the active tab's record page, its head in
     the Instrument register and its body in the manuscript register, then the lore it carries and
     its pictures. Empty sections are not drawn. Another record's page starts at its top. -->
<script lang="ts">
  import EmptyWindow from '../placeholder/EmptyWindow.svelte';
  import Body from './Body.svelte';
  import LoreTable from './LoreTable.svelte';
  import { pageOf, pages } from './pages.svelte';
  import Pictures from './Pictures.svelte';
  import RecordHeader from './RecordHeader.svelte';
  import { activeRecord, openRecord } from './tabs.svelte';

  const page = $derived(pageOf(activeRecord()));
  const open = (id: string): void => void openRecord(id);
</script>

{#if page === null}
  <EmptyWindow mark="◈" text="The record you open. Pick one in the tree." />
{:else}
  {#key page.header.record.id}
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
