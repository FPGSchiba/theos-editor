<!-- The glyph of a record's kind (`tu-kind`); the glyph and its colour come from Theos UI's CSS. -->
<script lang="ts">
  interface Props {
    /** The record's kind, such as `floor`. */
    kind: string;
  }

  let { kind }: Props = $props();
</script>

<span class="tu-kind" data-kind={kind} aria-hidden="true"></span>
