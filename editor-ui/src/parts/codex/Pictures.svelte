<!-- The pictures of a record (screen 01's grid): one empty frame per concept for it, captioned with
     the concept and its pick. Images arrive with P16; nothing is fetched. -->
<script lang="ts">
  import type { PictureRow } from '../../lib/ipc/bindings';
  import { recordKey } from '../../lib/record-key';

  interface Props {
    /** The concepts; the caller draws the grid only when there is one. */
    rows: readonly PictureRow[];
  }

  let { rows }: Props = $props();

  /** The caption of `row`: the concept, and its picked version without the concept's id. */
  function caption(row: PictureRow): string {
    const id = row.concept.id;
    if (row.picked === null) {
      return `◫ ${id} · no pick`;
    }
    const version = row.picked.startsWith(`${id}-`) ? row.picked.slice(id.length + 1) : row.picked;
    return `◫ ${id} · picked ${version}`;
  }
</script>

<div class="pictures">
  {#each rows as row (recordKey(row.concept))}
    <figure>
      <div class="frame data-sm">Images arrive with P16</div>
      <figcaption class="tu-mono data-sm">{caption(row)}</figcaption>
    </figure>
  {/each}
</div>

<style>
  .pictures {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--sp-10);
    max-width: 70ch;
  }

  figure {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-6);
  }

  .frame {
    aspect-ratio: 16 / 9;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--borderHi);
    background: var(--panel3);
    color: var(--textDim);
  }

  figcaption {
    color: var(--textDim);
  }
</style>
