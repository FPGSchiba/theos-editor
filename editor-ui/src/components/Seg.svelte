<!-- A segmented control (`tu-seg`): options of which one is active. -->
<script lang="ts">
  /** One option. A disabled one says in `title` which phase brings it. */
  export interface SegOption {
    label: string;
    active?: boolean;
    disabled?: boolean;
    title?: string;
  }

  interface Props {
    /** What the options choose, for assistive technology. */
    label: string;
    options: readonly SegOption[];
    /** Called with the label of the option the user picks. */
    onselect?: (label: string) => void;
  }

  let { label, options, onselect }: Props = $props();
</script>

<span class="tu-seg" role="group" aria-label={label}>
  {#each options as option (option.label)}
    <button
      class="tu-seg__opt"
      class:is-active={option.active === true}
      type="button"
      aria-pressed={option.active === true}
      disabled={option.disabled === true}
      title={option.title}
      onclick={() => onselect?.(option.label)}>{option.label}</button
    >
  {/each}
</span>

<style>
  button.tu-seg__opt:disabled {
    color: var(--textMute);
  }
</style>
