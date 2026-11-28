<!-- The command palette (`tu-pop`, Instrument; P11 §5.11): every command with its label and keys,
     filtered by `fuzzysort` as you type. Disabled commands stay listed, dimmed, with the phase that
     brings them, and Enter does nothing on them. Arrows move, Enter runs, Escape closes and gives
     the focus back. The palette is modal: Tab stays in it, and the shell runs no other command
     while it is open. -->
<script lang="ts">
  import fuzzysort from 'fuzzysort';

  import { commands, runCommand } from '../lib/commands';
  import type { CommandId, Os } from '../lib/keymap';
  import { paletteRows } from '../lib/palette-rows';
  import { closePalette } from '../lib/state/palette.svelte';

  interface Props {
    /** The OS, which picks how the keys are written. */
    os: Os;
  }

  let { os }: Props = $props();

  const rows = $derived(paletteRows(os));
  let query = $state('');
  let selected = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let list: HTMLElement | undefined = $state();

  const shown = $derived(
    query.trim() === ''
      ? rows
      : fuzzysort.go(query, rows, { key: 'label' }).map((result) => result.obj),
  );

  /** The id of the option element of `command`. */
  const optionId = (command: CommandId): string => `palette-option-${command.replace('.', '-')}`;

  const activeRow = $derived(shown[selected]);
  const activeId = $derived(activeRow === undefined ? undefined : optionId(activeRow.id));

  $effect(() => {
    input?.focus();
  });

  // The selected row stays in view as the arrows move it.
  $effect(() => {
    if (activeId !== undefined) {
      list?.querySelector(`#${activeId}`)?.scrollIntoView({ block: 'nearest' });
    }
  });

  /** Runs the row at `index` when its command is enabled, after closing the palette. */
  async function run(index: number): Promise<void> {
    const row = shown[index];
    if (row === undefined || !commands[row.id].enabled()) {
      return;
    }
    await closePalette();
    runCommand(row.id);
  }

  /** Runs the row the user clicked (the listbox takes the clicks of its rows). */
  function onclick(event: MouseEvent): void {
    const row = event.target instanceof Element ? event.target.closest('[data-index]') : null;
    if (row instanceof HTMLElement) {
      void run(Number(row.dataset['index']));
    }
  }

  /**
   * The palette's keys, from the query field or from a row that has the focus. Tab keeps the
   * focus in the query field (the dialog is modal), and Enter does nothing while an IME composes.
   */
  function onkeydown(event: KeyboardEvent): void {
    const count = shown.length;
    if (event.key === 'Escape') {
      void closePalette();
    } else if (event.key === 'Tab') {
      input?.focus();
    } else if (event.key === 'ArrowDown' && count > 0) {
      selected = (selected + 1) % count;
    } else if (event.key === 'ArrowUp' && count > 0) {
      selected = (selected - 1 + count) % count;
    } else if (event.key === 'Enter' && !event.isComposing) {
      void run(selected);
    } else {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
  }
</script>

<div class="scrim" role="presentation" onclick={() => void closePalette()}></div>
<div
  class="tu-pop tu-instrument palette"
  role="dialog"
  aria-modal="true"
  aria-label="Command palette"
  tabindex="-1"
  {onkeydown}
>
  <div class="tu-pop__head">
    <input
      bind:this={input}
      bind:value={query}
      oninput={() => (selected = 0)}
      class="query"
      type="text"
      role="combobox"
      aria-label="Command"
      aria-expanded="true"
      aria-controls="palette-commands"
      aria-autocomplete="list"
      aria-activedescendant={activeId}
      placeholder="type a command"
    />
  </div>
  <div
    bind:this={list}
    id="palette-commands"
    class="rows"
    role="listbox"
    aria-label="Commands"
    tabindex="-1"
    {onclick}
    {onkeydown}
  >
    {#each shown as row, index (row.id)}
      {@const enabled = commands[row.id].enabled()}
      {@const phase = commands[row.id].phase}
      <div
        id={optionId(row.id)}
        class="tu-pop__row row"
        class:is-active={index === selected}
        class:is-off={!enabled}
        role="option"
        tabindex="-1"
        aria-selected={index === selected}
        aria-disabled={!enabled}
        data-index={index}
        onmouseenter={() => (selected = index)}
      >
        <span class="tu-pop__title">{row.label}</span>
        {#if phase !== undefined}<span class="tu-pop__kind">arrives with {phase}</span>{/if}
        <span class="tu-kbd">{row.keys}</span>
      </div>
    {:else}
      <div class="tu-pop__row row is-note">No command matches.</div>
    {/each}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 10;
  }

  .palette {
    position: fixed;
    top: var(--sp-48);
    left: 50%;
    transform: translateX(-50%);
    width: 520px;
    z-index: 11;
  }

  .query {
    flex: 1;
    background: transparent;
    border: 0;
    outline: none;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: inherit;
  }

  .rows {
    max-height: 60vh;
    overflow-y: auto;
  }

  .row {
    grid-template-columns: minmax(0, 1fr) auto auto;
    cursor: default;
  }

  /* Only a disabled option is muted; a note is text to read. */
  .row.is-off {
    color: var(--textMute);
  }

  .row.is-note {
    color: var(--textDim);
  }
</style>
