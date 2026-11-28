<!-- A clickable `tu-prop` row: the kind glyph, the value and its type, such as a record that links
     to the open one. A click opens it. -->
<script lang="ts">
  import KindGlyph from './KindGlyph.svelte';

  interface Props {
    /** The kind, for the glyph; `null` draws the plain record glyph. */
    kind: string | null;
    /** The value, such as the source record's id and title. */
    value: string;
    /** The type, such as the link's slot or the relations. */
    type: string;
    /** Called on a click. */
    onclick: () => void;
  }

  let { kind, value, type, onclick }: Props = $props();
</script>

<button class="tu-prop" type="button" {onclick}>
  <KindGlyph kind={kind ?? 'record'} />
  <span class="tu-prop__val">{value}</span>
  <span class="tu-prop__type">{type}</span>
</button>

<style>
  .tu-prop {
    width: 100%;
    border-left: 0;
    border-right: 0;
    border-bottom: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .tu-prop:first-child {
    border-top: 0;
  }
</style>
