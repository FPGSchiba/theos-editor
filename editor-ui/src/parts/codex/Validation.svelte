<!-- Validation (P11 §5.8): the open record's findings, errors first, each with its severity
     glyph, message and rule (`tu-val`). P11 shows findings only; rules have no pass messages. -->
<script lang="ts">
  import Note from '../../components/Note.svelte';
  import Spoken from '../../components/Spoken.svelte';
  import type { Finding, Severity } from '../../lib/ipc/bindings';
  import { pageOf } from './pages.svelte';
  import { activeRecord } from './tabs.svelte';

  /** The `tu-val` severity of each finding severity, in the order they are listed. */
  const SEVERITIES: Record<Severity, string> = { error: 'error', warning: 'warn' };
  const ORDER = Object.keys(SEVERITIES);

  const page = $derived(pageOf(activeRecord()));
  const findings = $derived(
    [...(page?.findings ?? [])].sort((a, b) => rank(a) - rank(b)),
  );

  /** Where `finding`'s severity sorts. */
  function rank(finding: Finding): number {
    return ORDER.indexOf(finding.severity);
  }
</script>

<div class="tu-window__body">
  {#if page === null}
    <Note>The open record's findings.</Note>
  {:else}
    {#each findings as finding, index (index)}
      <div class="tu-val tu-instrument" data-sev={SEVERITIES[finding.severity]}>
        <span class="tu-val__sev"><Spoken>{finding.severity}</Spoken></span>
        <span class="tu-val__msg">{finding.message}</span>
        <div class="tu-val__meta">
          <span class="tu-val__rule">{finding.rule}</span>
          {#if finding.line !== null}<span>line {finding.line}</span>{/if}
        </div>
      </div>
    {:else}
      <Note>No findings on this record.</Note>
    {/each}
  {/if}
</div>

<style>
  .tu-val:first-child {
    border-top: 0;
  }
</style>
