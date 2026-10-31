<script lang="ts">
  /**
   * A link chip (DOM contract): `.tu-link[data-link-id]`, `.is-broken` when the id does not
   * resolve, `spellcheck="false"`, with `.tu-kind[data-kind]` and `.tu-link__title`.
   * `shown` is the alias of `[[id|shown]]`; without it the chip shows the record's title.
   */
  import type { EditorContext } from '../harness/types.ts';

  interface Props {
    id: string;
    shown?: string;
    ctx: EditorContext;
  }

  let { id, shown, ctx }: Props = $props();

  const info = $derived(ctx.resolve(id));
  const title = $derived(shown ?? info?.title ?? id);
  // Records with a code id (T-012, C-031) show it before the title, as the reference screens do.
  const showId = $derived(info !== undefined && /^[A-Z]{1,3}-\d+$/.test(id));
</script>

<span
  class="tu-link"
  class:is-broken={info === undefined}
  data-link-id={id}
  data-kind={info?.kind}
  contenteditable="false"
  spellcheck="false"
>
  <span class="tu-kind" data-kind={info?.kind}></span>
  {#if showId}<span class="tu-link__id">{id}</span>{/if}
  <span class="tu-link__title">{title}</span>
</span>
