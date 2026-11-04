<script lang="ts">
  /**
   * The head of a voice block (DOM contract): `.tu-voice__head` with the speaker's title, the
   * beliefs and the tier. The adapter owns the block around it: `figure.tu-voice[data-callout="voice"]`,
   * the editable body and the `.tu-voice__by` caption (see the Showcase for the markup).
   */
  import type { EditorContext } from '../harness/types.ts';

  interface Props {
    /** The id of the speaker (people, faction or npc). */
    speaker: string;
    /** Belief ids, as written in the head line. */
    beliefs?: string[];
    /** The tier, e.g. `T1`. */
    tier?: string;
    ctx: EditorContext;
  }

  let { speaker, beliefs = [], tier, ctx }: Props = $props();

  const title = $derived(ctx.resolve(speaker)?.title ?? speaker);
  const right = $derived([...beliefs, ...(tier === undefined ? [] : [`${tier} lens`])].join(' · '));
</script>

<div class="tu-voice__head" contenteditable="false" spellcheck="false">
  <span class="tu-kind" data-kind="perspective"></span>
  <span class="tu-voice__speaker">In-world · {title}</span>
  {#if right !== ''}<span class="tu-voice__head-right">{right}</span>{/if}
</div>
