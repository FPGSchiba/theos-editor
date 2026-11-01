<script lang="ts">
  /**
   * A beat's heading and parameter row (DOM contract): `[data-beat=<n>]` showing `<n> · <name>`;
   * `[data-beat-row=<n>]` with `.tu-beat`, params `[data-beat-param=<key>]`, the add button
   * `[data-beat-add]` with its key choices `[data-beat-add-key=<key>]`, the value field
   * `[data-beat-input]` (Enter commits) and the findings of the beat
   * `[data-beat-finding][data-finding-rule=<rule>]`. The row has `spellcheck="false"`.
   *
   * The row only reports edits: `oncommit(key, value)` carries the raw value that belongs after
   * `key:: `, e.g. `[[echo-vault]]` or `T2`. The adapter turns it into a change of the text and
   * hands the row its new `params`.
   */
  import { BEAT_KEY_ORDER } from '../harness/types.ts';
  import type { BeatKey, EditorContext } from '../harness/types.ts';
  import LinkChip from './LinkChip.svelte';

  interface Props {
    /** The beat's number. */
    beat: number;
    /** The beat's name; the heading is drawn when it is given. */
    name?: string;
    /** The attribute lines of the beat in document order, values as written. */
    params: Array<{ key: string; value: string }>;
    ctx: EditorContext;
    oncommit?: (key: BeatKey, value: string) => void;
  }

  let { beat, name, params, ctx, oncommit }: Props = $props();

  const isBeatKey = (key: string): key is BeatKey => (BEAT_KEY_ORDER as readonly string[]).includes(key);

  const findings = $derived(ctx.findings.filter((finding) => 'beat' in finding.at && finding.at.beat === beat));
  const missing = $derived(BEAT_KEY_ORDER.filter((key) => !params.some((param) => param.key === key)));

  /** The param being edited, or being added (`isNew`); at most one at a time. */
  let editing = $state<{ key: BeatKey; draft: string; isNew: boolean } | undefined>();
  let choosing = $state(false);

  type Piece = { text: string } | { id: string; shown?: string };

  /** A value cut into plain text and `[[id]]` / `[[id|shown]]` links. */
  function pieces(value: string): Piece[] {
    const result: Piece[] = [];
    let from = 0;
    for (const match of value.matchAll(/\[\[([^\]|]+)(?:\|([^\]]*))?\]\]/g)) {
      if (match.index > from) result.push({ text: value.slice(from, match.index) });
      result.push(match[2] === undefined ? { id: match[1]! } : { id: match[1]!, shown: match[2] });
      from = match.index + match[0].length;
    }
    if (from < value.length) result.push({ text: value.slice(from) });
    return result;
  }

  function edit(key: string, value: string): void {
    if (isBeatKey(key)) editing = { key, draft: value, isNew: false };
  }

  function add(key: BeatKey): void {
    choosing = false;
    editing = { key, draft: '', isNew: true };
  }

  function commit(): void {
    if (editing === undefined) return;
    const { key, draft } = editing;
    editing = undefined;
    oncommit?.(key, draft);
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.isComposing) return;
    if (event.key === 'Enter') {
      event.preventDefault();
      commit();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      editing = undefined;
    }
  }

  /** Focus the field the moment it appears. */
  function focusNow(node: HTMLInputElement): void {
    node.focus();
    node.select();
  }
</script>

{#if name !== undefined}
  <h2 class="hx-beat-heading" data-beat={beat}>{beat} · {name}</h2>
{/if}
<div class="hx-beat-row" data-beat-row={beat} spellcheck="false">
  <div class="tu-beat">
    {#each params as param, index (`${index}:${param.key}`)}
      {#if index > 0}<span class="tu-beat__sep">·</span>{/if}
      {#if editing !== undefined && !editing.isNew && editing.key === param.key}
        <span class="tu-beat__param is-editing" data-beat-param={param.key}>
          <span class="tu-beat__key">{param.key}</span>
          <input
            class="hx-beat-input"
            data-beat-input
            spellcheck="false"
            autocomplete="off"
            bind:value={editing.draft}
            {onkeydown}
            onblur={() => (editing = undefined)}
            use:focusNow
          />
        </span>
      {:else}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <span class="tu-beat__param" data-beat-param={param.key} onclick={() => edit(param.key, param.value)}>
          <span class="tu-beat__key">{param.key}</span>
          <span class="tu-beat__val">
            {#each pieces(param.value) as piece}
              {#if 'id' in piece}
                <LinkChip id={piece.id} shown={piece.shown} {ctx} />
              {:else if /^T[1-3]$/.test(piece.text.trim())}
                <span class="tu-tiertag">{piece.text.trim()}</span>
              {:else}
                {piece.text}
              {/if}
            {/each}
          </span>
        </span>
      {/if}
    {/each}

    {#if editing !== undefined && editing.isNew}
      {#if params.length > 0}<span class="tu-beat__sep">·</span>{/if}
      <span class="tu-beat__param is-editing is-pending" data-beat-param={editing.key}>
        <span class="tu-beat__key">{editing.key}</span>
        <input
          class="hx-beat-input"
          data-beat-input
          spellcheck="false"
          autocomplete="off"
          bind:value={editing.draft}
          {onkeydown}
          onblur={() => (editing = undefined)}
          use:focusNow
        />
      </span>
    {/if}

    {#if missing.length > 0 && !(editing?.isNew ?? false)}
      <span class="tu-beat__sep">·</span>
      {#if choosing}
        <span class="hx-beat-keys">
          {#each missing as key (key)}
            <button type="button" class="tu-btn tu-btn--sm tu-btn--ghost" data-beat-add-key={key} onclick={() => add(key)}>{key}</button>
          {/each}
        </span>
      {:else}
        <button type="button" class="tu-beat__param is-empty hx-beat-add" data-beat-add onclick={() => (choosing = true)}>
          <span class="tu-beat__val">+ add</span>
        </button>
      {/if}
    {/if}
  </div>

  {#each findings as finding}
    <div class="hx-finding is-{finding.severity}" data-beat-finding data-finding-rule={finding.rule} data-severity={finding.severity}>
      <span class="tu-pip"></span>
      <span class="hx-finding__rule">{finding.rule}</span>
      <span class="hx-finding__message">{finding.message}</span>
    </div>
  {/each}
</div>
