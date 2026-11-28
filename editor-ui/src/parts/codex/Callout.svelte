<!-- A callout that is not a voice block (P11 §5.8): a design remark, a note with its state, or a
     callout of another type, each with its label and its blocks. Plain props, so a CodeMirror
     widget can mount it. -->
<script lang="ts">
  import type { RecordTarget } from '../../lib/record-key';
  import type { ViewInline, ViewNode } from '../../lib/ipc/bindings';
  import Body from './Body.svelte';
  import Inline from './Inline.svelte';

  interface Props {
    /** The callout's type in lower case, such as `design` or `note`. */
    name: string;
    /** The title after the marker. */
    title?: readonly ViewInline[];
    /** A note's state as written. */
    state?: string | null;
    /** The blocks. */
    nodes: readonly ViewNode[];
    /** Opens a chip's record. */
    onopen?: ((record: RecordTarget) => void) | undefined;
  }

  let { name, title = [], state = null, nodes, onopen }: Props = $props();
  const label = $derived(name === 'design' ? 'Design note' : name);
</script>

<aside class="callout" data-callout={name}>
  <div class="head">
    <span class="tu-label tu-label--xs">{label}</span>
    {#if title.length > 0}<span class="title"><Inline inlines={title} {onopen} /></span>{/if}
    {#if state !== null}<span class="tu-status" data-status={state}>{state}</span>{/if}
  </div>
  <div class="tu-note"><Body {nodes} {onopen} /></div>
</aside>

<style>
  .callout {
    display: flex;
    flex-direction: column;
    gap: var(--sp-6);
    padding: var(--sp-8) var(--sp-12);
    border: 1px dashed var(--border);
    background: var(--panel);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--sp-8);
  }

  .title {
    color: var(--text);
  }

  .tu-note > :global(*) {
    margin: 0;
  }
</style>
