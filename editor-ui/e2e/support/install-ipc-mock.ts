// Runs in the page before the app (bundled by `replay.ts` into a script). It installs Tauri's own
// `mockIPC` and asks the test, through the function `replay.ts` exposes, how to answer each call.
// Events are mocked too (`listen` and `emit` stay in the page), and the window is `main`, so the
// app can listen to its window's focus and a test can send it with `theosReplayEmit`.

import { emit } from '@tauri-apps/api/event';
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';

/** How a call is answered: like the command, with its value or with its error. */
export interface Answer {
  result: 'ok' | 'error';
  value: unknown;
}

declare global {
  interface Window {
    /** Exposed by `replay.ts`: the recorded answer to `cmd` with `args`. */
    theosReplayAnswer: (cmd: string, args: unknown) => Promise<Answer>;
    /** Sends the event `name` to the page's listeners, as the app's window would. */
    theosReplayEmit: (name: string) => Promise<void>;
  }
}

mockIPC(async (cmd, args) => {
  const answer = await window.theosReplayAnswer(cmd, args ?? {});
  if (answer.result === 'error') {
    // A failed command rejects with its error value, not an Error, as Tauri does; the generated
    // bindings turn that into `{ status: "error", error }`.
    throw answer.value;
  }
  return answer.value;
}, { shouldMockEvents: true });

mockWindows('main');

window.theosReplayEmit = (name) => emit(name);
