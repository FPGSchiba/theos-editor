<!-- The side window's body (P11 §5.8, screen 01): the open record's front matter in file order,
     with the disabled "+ field" and its schema, or, on the second tab, every link into it. -->
<script lang="ts">
  import KindGlyph from '../../components/KindGlyph.svelte';
  import Note from '../../components/Note.svelte';
  import PropRow from '../../components/PropRow.svelte';
  import FieldValue from './FieldValue.svelte';
  import { pageOf } from './pages.svelte';
  import { activeRecord, openRecord, sideSelection } from './tabs.svelte';

  const page = $derived(pageOf(activeRecord()));
  const open = (id: string): void => void openRecord(id);
</script>

<div class="tu-window__body">
  {#if page === null}
    <Note>The open record's fields, and the records that link to it.</Note>
  {:else if sideSelection.index === 0}
    <div class="tu-fm tu-instrument">
      {#each page.front_matter as field, index (index)}
        <div class="tu-fm__row">
          <span class="tu-fm__key">{field.key}</span>
          <span class="tu-fm__val">
            {#if field.key === 'kind' && field.value.type === 'text'}
              <span class="tu-kindtag" data-kind={field.value.value}
                ><KindGlyph kind={field.value.value} />{field.value.value}</span
              >
            {:else}
              <FieldValue value={field.value} onopen={open} />
            {/if}
          </span>
        </div>
      {/each}
    </div>
    <div class="foot">
      <button
        class="tu-btn tu-btn--dashed tu-btn--sm"
        type="button"
        disabled
        title="Editing arrives with P12">+ field</button
      >
      <span class="tu-spacer"></span>
      <span class="tu-mono tu-dim data-sm">schema · {page.header.record.kind ?? 'record'} v1</span>
    </div>
  {:else}
    <div class="links-in">
      {#each page.links_in as link, index (index)}
        <PropRow
          kind={link.source.kind}
          value={`${link.source.id} ${link.source.title ?? ''}`}
          type={link.slot}
          onclick={() => open(link.source.id)}
        />
      {:else}
        <Note>No record links here.</Note>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tu-fm {
    border: 0;
  }

  .foot {
    display: flex;
    gap: var(--sp-6);
    align-items: center;
    padding: var(--sp-6) var(--sp-10);
    border-top: 1px solid var(--border);
  }

</style>
