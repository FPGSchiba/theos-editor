<!-- A status pip with its label (`tu-statuspip`): a pip never stands alone (Theos UI). -->
<script lang="ts">
  import type { Signal } from '../lib/counts';

  interface Props {
    /** The signal, as `is-<signal>`. */
    signal: Signal;
    /** The label beside the pip. */
    label: string;
    /** Whether the pip glows. */
    glow?: boolean;
  }

  let { signal, label, glow = false }: Props = $props();
</script>

<span class="tu-statuspip"
  ><span class="tu-pip is-{signal}" class:tu-pip--glow={glow} aria-hidden="true"></span>{label}</span
>
