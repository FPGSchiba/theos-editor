// The open project (P11 §5.6, §5.9): its view, the active rail entry, and the error of the last
// attempt to open or sync one. The active preset is state, not a URL. A project opens from the
// command line, else the last project, or from the folder dialog; it syncs on Reload and when the
// window gets the focus back.

import { closeAllRecords, refreshRecords } from '../../parts/codex/freshness';
import { pickFolder } from '../ipc/dialog';
import { call, failureText } from '../ipc/client';
import { commands, type ProjectView, type SyncReason } from '../ipc/bindings';
import { resetFind } from './find.svelte';
import { hush } from './notice.svelte';

/** The id of the Codex's rail entry. */
export const CODEX = 'codex';

/** The rail entry a project opens on. */
export const FIRST_ENTRY = CODEX;

/** The open project and what the shell shows of it. */
export interface ProjectState {
  /** The open project, or `null`. */
  view: ProjectView | null;
  /** The id of the active rail entry. */
  active: string;
  /**
   * Why the last attempt to open or sync a project failed, or `null`. Without a project the work
   * area says it; with one, a banner above the windows does, and the project stays open.
   */
  error: string | null;
  /** Whether a project is being opened. */
  opening: boolean;
}

/** The open project. */
export const project = $state<ProjectState>({
  view: null,
  active: FIRST_ENTRY,
  error: null,
  opening: true,
});

/** Opens the project in `path`. A failure keeps the open project and is kept in `error`. */
export async function openProject(path: string): Promise<void> {
  project.opening = true;
  try {
    const view = await call(commands.projectOpen(path));
    closeAllRecords();
    resetFind();
    hush();
    project.view = view;
    project.active = FIRST_ENTRY;
    project.error = null;
  } catch (failure) {
    project.error = failureText(failure);
  } finally {
    project.opening = false;
  }
}

/**
 * Opens the project the app starts with: the folder named on the command line, else the project
 * opened last, if the app remembers one. A failure is kept in `error`.
 */
export async function openInitialProject(): Promise<void> {
  try {
    const argument = await commands.projectArgument();
    if (argument !== null) {
      await openProject(argument);
      return;
    }
  } catch (failure) {
    project.error = `The command line could not be read: ${failureText(failure)}`;
  }
  await openLastProject();
}

/** Opens the project opened last, if the app remembers one. A failure is kept in `error`. */
async function openLastProject(): Promise<void> {
  try {
    const last = await commands.projectLast();
    if (last !== null) {
      await openProject(last);
    }
  } catch (failure) {
    project.error = `The last project could not be read: ${failureText(failure)}`;
  } finally {
    project.opening = false;
  }
}

/**
 * Asks for a folder with the OS dialog and opens it (the command Open project). Cancelling
 * changes nothing; a folder that does not open keeps the open project and is kept in `error`.
 */
export async function chooseProject(): Promise<void> {
  let path: string | null;
  try {
    path = await pickFolder();
  } catch (failure) {
    project.error = `The folder dialog failed: ${failureText(failure)}`;
    return;
  }
  if (path !== null) {
    await openProject(path);
  }
}

/** The sync that runs and why, so a held key or a second Reload starts no second one. */
let syncing: { reason: SyncReason; done: Promise<void> } | null = null;

/** The error the last failed sync left in `error`, which the next good sync takes back. */
let syncError: string | null = null;

/**
 * Syncs the open project with its files and shows the view after it: `reload` for the command
 * Reload project, which always syncs, and `focus` for the window getting the focus back, which
 * the app holds back within a second of the last sync. Open records deleted outside the editor
 * close, changed ones load again. A failure keeps the view and is kept in `error`. A call while a
 * reload runs, or a focus sync while any sync runs, waits for it instead of starting one; a
 * reload while a focus sync runs syncs once more after it, since the app may hold that one back.
 * Without an open project it does nothing.
 */
export function reloadProject(reason: SyncReason = 'reload'): Promise<void> {
  if (project.view === null) {
    return Promise.resolve();
  }
  if (syncing !== null && (syncing.reason === 'reload' || reason === 'focus')) {
    return syncing.done;
  }
  const done = syncing === null ? syncOnce(reason) : syncing.done.then(() => syncOnce(reason));
  const entry = { reason, done };
  syncing = entry;
  void entry.done.then(() => {
    if (syncing === entry) {
      syncing = null;
    }
  });
  return entry.done;
}

/** One sync for `reason`; a failure is kept in `error`. */
async function syncOnce(reason: SyncReason): Promise<void> {
  try {
    const synced = await call(commands.projectSync(reason));
    const files = synced.changed.length + synced.removed.length + synced.moved.length;
    if (files > 0 || differs(synced.project)) {
      project.view = synced.project;
    }
    if (project.error !== null && project.error === syncError) {
      project.error = null;
    }
    syncError = null;
    await refreshRecords(synced);
  } catch (failure) {
    syncError = `The project could not be synced: ${failureText(failure)}`;
    project.error = syncError;
  }
}

/** Whether `view` differs from the open project's view, so the shell redraws only on news. */
function differs(view: ProjectView): boolean {
  return JSON.stringify(view) !== JSON.stringify(project.view);
}

/** Dismisses the error the banner shows. */
export function dismissError(): void {
  project.error = null;
  syncError = null;
}

/** Makes the rail entry `id` active: its preset replaces the dock tree. */
export function activate(id: string): void {
  project.active = id;
}
