<script lang="ts">
  /**
   * The head of a note callout: `> [!note] n1 · [[f04-terrain-v2]] · open`. The adapter owns the
   * block around it, `div[data-callout="note"][data-note=<n>][data-note-state=<state>]`, and the
   * editable body (see the Showcase for the markup); the head shows the title line rendered.
   */
  import type { EditorContext } from '../harness/types.ts';
  import LinkChip from './LinkChip.svelte';

  interface Props {
    /** The note's id, e.g. `n1`. */
    note: string;
    /** The version the note is about, if the title names one. */
    version?: string;
    state: 'open' | 'dropped' | 'resolved';
    /** The version that resolves the note (`resolved by [[id]]`). */
    resolvedBy?: string;
    ctx: EditorContext;
  }

  let { note, version, state, resolvedBy, ctx }: Props = $props();
</script>

<div class="hx-note__head" contenteditable="false" spellcheck="false">
  <span class="hx-note__id">{note}</span>
  {#if version !== undefined}<LinkChip id={version} {ctx} />{/if}
  <span class="hx-note__state">
    <span class="hx-note__label">{state === 'resolved' ? 'resolved by' : state}</span>
    {#if state === 'resolved' && resolvedBy !== undefined}<LinkChip id={resolvedBy} {ctx} />{/if}
  </span>
</div>
