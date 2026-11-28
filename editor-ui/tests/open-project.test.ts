// Opening a project (P11 §5.9): the folder named on the command line opens first, else the last
// project; the folder dialog opens the folder chosen, and cancelling it changes nothing.

import { beforeEach, expect, test, vi } from 'vitest';

const ipc = vi.hoisted(() => ({
  argument: null as string | null,
  last: null as string | null,
  chosen: null as string | null,
  calls: [] as string[],
}));

vi.mock('../src/lib/ipc/bindings', () => ({
  commands: {
    projectArgument: () => {
      ipc.calls.push('project_argument');
      return Promise.resolve(ipc.argument);
    },
    projectLast: () => {
      ipc.calls.push('project_last');
      return Promise.resolve(ipc.last);
    },
    projectOpen: (path: string) => {
      ipc.calls.push(`project_open ${path}`);
      return Promise.resolve(
        path.startsWith('synthetic/')
          ? { status: 'ok', data: { name: path } }
          : { status: 'error', error: { kind: 'not_a_project', message: `\`${path}\` is not a content project` } },
      );
    },
  },
}));

vi.mock('../src/lib/ipc/dialog', () => ({
  pickFolder: () => {
    ipc.calls.push('dialog');
    return Promise.resolve(ipc.chosen);
  },
}));

const { chooseProject, openInitialProject, project } = await import(
  '../src/lib/state/project.svelte'
);
const { find } = await import('../src/lib/state/find.svelte');

beforeEach(() => {
  Object.assign(ipc, { argument: null, last: null, chosen: null, calls: [] });
  project.view = null;
  project.error = null;
  project.opening = true;
});

// AC: CE-32.1
test('the command-line argument opens before the last project', async () => {
  ipc.argument = 'synthetic/from-the-command-line';
  ipc.last = 'synthetic/last';
  await openInitialProject();
  expect(ipc.calls).toEqual(['project_argument', 'project_open synthetic/from-the-command-line']);
  expect(project.view?.name).toBe('synthetic/from-the-command-line');
  expect(project.opening).toBe(false);
});

// AC: CE-32.1
test('without an argument the last project opens', async () => {
  ipc.last = 'synthetic/last';
  await openInitialProject();
  expect(ipc.calls).toEqual(['project_argument', 'project_last', 'project_open synthetic/last']);
  expect(project.view?.name).toBe('synthetic/last');
});

test('with neither the shell stays empty', async () => {
  await openInitialProject();
  expect(ipc.calls).toEqual(['project_argument', 'project_last']);
  expect(project.view).toBeNull();
  expect(project.opening).toBe(false);
});

// AC: CE-32.1
test('the folder dialog opens the folder chosen; cancelling changes nothing', async () => {
  await chooseProject();
  expect(ipc.calls).toEqual(['dialog']);
  ipc.chosen = 'synthetic/chosen';
  await chooseProject();
  expect(ipc.calls).toEqual(['dialog', 'dialog', 'project_open synthetic/chosen']);
  expect(project.view?.name).toBe('synthetic/chosen');
});

// AC: CE-32.1
test('a command-line path that does not open ends in the empty shell, not the last project', async () => {
  ipc.argument = 'missing/folder';
  ipc.last = 'synthetic/last';
  await openInitialProject();
  expect(ipc.calls).toEqual(['project_argument', 'project_open missing/folder']);
  expect(project.view).toBeNull();
  expect(project.error).toBe('`missing/folder` is not a content project');
  expect(project.opening).toBe(false);
});

test('another project opening empties the find field and its hits', async () => {
  Object.assign(find, {
    query: ':clue',
    open: true,
    hits: [{ id: 'C-031' }],
    truncated: true,
    note: 'synthetic note',
    selected: 0,
  });
  ipc.chosen = 'synthetic/chosen';
  await chooseProject();
  expect(find).toEqual({ query: '', open: false, hits: [], truncated: false, note: null, selected: -1 });
});
