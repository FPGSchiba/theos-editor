// Stops a child process and everything it started (CLAUDE.md: processes are stopped as whole
// trees). On Windows `taskkill /T /F` ends the tree (cargo, the app, its WebView2 children); on
// Unix the child leads its own process group (`detached: true`), which gets SIGTERM. It runs
// synchronously, so it also works in node's `exit` handler.

import { spawnSync } from 'node:child_process';

/**
 * @typedef {object} StopTools
 * @property {NodeJS.Platform} platform The OS, as `process.platform`.
 * @property {(cmd: string, args: string[]) => unknown} run Runs a command without a shell and waits.
 * @property {(pid: number, signal: NodeJS.Signals) => unknown} kill Sends a signal, as `process.kill`.
 */

/** @type {StopTools} */
const NODE = {
  platform: process.platform,
  run: (cmd, args) => spawnSync(cmd, args, { stdio: 'ignore', windowsHide: true }),
  kill: (pid, signal) => process.kill(pid, signal),
};

/**
 * Stops the process `pid` and its whole tree; nothing when `pid` is undefined. A tree that is
 * already gone is no error.
 * @param {number | undefined} pid
 * @param {StopTools} [tools]
 */
export function stopTree(pid, tools = NODE) {
  if (pid === undefined) {
    return;
  }
  if (tools.platform === 'win32') {
    tools.run('taskkill', ['/T', '/F', '/PID', String(pid)]);
    return;
  }
  try {
    tools.kill(-pid, 'SIGTERM');
  } catch {
    // The group has exited already.
  }
}
