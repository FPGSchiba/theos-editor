<!-- The toolbar (P11 §5.8): the find field, Read / Write / Source, Open in graph, New record and
     Save. What is not built yet is disabled and names its phase. -->
<script lang="ts">
  import Button from '../components/Button.svelte';
  import Seg from '../components/Seg.svelte';
  import { keyLabel, type Os } from '../lib/keymap';
  import Find from './Find.svelte';

  interface Props {
    /** The OS, which picks the key label of Save. */
    os: Os;
  }

  let { os }: Props = $props();

  const modes = [
    { label: 'Read', active: true },
    { label: 'Write', disabled: true, title: 'Write mode arrives with P12' },
    { label: 'Source', disabled: true, title: 'Source mode arrives with P12' },
  ];
</script>

<div class="tu-toolbar">
  <Find />
  <span class="tu-divider"></span>
  <Seg label="Editor mode" options={modes} />
  <span class="tu-divider"></span>
  <Button variant="ghost" icon="⋈" disabled title="Open in graph arrives with P15"
    >Open in graph</Button
  >
  <span class="tu-spacer"></span>
  <Button variant="secondary" icon="+" disabled title="New record arrives with P12"
    >New record</Button
  >
  <Button variant="accent" disabled title="Saving arrives with P12"
    >Save {keyLabel('file.save', os)}</Button
  >
</div>
