<!-- The lore a record carries (screen 01's table): the truths, beliefs and clues that link to it,
     each with its id, title, what it touches and how, and its tier. A `never` truth is marked. -->
<script lang="ts">
  import KindGlyph from '../../components/KindGlyph.svelte';
  import type { LoreRow } from '../../lib/ipc/bindings';
  import { recordKey } from '../../lib/record-key';

  interface Props {
    /** The rows; the caller draws the table only when there is one. */
    rows: readonly LoreRow[];
    /** Opens a row's record. */
    onopen: (id: string) => void;
  }

  let { rows, onopen }: Props = $props();
</script>

<div class="lore-table tu-instrument">
  <div class="grid head">
    <span></span><span class="tu-label tu-label--xs">ID</span><span class="tu-label tu-label--xs"
      >What</span
    ><span class="tu-label tu-label--xs">Touches</span><span class="tu-label tu-label--xs">Tier</span>
  </div>
  {#each rows as row (recordKey(row.record))}
    <button class="grid lore-row data" type="button" onclick={() => onopen(row.record.id)}>
      <KindGlyph kind={row.record.kind ?? 'record'} />
      <span>{row.record.id}</span>
      <span class="what inst-body">{row.record.title ?? row.record.id}</span>
      <span class="tu-dim"
        >{row.verb ?? '—'}
        {#if row.touches !== null}<KindGlyph kind={row.touches.kind ?? 'record'} />{row.touches
            .id}{#if row.touches.never}
            <span class="never" data-never="true">· never</span>{/if}{/if}</span
      >
      <span>{#if row.tier !== null}<span class="tu-tiertag">{row.tier}</span>{/if}</span>
    </button>
  {/each}
</div>

<style>
  .lore-table {
    max-width: 70ch;
    border: 1px solid var(--border);
    background: var(--panel);
  }

  .grid {
    display: grid;
    grid-template-columns: 22px 58px minmax(0, 1fr) 150px 36px;
    align-items: center;
    gap: var(--sp-8);
    padding: var(--sp-6) var(--sp-10);
  }

  .head {
    background: var(--panel2);
    border-bottom: 1px solid var(--border);
  }

  .lore-row {
    width: 100%;
    border: 0;
    border-top: 1px dashed var(--border);
    background: transparent;
    color: var(--textDim);
    text-align: left;
    cursor: pointer;
  }

  .lore-row:first-of-type {
    border-top: 0;
  }

  .what {
    color: var(--text);
  }

  .never {
    margin-left: var(--sp-4);
    color: var(--sig-reject);
  }
</style>
