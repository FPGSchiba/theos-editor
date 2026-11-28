<!-- The editor's root: asks the app for its version and OS, opens the project named on the command
     line or the last one, syncs the project each time the window gets the focus back, and draws
     the shell (P11 §5.9). -->
<script lang="ts">
  import { onMount } from 'svelte';

  import { failureText } from './lib/ipc/client';
  import { onWindowFocus } from './lib/ipc/focus';
  import { loadApp } from './lib/state/app.svelte';
  import { openInitialProject, project, reloadProject } from './lib/state/project.svelte';
  import Shell from './shell/Shell.svelte';

  /** Loads the app's info and the first project; neither rejects, both keep their failure. */
  async function start(): Promise<void> {
    const [appFailure] = await Promise.all([loadApp(), openInitialProject()]);
    if (appFailure !== null) {
      project.error ??= appFailure;
    }
  }

  /** Syncs the open project when the window gets the focus back; the app limits it to once a second. */
  function onFocus(): void {
    if (!project.opening) {
      void reloadProject('focus');
    }
  }

  onMount(() => {
    void start();
    const listening = onWindowFocus(onFocus).catch((failure: unknown) => {
      project.error ??= `The editor cannot follow the window's focus: ${failureText(failure)}`;
      return null;
    });
    return () => {
      void listening.then((stop) => stop?.());
    };
  });
</script>

<Shell />

<style>
  :global(html, body, #app) {
    height: 100%;
  }
</style>
