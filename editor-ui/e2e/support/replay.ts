// Page tests answer IPC from the recordings in e2e/ipc/ (P11 §5.12), which a Rust test writes
// from real answers of the commands. `replay(page, { os })` installs `mockIPC` before the app
// loads; each call is answered by the recording with the same `cmd` and deep-equal `args`. A call
// no recording answers fails the test, naming its cmd and args.
//
// Several recordings may share cmd and args (`project_sync-changed` and `project_sync-removed`).
// Then the one named in `use` answers, the one named last when several are; without one, the
// only recording with neither a `derived` key nor a `fixture` key (a call on another fixture
// project, such as `duplicate-id`); else the call is ambiguous and fails like an unmatched one.
// `app_info` is answered by `app_info-<os>`. A command named in `fail` rejects with the error
// value given there, so a test can show how the page handles a call that fails. A command named
// in `answer` (a plugin's command, such as the folder dialog's `plugin:dialog|open`, which no
// recording can hold) answers with the value given there. Events are mocked in the page:
// `emit(name)` sends one to the app's listeners, as the window's focus does.

import { readdirSync, readFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

import { expect, type Page } from '@playwright/test';
import { build } from 'vite';

import type { Answer } from './install-ipc-mock';

/** The OS the app says it runs on (`app_info`), which picks Ctrl or Cmd. */
export type Os = 'windows' | 'macos' | 'linux';

/** One file of e2e/ipc/, named `<cmd>[-<variant>].json`. */
export interface Recording extends Answer {
  /** The file name without `.json`. */
  name: string;
  cmd: string;
  args: unknown;
  /** How a variant was derived from a real answer, when it was. */
  derived?: string;
  /** The fixture project of a call on another one than `editor-codex`. */
  fixture?: string;
}

/** What `replay` is told. */
export interface ReplayOptions {
  /** The OS `app_info` answers with. */
  os: Os;
  /** Recordings that answer instead of others with the same cmd and args. */
  use?: readonly string[];
  /** Commands that reject, each with the error value it rejects with, instead of a recording. */
  fail?: Readonly<Record<string, unknown>>;
  /** Commands that answer with the value given, instead of a recording (plugin commands). */
  answer?: Readonly<Record<string, unknown>>;
  /** The folder of the recordings; e2e/ipc/ unless a test of this helper names another. */
  dir?: string;
}

/** A call the page made. */
export interface Call {
  cmd: string;
  args: unknown;
}

/** The replay of one page. */
export interface Replay {
  /** Every call the page made, in order. */
  readonly calls: readonly Call[];
  /** From now on, these recordings answer instead of others with the same cmd and args. */
  use(...names: string[]): void;
  /** Sends the event `name` (such as `tauri://focus`) to the page's listeners. */
  emit(name: string): Promise<void>;
}

const SUPPORT = fileURLToPath(new URL('.', import.meta.url));
const RECORDINGS_DIR = join(SUPPORT, '..', 'ipc');

/** Every recording in `dir` (e2e/ipc/). */
export function loadRecordings(dir: string = RECORDINGS_DIR): Recording[] {
  return readdirSync(dir)
    .filter((file) => file.endsWith('.json'))
    .map((file) => ({
      ...(JSON.parse(readFileSync(join(dir, file), 'utf8')) as Omit<Recording, 'name'>),
      name: basename(file, '.json'),
    }));
}

/** The recording that answers `call`, or why none does. */
export function choose(
  recordings: readonly Recording[],
  preferred: readonly string[],
  call: Call,
): Recording | string {
  const candidates = recordings.filter(
    (r) => r.cmd === call.cmd && isDeepStrictEqual(r.args, call.args),
  );
  const named = preferred.findLast((name) => candidates.some((r) => r.name === name));
  const plain = candidates.filter((r) => r.derived === undefined && r.fixture === undefined);
  let chosen: Recording | undefined;
  if (candidates.length === 1) {
    chosen = candidates[0];
  } else if (named !== undefined) {
    chosen = candidates.find((r) => r.name === named);
  } else if (plain.length === 1) {
    chosen = plain[0];
  }
  if (chosen !== undefined) {
    return chosen;
  }
  const shown = `${call.cmd} ${JSON.stringify(call.args)}`;
  if (candidates.length === 0) {
    return `no recording in e2e/ipc/ answers ${shown}`;
  }
  const names = candidates.map((r) => r.name).join(', ');
  return `several recordings answer ${shown} (${names}); name one in \`use\``;
}

let mockScript: Promise<string> | undefined;

/** `install-ipc-mock.ts` with Tauri's mocks, bundled by Vite into one script (once per worker). */
function installScript(): Promise<string> {
  mockScript ??= build({
    configFile: false,
    logLevel: 'silent',
    publicDir: false,
    build: {
      write: false,
      minify: false,
      lib: {
        entry: join(SUPPORT, 'install-ipc-mock.ts'),
        formats: ['iife'],
        name: 'theosReplayMock',
      },
    },
  }).then((output) => {
    const chunk = (Array.isArray(output) ? output : [output])
      .flatMap((o) => ('output' in o ? o.output : []))
      .find((o) => o.type === 'chunk');
    if (chunk === undefined) {
      throw new Error('Vite built no script from e2e/support/install-ipc-mock.ts');
    }
    return chunk.code;
  });
  return mockScript;
}

/** Answers every IPC call of `page` from the recordings; call it before `page.goto`. */
export async function replay(page: Page, options: ReplayOptions): Promise<Replay> {
  const recordings = loadRecordings(options.dir);
  const preferred = [`app_info-${options.os}`];
  const calls: Call[] = [];
  const use = (...names: string[]): void => {
    const unknown = names.filter((name) => !recordings.some((r) => r.name === name));
    if (unknown.length > 0) {
      throw new Error(`no recording named ${unknown.join(', ')} in e2e/ipc/`);
    }
    preferred.push(...names);
  };
  use(...(options.use ?? []));

  await page.exposeFunction('theosReplayAnswer', (cmd: string, args: unknown): Answer => {
    const call = { cmd, args };
    calls.push(call);
    if (options.fail !== undefined && Object.hasOwn(options.fail, cmd)) {
      return { result: 'error', value: options.fail[cmd] };
    }
    if (options.answer !== undefined && Object.hasOwn(options.answer, cmd)) {
      return { result: 'ok', value: options.answer[cmd] };
    }
    const chosen = choose(recordings, preferred, call);
    if (typeof chosen === 'string') {
      expect.soft(chosen, 'every IPC call has a recording').toBe('');
      return { result: 'error', value: { kind: 'replay', message: chosen } };
    }
    return { result: chosen.result, value: chosen.value };
  });
  await page.addInitScript({ content: await installScript() });
  const emit = (name: string): Promise<void> =>
    page.evaluate((event) => window.theosReplayEmit(event), name);
  return { calls, use, emit };
}
