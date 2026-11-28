<!-- The title bar (P11 §5.8): THEOS//EDITOR ▸ project ▸ open record ▸ layout, and the validator
     pip. The git pip comes with P46. -->
<script lang="ts">
  import Pip from '../components/Pip.svelte';
  import { validator } from '../lib/counts';
  import type { ProjectView } from '../lib/ipc/bindings';

  interface Props {
    /** The open project, or `null`. */
    view: ProjectView | null;
    /** The label of the active preset. */
    preset: string | null;
    /** The path of the open record without `.md`, when one is open. */
    path?: string | null;
  }

  let { view, preset, path = null }: Props = $props();
  const pip = $derived(view === null ? null : validator(view));
</script>

<header class="tu-titlebar">
  <span class="tu-titlebar__brand"><span class="tu-mark"></span>THEOS//EDITOR</span>
  {#if view !== null}
    <span class="tu-titlebar__sep">▸</span><span>{view.name}</span>
    {#if path !== null}
      <span class="tu-titlebar__sep">▸</span><span>{path}</span>
    {/if}
    {#if preset !== null}
      <span class="tu-titlebar__sep">▸</span><span class="tu-titlebar__preset">layout · {preset}</span>
    {/if}
  {/if}
  <span class="tu-spacer"></span>
  {#if pip !== null}
    <Pip signal={pip.signal} label={pip.text} />
  {/if}
</header>
