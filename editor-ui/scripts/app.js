// `pnpm --dir editor-ui app [<project>]` (P11 §5.2): the Vite dev server on port 5179 in this
// process, and the app (`cargo run -p theos-editor -- [<project>]`) as its one child. A relative
// project path is taken from the folder the command was typed in (pnpm's INIT_CWD), not from
// editor-ui. When the app exits (its window closed), the dev server stops and the script exits
// with the app's code. On Ctrl+C, SIGTERM, SIGHUP or any exit of this script, the app's whole
// process tree is stopped (`stop-tree.js`).

import { spawn } from 'node:child_process';
import { resolve } from 'node:path';

import { createServer } from 'vite';

import { stopTree } from './stop-tree.js';

const typedIn = process.env['INIT_CWD'] ?? process.cwd();
const project = process.argv.slice(2).map((arg) => resolve(typedIn, arg));

const server = await createServer();
await server.listen();
server.printUrls();

// On Unix the child leads its own process group, so the whole group can be stopped.
const app = spawn('cargo', ['run', '-p', 'theos-editor', '--', ...project], {
  stdio: 'inherit',
  detached: process.platform !== 'win32',
});

/** Whether the app's process is still running. */
const running = () => app.exitCode === null && app.signalCode === null;

/**
 * Stops the app's tree if it runs, then the dev server, and exits with `code`.
 * @param {number} code
 */
async function stop(code) {
  if (running()) {
    stopTree(app.pid);
  }
  await server.close();
  process.exit(code);
}

app.on('error', (error) => {
  console.error(`cargo could not be started: ${error.message}`);
  void stop(1);
});
app.on('exit', (code) => void stop(code ?? 1));
for (const signal of /** @type {const} */ (['SIGINT', 'SIGTERM', 'SIGHUP'])) {
  process.on(signal, () => void stop(130));
}
process.on('exit', () => {
  if (running()) {
    stopTree(app.pid);
  }
});
