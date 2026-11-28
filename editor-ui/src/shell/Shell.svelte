<!-- The editor's shell (P11 §5.6, screen 01): title bar, toolbar, rail, the work area with the
     active preset's dock tree, the status bar, the command palette, and the one key listener. -->
<script lang="ts">
  import { pageOf } from '../parts/codex/pages.svelte';
  import { activeRecord } from '../parts/codex/tabs.svelte';
  import { presetFor } from '../parts/presets';
  import { runCommand } from '../lib/commands';
  import { match } from '../lib/keymap';
  import { activeEntry } from '../lib/rail';
  import { app } from '../lib/state/app.svelte';
  import { palette } from '../lib/state/palette.svelte';
  import { notice } from '../lib/state/notice.svelte';
  import { activate, CODEX, dismissError, project } from '../lib/state/project.svelte';
  import Dock from './Dock.svelte';
  import EmptyWork from './EmptyWork.svelte';
  import ErrorBanner from './ErrorBanner.svelte';
  import Palette from './Palette.svelte';
  import Rail from './Rail.svelte';
  import StatusBar from './StatusBar.svelte';
  import TitleBar from './TitleBar.svelte';
  import Toolbar from './Toolbar.svelte';

  const entry = $derived(
    project.view === null ? null : activeEntry(project.view.rail, project.active),
  );
  const preset = $derived(entry === null ? null : presetFor(entry));
  /** The path of the Codex's open record without `.md`, for the title bar. */
  const recordPath = $derived(
    preset?.id === CODEX
      ? (pageOf(activeRecord())?.header.record.path?.replace(/\.md$/, '') ?? null)
      : null,
  );

  /**
   * The editor's one key listener (P11 §5.11): a press that is a command's keys runs it, and the
   * webview's own use of those keys (reload, close, print) never happens, even while the command
   * is disabled. A held key (`repeat`) runs nothing. While the palette is open it is modal: only
   * the palette's own keys toggle it.
   */
  function onkeydown(event: KeyboardEvent): void {
    if (event.isComposing) {
      return;
    }
    const command = match(event, app.os);
    if (command === null) {
      return;
    }
    event.preventDefault();
    if (event.repeat || (palette.open && command !== 'palette.open')) {
      return;
    }
    runCommand(command);
  }
</script>

<svelte:window {onkeydown} />

<div class="tu-shell">
  <TitleBar view={project.view} preset={entry?.label ?? null} path={recordPath} />
  <Toolbar os={app.os} />
  {#if project.view === null}
    <div class="no-rail"></div>
  {:else}
    <Rail entries={project.view.rail} active={entry?.id ?? project.active} onselect={activate} />
  {/if}
  <main class="tu-shell__work" class:stacked={project.view !== null && project.error !== null}>
    {#if project.view === null || preset === null}
      {#if !project.opening}
        <EmptyWork os={app.os} error={project.error} onopen={() => runCommand('project.open')} />
      {/if}
    {:else}
      {#if project.error !== null}
        <ErrorBanner text={project.error} ondismiss={dismissError} />
      {/if}
      {#key preset.id}
        <Dock split={preset.windows} />
      {/key}
    {/if}
  </main>
  <StatusBar
    version={app.version}
    view={project.view}
    preset={entry?.label ?? null}
    notice={notice.text}
  />
</div>
{#if palette.open}
  <Palette os={app.os} />
{/if}

<style>
  .tu-shell {
    height: 100%;
  }

  .stacked {
    flex-direction: column;
  }

  .no-rail {
    width: var(--w-rail);
    background: var(--bg);
    border-right: 1px solid var(--border);
  }
</style>
