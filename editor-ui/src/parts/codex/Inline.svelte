<!-- The inlines of a block (P11 §5.8): text, emphasis, strong, code, breaks, link chips, and
     Markdown links and images, which show as text with their URL: never a live link, never
     fetched (G12). Record text is always text, never HTML. -->
<script lang="ts">
  import type { ViewInline } from '../../lib/ipc/bindings';
  import Inline from './Inline.svelte';
  import LinkChip from './LinkChip.svelte';

  interface Props {
    /** The inlines. */
    inlines: readonly ViewInline[];
    /** Opens a chip's record. */
    onopen?: ((id: string) => void) | undefined;
  }

  let { inlines, onopen }: Props = $props();
</script>

{#each inlines as inline, index (index)}
  {#if inline.type === 'text'}{inline.text}{:else if inline.type === 'emphasis'}<em
      ><Inline inlines={inline.inlines} {onopen} /></em
    >{:else if inline.type === 'strong'}<strong><Inline inlines={inline.inlines} {onopen} /></strong
    >{:else if inline.type === 'code'}<code>{inline.text}</code>{:else if inline.type === 'break'}<br
    />{:else if inline.type === 'link'}<LinkChip
      id={inline.id}
      shown={inline.shown}
      target={inline.target}
      {onopen}
    />{:else if inline.type === 'external'}<span class="external"
      >{inline.text} <span class="url">({inline.url})</span></span
    >{/if}
{/each}

<style>
  .url {
    font-family: var(--font-mono);
    color: var(--textDim);
  }
</style>
