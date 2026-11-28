<!-- Text for a screen reader only: it takes no space on screen, where a glyph or colour says it. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** The text. */
    children: Snippet;
  }

  let { children }: Props = $props();
</script>

<span class="spoken">{@render children()}</span>

<style>
  .spoken {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
