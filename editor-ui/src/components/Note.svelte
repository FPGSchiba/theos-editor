<!-- A line of a window that has nothing else to show: what goes there, or why it failed. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Whether the line reports a failure: red, and announced at once. */
    alert?: boolean;
    /** The text. */
    children: Snippet;
  }

  let { alert = false, children }: Props = $props();
</script>

<p class="note inst-body" class:alert role={alert ? 'alert' : undefined}>{@render children()}</p>

<style>
  .note {
    margin: 0;
    padding: var(--sp-8) var(--sp-10);
    color: var(--textDim);
  }

  .alert {
    color: var(--sig-reject);
  }
</style>
