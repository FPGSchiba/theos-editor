import { expect, test } from 'vitest';

import type { IpcError } from './bindings';
import { call, IpcFailure, type Result } from './client';

test('call gives the data of an ok result', async () => {
  const ok: Result<number, IpcError> = { status: 'ok', data: 7 };
  await expect(call(Promise.resolve(ok))).resolves.toBe(7);
});

test('call throws an IpcFailure with the kind and message of an error result', async () => {
  const failed: Result<number, IpcError> = {
    status: 'error',
    error: { kind: 'not_found', message: 'no record `x`' },
  };
  const thrown = await call(Promise.resolve(failed)).catch((e: unknown) => e);
  expect(thrown).toBeInstanceOf(IpcFailure);
  expect(thrown).toMatchObject({ kind: 'not_found', message: 'no record `x`' });
});
