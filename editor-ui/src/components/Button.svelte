<!-- A button (`tu-btn`). A disabled one says in its title which phase brings it. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** The look: `secondary`, `ghost`, `accent`, `primary`, …, as `tu-btn--<variant>`. */
    variant?: 'primary' | 'secondary' | 'ghost' | 'accent' | 'dashed';
    /** The glyph before the label. */
    icon?: string;
    /** Whether the button is disabled. */
    disabled?: boolean;
    /** The tooltip; for a disabled button, "… arrives with PNN". */
    title?: string;
    /** Called on a click. */
    onclick?: () => void;
    /** The label. */
    children: Snippet;
  }

  let { variant = 'secondary', icon, disabled = false, title, onclick, children }: Props =
    $props();
</script>

<button class="tu-btn tu-btn--{variant}" type="button" {disabled} {title} {onclick}>
  {#if icon !== undefined}<span class="tu-btn__icon" aria-hidden="true">{icon}</span>{/if}
  {@render children()}
</button>
