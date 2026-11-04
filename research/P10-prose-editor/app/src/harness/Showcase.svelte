<script lang="ts">
  /**
   * `/?view=components`: the five shared components on sample data, drawn the way an adapter
   * has to draw its blocks. It is the reference markup for the DOM contract (context.md) and
   * what `tests/e2e/components.spec.ts` checks the contract against.
   */
  import BeatRow from '../components/BeatRow.svelte';
  import LinkChip from '../components/LinkChip.svelte';
  import LinkPopover from '../components/LinkPopover.svelte';
  import NoteHead from '../components/NoteHead.svelte';
  import VoiceHead from '../components/VoiceHead.svelte';
  import { createMockContext } from './mock-index.ts';
  import type { BeatKey } from './types.ts';

  const ctx = createMockContext({ fileId: 'records/story/scripts/S-004.md' });
  const place = { in: 'prose' } as const;

  let popoverOpen = $state(false);
  let picked = $state('');
  let closed = $state(0);
  let commits = $state<string[]>([]);
  let beatOne = $state([
    { key: 'at', value: '[[fates-shrine]]' },
    { key: 'with', value: '[[mother-ilse]]' },
    { key: 'says', value: '[[B-016]]' },
    { key: 'when', value: 'T1' },
  ]);
  const beatTwo = [
    { key: 'at', value: '[[fates-shrine]]' },
    { key: 'reveals', value: '[[C-031]]' },
    { key: 'when', value: 'T1' },
  ];

  function commit(key: BeatKey, value: string): void {
    commits = [...commits, `${key}:: ${value}`];
    const found = beatOne.find((param) => param.key === key);
    beatOne = found === undefined
      ? [...beatOne, { key, value }]
      : beatOne.map((param) => (param.key === key ? { key, value } : param));
  }
</script>

<div class="hx-showcase tu-manuscript">
  <h1>Shared components</h1>

  <section data-showcase="chips" class="tu-prose">
    <h2 class="hx-showcase__title">Link chips</h2>
    <p>
      A resolved npc <LinkChip id="mother-ilse" {ctx} />, a code id <LinkChip id="T-012" {ctx} />,
      an alias <LinkChip id="fates-shrine" shown="Fates' shrine" {ctx} />, a broken link
      <LinkChip id="song-cellar" {ctx} />.
    </p>
  </section>

  <section data-showcase="popover">
    <h2 class="hx-showcase__title">The <code>[[</code> popover</h2>
    <!-- Mounted only while open, as an adapter does: an open popover owns Enter and Escape. -->
    {#if popoverOpen}
      <p class="tu-typing">
        The binders call them ancestors, but the operator-plate names
        <span class="tu-typing__q">[[Listen</span><span class="tu-typing__caret"></span>
      </p>
      <LinkPopover
        options={ctx.search('Listen', place)}
        query="Listen"
        {place}
        onpick={(id) => { picked = id; popoverOpen = false; }}
        onclose={() => { closed += 1; popoverOpen = false; }}
      />
    {:else}
      <button type="button" class="tu-btn tu-btn--sm" data-showcase-open-popover onclick={() => (popoverOpen = true)}>Open popover</button>
    {/if}
    <p class="tu-dim">picked: <span data-showcase-picked>{picked}</span> · closed: <span data-showcase-closed>{closed}</span></p>
  </section>

  <section data-showcase="callouts" class="tu-prose">
    <h2 class="hx-showcase__title">Callouts</h2>
    <figure class="tu-voice" data-callout="voice">
      <VoiceHead speaker="throat-kin" beliefs={['B-020']} tier="T2" {ctx} />
      <blockquote class="tu-voice__body">
        <p>
          Sing a name the way it was first sung and they lean close to hear it.
          Never <span data-dev-word title="developer word">confirm</span> it, for the <span data-dev-word title="developer word">tier</span> is a ledger.
        </p>
      </blockquote>
      <figcaption class="tu-voice__by">a binder, teaching the naming-song</figcaption>
    </figure>

    <div class="hx-callout" data-callout="design">
      <p>A designer remark that is not in-world.</p>
    </div>
    <p class="tu-note">Design note: the player first meets this belief at the <LinkChip id="fates-shrine" shown="Fates' shrine" {ctx} /> on F4.</p>

    <div class="hx-callout hx-note" data-callout="note" data-note="n1" data-note-state="open">
      <NoteHead note="n1" version="f04-terrain-v2" state="open" {ctx} />
      <p>Too much relief for the Hearth; keep the floor walkable.</p>
    </div>
    <div class="hx-callout hx-note" data-callout="note" data-note="n4" data-note-state="resolved">
      <NoteHead note="n4" state="resolved" resolvedBy="fates-shrine-v5" {ctx} />
      <p>The roof reads as a church.</p>
    </div>
  </section>

  <section data-showcase="beats">
    <h2 class="hx-showcase__title">Beats</h2>
    <BeatRow beat={1} name="Arrival" params={beatOne} {ctx} oncommit={commit} />
    <BeatRow beat={2} name="The murmur" params={beatTwo} {ctx} />
    <p class="tu-dim">commits: <span data-showcase-commits>{commits.join(' | ')}</span></p>
  </section>
</div>
