<!-- One attribute line of a beat (`tu-beat__param`, Theos UI BeatParams): its key and its value.
     Plain props, so a CodeMirror widget can mount it. -->
<script lang="ts">
  import type { RecordTarget } from '../../lib/record-key';
  import type { ViewInline } from '../../lib/ipc/bindings';
  import Inline from './Inline.svelte';

  interface Props {
    /** The key before `::`. */
    key: string;
    /** The value. */
    value: readonly ViewInline[];
    /** Opens a chip's record. */
    onopen?: ((record: RecordTarget) => void) | undefined;
  }

  let { key, value, onopen }: Props = $props();
</script>

<span class="tu-beat__param" class:is-empty={value.length === 0}>
  <span class="tu-beat__key">{key}</span>
  <span class="tu-beat__val"
    >{#if value.length === 0}—{:else}<Inline inlines={value} {onopen} />{/if}</span
  >
</span>
