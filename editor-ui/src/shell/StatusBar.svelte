<!-- The status bar (P11 §5.8, §5.9): THEOS//EDITOR, the version and the layout, the counts, a
     note when the index lives in memory, and the notice of what the editor did on its own (a tab
     closed because its record was deleted). The other slots of screen 01 come with their phases. -->
<script lang="ts">
  import { totals } from '../lib/counts';
  import type { ProjectView } from '../lib/ipc/bindings';

  interface Props {
    /** The app's version; empty until the app answered. */
    version: string;
    /** The open project, or `null`. */
    view: ProjectView | null;
    /** The label of the active preset. */
    preset: string | null;
    /** The notice, or `null`. */
    notice?: string | null;
  }

  let { version, view, preset, notice = null }: Props = $props();
  const where = $derived(
    [version === '' ? null : `v${version}`, preset === null ? null : `${preset} layout`]
      .filter((part) => part !== null)
      .join(' · '),
  );
</script>

<footer class="tu-statusbar">
  <span class="tu-statusbar__brand">THEOS//EDITOR</span>
  {#if where !== ''}<span>{where}</span>{/if}
  {#if view === null}
    <span>no project</span>
  {:else}
    <span>{totals(view)}</span>
    {#if view.in_memory}
      <span class="held">index in memory: another program holds the cache</span>
    {/if}
  {/if}
  {#if notice !== null}<span class="notice" role="status">{notice}</span>{/if}
</footer>

<style>
  .held,
  .notice {
    color: var(--sig-warn);
  }
</style>
