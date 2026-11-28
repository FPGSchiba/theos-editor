// Reload project (P11 §5.9, §5.11): a second Reload while a sync runs starts no second sync;
// without a project nothing syncs; a failed sync keeps its error until a good sync takes it back.

import { beforeEach, expect, test, vi } from 'vitest';

import type { ProjectView } from '../src/lib/ipc/bindings';

const sync = vi.hoisted(() => ({
  calls: 0,
  reasons: [] as string[],
  resolve: (): void => undefined,
  reject: (): void => undefined,
}));

vi.mock('../src/lib/ipc/bindings', () => ({
  commands: {
    projectSync: (reason: string) => {
      sync.calls += 1;
      sync.reasons.push(reason);
      return new Promise((resolve) => {
        sync.resolve = () =>
          resolve({ status: 'ok', data: { project: { name: 'p' }, changed: [], removed: [] } });
        sync.reject = () =>
          resolve({ status: 'error', error: { kind: 'index', message: 'damaged' } });
      });
    },
  },
}));

const { project, reloadProject } = await import('../src/lib/state/project.svelte');

beforeEach(() => {
  sync.calls = 0;
  sync.reasons = [];
  project.view = { name: 'open' } as ProjectView;
  project.error = null;
});

test('a reload while one runs waits for it instead of syncing again', async () => {
  const first = reloadProject();
  const second = reloadProject();
  expect(sync.calls).toBe(1);
  sync.resolve();
  await Promise.all([first, second]);
  expect(project.view?.name).toBe('p');
  const third = reloadProject();
  expect(sync.calls).toBe(2);
  sync.resolve();
  await third;
});

test('reload asks for a reload and focus for a focus sync', async () => {
  const reload = reloadProject();
  sync.resolve();
  await reload;
  const focus = reloadProject('focus');
  sync.resolve();
  await focus;
  expect(sync.reasons).toEqual(['reload', 'focus']);
});

test('without a project nothing syncs', async () => {
  project.view = null;
  await reloadProject();
  expect(sync.calls).toBe(0);
  expect(project.error).toBeNull();
});

test('a failed sync keeps the view and its error until a good sync', async () => {
  const failing = reloadProject();
  sync.reject();
  await failing;
  expect(project.view?.name).toBe('open');
  expect(project.error).toBe('The project could not be synced: damaged');
  const good = reloadProject();
  sync.resolve();
  await good;
  expect(project.error).toBeNull();
});

test('a good sync leaves an error that did not come from a sync', async () => {
  project.error = 'the folder is not a project';
  const good = reloadProject();
  sync.resolve();
  await good;
  expect(project.error).toBe('the folder is not a project');
});

test('a reload during a focus sync syncs again after it', async () => {
  const focus = reloadProject('focus');
  const reload = reloadProject('reload');
  const again = reloadProject('reload');
  expect(sync.reasons).toEqual(['focus']);
  sync.resolve();
  await focus;
  await vi.waitFor(() => expect(sync.reasons).toEqual(['focus', 'reload']));
  sync.resolve();
  await Promise.all([reload, again]);
  expect(sync.reasons).toEqual(['focus', 'reload']);
});

test('a focus sync during a reload waits for it', async () => {
  const reload = reloadProject('reload');
  const focus = reloadProject('focus');
  sync.resolve();
  await Promise.all([reload, focus]);
  expect(sync.reasons).toEqual(['reload']);
});
