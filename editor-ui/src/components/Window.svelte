<!-- A window of the dock (`tu-window`): its tab bar, and its component as the body. -->
<script lang="ts">
  import { tabText, type WindowSpec } from '../parts/layout';
  import TabBar from './TabBar.svelte';

  interface Props {
    /** What the window holds. */
    spec: WindowSpec;
  }

  let { spec }: Props = $props();
  let own = $state(0);
  const active = $derived(spec.selected?.index ?? own);
  const Body = $derived(spec.component);
  const shown = $derived(spec.tabs[Math.max(active, 0)]);
  const label = $derived(shown === undefined ? spec.id : tabText(shown));

  /** Makes the tab at `index` active, in the window or in the state that holds its tab. */
  function select(index: number): void {
    if (spec.selected === undefined) {
      own = index;
    } else {
      spec.selected.select(index);
    }
  }
</script>

<section class="tu-window" aria-label={label} data-window={spec.id}>
  <TabBar tabs={spec.tabs} {active} onselect={select} />
  <Body {...spec.props ?? {}} />
</section>
