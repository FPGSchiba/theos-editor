// The one place that awaits the results of the generated commands (P11 §5.6): a screen calls
// `call(commands.recordView(id))` and gets the data, or an `IpcFailure` it can show.

import type { IpcError, IpcErrorKind } from './bindings';

/** What a generated command with a typed error resolves to (tauri-specta's `typedError`). */
export type Result<T, E> = { status: 'ok'; data: T } | { status: 'error'; error: E };

/** A command that answered with an `IpcError`: its kind, and the text to show as `message`. */
export class IpcFailure extends Error {
  /** What kind of failure. */
  readonly kind: IpcErrorKind;

  constructor(error: IpcError) {
    super(error.message);
    this.name = 'IpcFailure';
    this.kind = error.kind;
  }
}

/**
 * A failure of a command as text for the user: the message of an `IpcFailure`, of an `Error` or
 * of an error value with a `message`, else the value as text.
 */
export function failureText(failure: unknown): string {
  if (failure instanceof Error) {
    return failure.message;
  }
  if (typeof failure === 'object' && failure !== null && 'message' in failure) {
    return String(failure.message);
  }
  return String(failure);
}

/** The data of the command's result; throws an `IpcFailure` when it answered with an error. */
export async function call<T>(result: Promise<Result<T, IpcError>>): Promise<T> {
  const answer = await result;
  if (answer.status === 'error') {
    throw new IpcFailure(answer.error);
  }
  return answer.data;
}
