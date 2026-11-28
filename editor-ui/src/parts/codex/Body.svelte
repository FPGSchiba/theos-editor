<!-- A record body's blocks (P11 §5.8), drawn from the block tree Rust builds: headings,
     paragraphs, lists, quotes, code, rules, voice blocks, callouts, beats, and raw source as
     plain text. Nothing is HTML. -->
<script lang="ts">
  import type { ViewNode } from '../../lib/ipc/bindings';
  import BeatRow from './BeatRow.svelte';
  import Body from './Body.svelte';
  import Callout from './Callout.svelte';
  import Inline from './Inline.svelte';
  import VoiceBlock from './VoiceBlock.svelte';

  interface Props {
    /** The blocks. */
    nodes: readonly ViewNode[];
    /** Opens a chip's record. */
    onopen?: ((id: string) => void) | undefined;
  }

  let { nodes, onopen }: Props = $props();

  /** The heading element of `level`, kept to h1–h6. */
  const heading = (level: number): string => `h${Math.min(Math.max(level, 1), 6)}`;
</script>

{#each nodes as node, index (index)}
  {#if node.type === 'heading'}
    <svelte:element this={heading(node.level)}
      ><Inline inlines={node.inlines} {onopen} /></svelte:element
    >
  {:else if node.type === 'paragraph'}
    <p><Inline inlines={node.inlines} {onopen} /></p>
  {:else if node.type === 'list'}
    <svelte:element this={node.ordered ? 'ol' : 'ul'}>
      {#each node.items as item, at (at)}
        <li><Body nodes={item} {onopen} /></li>
      {/each}
    </svelte:element>
  {:else if node.type === 'quote'}
    <blockquote><Body nodes={node.nodes} {onopen} /></blockquote>
  {:else if node.type === 'code'}
    <pre class="tu-code">{node.text}</pre>
  {:else if node.type === 'rule'}
    <hr />
  {:else if node.type === 'voice'}
    <VoiceBlock
      speaker={node.speaker}
      beliefs={node.beliefs}
      tier={node.tier}
      nodes={node.nodes}
      caption={node.caption}
      {onopen}
    />
  {:else if node.type === 'design'}
    <Callout name="design" nodes={node.nodes} {onopen} />
  {:else if node.type === 'note'}
    <Callout name="note" title={node.title} state={node.state} nodes={node.nodes} {onopen} />
  {:else if node.type === 'callout'}
    <Callout name={node.name} title={node.title} nodes={node.nodes} {onopen} />
  {:else if node.type === 'beat'}
    <section class="beat">
      <h3><Inline inlines={node.heading} {onopen} /></h3>
      {#if node.attributes.length > 0}
        <div class="tu-beat">
          {#each node.attributes as row, at (at)}
            <BeatRow key={row.key} value={row.value} {onopen} />
          {/each}
        </div>
      {/if}
      <Body nodes={node.nodes} {onopen} />
    </section>
  {:else if node.type === 'raw'}
    <pre class="raw">{node.text}</pre>
  {/if}
{/each}

<style>
  li > :global(p) {
    margin: 0;
  }

  /* Margins without specificity, so tu-prose spaces the blocks. */
  :where(blockquote) {
    margin: 0;
  }

  blockquote {
    padding-left: var(--sp-12);
    border-left: 1px solid var(--borderHi);
  }

  hr {
    border: 0;
    border-top: 1px solid var(--border);
  }

  :where(.raw) {
    margin: 0;
  }

  .raw {
    white-space: pre-wrap;
    font-family: var(--font-mono);
    color: var(--textDim);
  }

  .beat {
    display: flex;
    flex-direction: column;
    gap: var(--sp-8);
  }

  .beat > h3 {
    margin: 0;
  }
</style>
