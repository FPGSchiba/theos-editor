<!-- The body of a window that is still empty (Theos UI "Empty states"): a mark, what will appear
     here and what to do, and the phase that brings it. -->
<script lang="ts">
  interface Props {
    /** The glyph in the dashed frame. */
    mark: string;
    /** What will appear here, and what to do. */
    text: string;
    /** The phase that brings the window, such as `P13`. */
    phase?: string;
  }

  let { mark, text, phase }: Props = $props();
</script>

<div class="tu-window__body">
  <div class="tu-empty">
    <span class="tu-empty__mark" aria-hidden="true">{mark}</span>
    <p class="inst-body">{text}</p>
    {#if phase !== undefined}
      <p class="tu-label tu-label--xs">Arrives with {phase}.</p>
    {/if}
  </div>
</div>

<style>
  p {
    margin: 0;
    max-width: 44ch;
  }
</style>
