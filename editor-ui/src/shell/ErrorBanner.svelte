<!-- The error banner of the work area (P11 §5.9): why the last attempt to open or sync a project
     failed, while the open project stays open (`tu-val`). -->
<script lang="ts">
  import Button from '../components/Button.svelte';

  interface Props {
    /** The error's text. */
    text: string;
    /** Called when the user dismisses the banner. */
    ondismiss: () => void;
  }

  let { text, ondismiss }: Props = $props();
</script>

<div class="tu-val" data-sev="error" role="alert">
  <span class="tu-val__sev" aria-hidden="true"></span>
  <span class="tu-val__msg">{text}</span>
  <span class="tu-val__act"><Button variant="ghost" onclick={ondismiss}>Dismiss</Button></span>
</div>

