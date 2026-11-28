<!-- The work area without a project (P11 §5.9): how to open one, by button or keys, and why the
     last attempt failed. -->
<script lang="ts">
  import Button from '../components/Button.svelte';
  import { keyLabel, type Os } from '../lib/keymap';

  interface Props {
    /** The OS, which picks Ctrl or ⌘. */
    os: Os;
    /** Why the last project did not open, or `null`. */
    error: string | null;
    /** Opens a project (the command Open project). */
    onopen: () => void;
  }

  let { os, error, onopen }: Props = $props();
</script>

<div class="tu-window">
  <div class="tu-empty">
    <span class="tu-empty__mark" aria-hidden="true">◈</span>
    <p class="inst-heading">Open a content project: {keyLabel('project.open', os)}</p>
    <div class="tu-instrument">
      <Button variant="primary" onclick={onopen}>Open project…</Button>
    </div>
    <p class="inst-body">
      A content project is a folder with a theos.toml. Its records, the Codex and the other
      layouts appear here.
    </p>
    {#if error !== null}
      <p class="inst-body error" role="alert">{error}</p>
    {/if}
  </div>
</div>

<style>
  .tu-window {
    flex: 1;
  }

  p {
    margin: 0;
    max-width: 52ch;
  }

  .error {
    color: var(--sig-reject);
  }
</style>
