<!-- The toolbar's find field (P11 §5.10): hits as you type (`#id`, `:kind`, `@people`), arrows move,
     Enter opens the record in the Codex, Escape closes the list. A hit shows the kind glyph, the
     id, the title and its first match's line, the matched slice in a `mark`. -->
<script lang="ts">
  import KindGlyph from '../components/KindGlyph.svelte';
  import { recordKey } from '../lib/record-key';
  import { registerFindField } from '../lib/state/find-field';
  import {
    closeFind,
    find,
    markedLine,
    moveSelection,
    reopenFind,
    setQuery,
  } from '../lib/state/find.svelte';
  import { activate, CODEX } from '../lib/state/project.svelte';
  import { openRecord } from '../parts/codex/tabs.svelte';

  let input: HTMLInputElement | null = $state(null);
  let box: HTMLElement | undefined = $state();

  $effect(() => {
    registerFindField(input);
    return () => registerFindField(null);
  });

  const activeId = $derived(find.selected >= 0 ? `find-hit-${find.selected}` : undefined);

  /** Opens the hit at `index` in the Codex, which it shows first, then closes the list and empties the field. */
  function open(index: number): void {
    const hit = find.hits[index];
    if (hit === undefined) {
      return;
    }
    setQuery('');
    activate(CODEX);
    void openRecord(hit.id);
  }

  /** The field's keys: arrows move, Enter opens the highlighted hit, or the first (not while an IME composes), Escape closes. */
  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      if (!find.open) {
        return;
      }
      closeFind();
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      if (!find.open) {
        reopenFind();
      }
      moveSelection(event.key === 'ArrowDown' ? 1 : -1);
    } else if (event.key === 'Enter' && !event.isComposing && find.open) {
      open(Math.max(find.selected, 0));
    } else {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
  }

  /** Closes the list when the focus leaves the field and the list. */
  function onfocusout(event: FocusEvent): void {
    const next = event.relatedTarget;
    if (!(next instanceof Node) || !box?.contains(next)) {
      closeFind();
    }
  }
</script>

<div class="box" bind:this={box} {onfocusout}>
  <label class="tu-input find">
    <span class="tu-input__icon" aria-hidden="true">~</span>
    <input
      bind:this={input}
      value={find.query}
      oninput={(event) => setQuery(event.currentTarget.value)}
      onfocus={reopenFind}
      {onkeydown}
      type="text"
      role="combobox"
      aria-label="Find record"
      aria-expanded={find.open}
      aria-controls={find.open && find.hits.length > 0 ? 'find-results' : undefined}
      aria-autocomplete="list"
      aria-activedescendant={activeId}
      placeholder="find record   #id   @people   :kind"
    />
  </label>
  {#if find.open}
    <div class="tu-pop results" role="presentation" onmousedown={(event) => event.preventDefault()}>
      {#if find.hits.length > 0}
        <div id="find-results" role="listbox" aria-label="Search results">
          {#each find.hits as hit, index (recordKey(hit))}
            {@const line = markedLine(hit)}
            <div
              id="find-hit-{index}"
              class="tu-pop__row row"
              class:is-active={index === find.selected}
              role="option"
              tabindex="-1"
              aria-selected={index === find.selected}
              onclick={() => open(index)}
              {onkeydown}
              onmouseenter={() => (find.selected = index)}
            >
              <span class="head">
                {#if hit.kind !== null}<KindGlyph kind={hit.kind} />{/if}
                <span class="id">{hit.id}</span>
                <span class="tu-pop__title">{hit.title ?? hit.id}</span>
              </span>
              {#if line !== null}
                <span class="line"
                  >{line.before}{#if line.matched !== ''}<mark>{line.matched}</mark>{/if}{line.after}</span
                >
              {/if}
            </div>
          {/each}
        </div>
      {/if}
      {#if find.truncated}
        <div class="tu-pop__row row is-off">more…</div>
      {/if}
      {#if find.note !== null}
        <div class="tu-pop__row row is-off">{find.note}</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .box {
    position: relative;
  }

  .find {
    width: 300px;
  }

  .results {
    position: absolute;
    top: 100%;
    left: 0;
    width: 480px;
    max-height: 60vh;
    overflow-y: auto;
    z-index: 11;
  }

  .row {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-2);
    cursor: default;
  }

  .row.is-off {
    color: var(--textMute);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--sp-8);
  }

  .id {
    font-family: var(--font-mono);
    color: var(--textMute);
  }

  .line {
    font-family: var(--font-mono);
    color: var(--textMute);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  mark {
    background: transparent;
    color: var(--text);
    font-weight: 600;
  }
</style>
