<!-- An in-world voice block (`tu-voice`, Theos UI VoiceBlock): who speaks, the beliefs voiced and
     the tier in the head, the speech, and the caption. Plain props, so a CodeMirror widget can
     mount it. -->
<script lang="ts">
  import type { ViewInline, ViewLink, ViewNode } from '../../lib/ipc/bindings';
  import type { RecordTarget } from '../../lib/record-key';
  import Body from './Body.svelte';
  import Inline from './Inline.svelte';

  interface Props {
    /** Who speaks. */
    speaker: ViewLink;
    /** The beliefs voiced. */
    beliefs?: readonly ViewLink[];
    /** The tier, when the title names one. */
    tier?: string | null;
    /** The speech. */
    nodes: readonly ViewNode[];
    /** The caption line, if any. */
    caption?: readonly ViewInline[] | null;
    /** Opens a record named in the speech. */
    onopen?: ((record: RecordTarget) => void) | undefined;
  }

  let { speaker, beliefs = [], tier = null, nodes, caption = null, onopen }: Props = $props();
  const who = $derived(speaker.shown ?? speaker.target?.title ?? speaker.id);
  const right = $derived(
    [beliefs.map((belief) => belief.id).join(', '), tier === null ? '' : `${tier} lens`]
      .filter((part) => part !== '')
      .join(' · '),
  );
</script>

<figure class="tu-voice">
  <div class="tu-voice__head">
    <span class="tu-kind" data-kind="perspective" aria-hidden="true"></span>In-world · {who}
    {#if right !== ''}<span class="tu-voice__head-right">{right}</span>{/if}
  </div>
  <blockquote class="tu-voice__body"><Body {nodes} {onopen} /></blockquote>
  {#if caption !== null}
    <figcaption class="tu-voice__by"><Inline inlines={caption} {onopen} /></figcaption>
  {/if}
</figure>

<style>
  /* No specificity, so a surrounding tu-prose spaces the block like its other blocks. */
  :where(figure) {
    margin: 0;
  }

  blockquote {
    margin: 0;
  }
</style>
