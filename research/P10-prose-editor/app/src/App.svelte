<script lang="ts">
  /** The toolbar of the harness: file picker over the corpus, variant picker, Write/Source switch. */
  import { onMount } from 'svelte';
  import { corpusEntries } from './harness/corpus-files.ts';
  import { harness } from './harness/bridge.ts';
  import { variants } from './harness/registry.ts';
  import type { Mode } from './harness/types.ts';

  let path = $state('');
  let variant = $state('');
  let mode = $state<Mode>('write');
  let error = $state('');

  // The bridge is also driven by the tests; the toolbar follows what it reports.
  onMount(() =>
    harness.subscribe(() => {
      const state = harness.state();
      path = state.fileId ?? path;
      variant = state.variant ?? variant;
      mode = state.mode;
    }),
  );

  async function load(): Promise<void> {
    const entry = corpusEntries.find((candidate) => candidate.path === path);
    if (entry === undefined || variant === '') return;
    error = '';
    try {
      await harness.load(variant, entry.text, entry.path);
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
      console.error(caught);
    }
  }

  function switchTo(next: Mode): void {
    try {
      harness.setMode(next);
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    }
  }
</script>

<header class="hx-toolbar tu-instrument">
  <span class="hx-toolbar__brand">P10 · prose editor harness</span>
  <label class="tu-row">
    <span class="tu-dim">file</span>
    <select class="hx-select" data-testid="file-picker" value={path} onchange={(event) => { path = event.currentTarget.value; void load(); }}>
      <option value="" disabled>pick a corpus file</option>
      {#each corpusEntries as entry (entry.path)}
        <option value={entry.path}>{entry.path}</option>
      {/each}
    </select>
  </label>
  <label class="tu-row">
    <span class="tu-dim">variant</span>
    <select class="hx-select" data-testid="variant-picker" value={variant} onchange={(event) => { variant = event.currentTarget.value; void load(); }}>
      <option value="" disabled>pick a variant</option>
      {#each variants as info (info.name)}
        <option value={info.name}>{info.name}</option>
      {/each}
    </select>
  </label>
  <span class="tu-seg" role="group" aria-label="Mode">
    <button type="button" class="tu-seg__opt" class:is-active={mode === 'write'} onclick={() => switchTo('write')}>Write</button>
    <button type="button" class="tu-seg__opt" class:is-active={mode === 'source'} onclick={() => switchTo('source')}>Source</button>
  </span>
  {#if error !== ''}<span class="hx-error" role="alert">{error}</span>{/if}
</header>
