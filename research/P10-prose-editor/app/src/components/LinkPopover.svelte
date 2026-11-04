<script lang="ts">
  /**
   * The `[[` popover (DOM contract): `.tu-pop[data-link-popover]` with rows
   * `.tu-pop__row[data-link-option=<id>][data-kind=<kind>]`. Enter or a click picks, Escape
   * closes. While it is mounted it owns ArrowUp, ArrowDown, Enter and Escape (in the capture
   * phase, so the editor never sees them); the editor keeps the focus and tells the popover
   * about typing through `update`. Mount it with `mount`, drop it with `unmount`.
   */
  import type { LinkInfo, LinkPlace } from '../harness/types.ts';

  interface Props {
    options: LinkInfo[];
    query?: string;
    place: LinkPlace;
    /** Called with the picked record's id. */
    onpick: (id: string) => void;
    /** Called when Escape is pressed; the editor unmounts the popover and keeps the typed text. */
    onclose?: () => void;
  }

  let { options: initialOptions, query: initialQuery = '', place, onpick, onclose }: Props = $props();

  // The props only seed the state; later changes come in through `update`.
  // svelte-ignore state_referenced_locally
  let options = $state(initialOptions);
  // svelte-ignore state_referenced_locally
  let query = $state(initialQuery);
  let active = $state(0);

  /** New options for a new query; the first row becomes the active one. */
  export function update(next: { options: LinkInfo[]; query: string }): void {
    options = next.options;
    query = next.query;
    active = 0;
  }

  export function move(delta: number): void {
    if (options.length === 0) return;
    active = (active + delta + options.length) % options.length;
  }

  /** The id of the active row, if there is one. */
  export function activeId(): string | undefined {
    return options[active]?.id;
  }

  export function pickActive(): void {
    const id = activeId();
    if (id !== undefined) onpick(id);
  }

  const scope = $derived(place.in === 'prose' ? 'all kinds' : `${place.key}:: only`);

  /** The title cut around the first match of the query, so the match can be marked. */
  function parts(title: string): [string, string, string] {
    const needle = query.trim().toLowerCase();
    const at = needle === '' ? -1 : title.toLowerCase().indexOf(needle);
    if (at === -1) return [title, '', ''];
    return [title.slice(0, at), title.slice(at, at + needle.length), title.slice(at + needle.length)];
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onclose?.();
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      event.stopPropagation();
      move(event.key === 'ArrowDown' ? 1 : -1);
    } else if (event.key === 'Enter' && options.length > 0) {
      event.preventDefault();
      event.stopPropagation();
      pickActive();
    }
  }
</script>

<svelte:document onkeydowncapture={onkeydown} />

<!-- mousedown is cancelled so a click on a row never takes the focus from the editor -->
<div
  class="tu-pop"
  data-link-popover
  role="listbox"
  aria-label="Link to record"
  tabindex="-1"
  spellcheck="false"
  onmousedown={(event) => event.preventDefault()}
>
  <div class="tu-pop__head">
    <span>[[ link</span><span class="tu-spacer"></span><span>{options.length} matches · {scope}</span>
  </div>
  {#each options as option, index (option.id)}
    {@const [before, match, after] = parts(option.title)}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="tu-pop__row"
      class:is-active={index === active}
      role="option"
      tabindex="-1"
      aria-selected={index === active}
      data-link-option={option.id}
      data-kind={option.kind}
      onclick={() => onpick(option.id)}
    >
      <span class="tu-kind" data-kind={option.kind}></span>
      <span class="tu-pop__id">{option.id}</span>
      <span class="tu-pop__title">{before}{#if match}<mark>{match}</mark>{/if}{after}</span>
      <span class="tu-pop__kind">{option.kind}</span>
    </div>
  {/each}
  <div class="tu-pop__foot"><span>↑↓ move</span><span>⏎ insert</span><span>esc close</span></div>
</div>
