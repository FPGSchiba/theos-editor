// `pnpm app` stops the app as a whole process tree (scripts/stop-tree.js): `taskkill /T /F` on
// Windows, SIGTERM to the process group elsewhere.

import { expect, test } from 'vitest';

import { stopTree } from '../scripts/stop-tree.js';

/** Tools that record what `stopTree` does on `platform`. */
function recorder(platform: NodeJS.Platform, kill?: () => never) {
  const done: unknown[] = [];
  return {
    done,
    tools: {
      platform,
      run: (cmd: string, args: string[]) => done.push([cmd, ...args]),
      kill:
        kill ??
        ((pid: number, signal: NodeJS.Signals) => {
          done.push([pid, signal]);
        }),
    },
  };
}

test('on windows taskkill stops the whole tree', () => {
  const { done, tools } = recorder('win32');
  stopTree(4242, tools);
  expect(done).toEqual([['taskkill', '/T', '/F', '/PID', '4242']]);
});

test('elsewhere the process group gets SIGTERM', () => {
  for (const platform of ['linux', 'darwin'] as const) {
    const { done, tools } = recorder(platform);
    stopTree(4242, tools);
    expect(done).toEqual([[-4242, 'SIGTERM']]);
  }
});

test('a group that is gone and a child that never started are no error', () => {
  const { done, tools } = recorder('linux', () => {
    throw new Error('ESRCH');
  });
  expect(() => stopTree(4242, tools)).not.toThrow();
  stopTree(undefined, tools);
  expect(done).toEqual([]);
});
