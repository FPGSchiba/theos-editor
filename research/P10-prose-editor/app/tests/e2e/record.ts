import { appendFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

/** The kinds of result rows; each one becomes `report/<kind>.json` and `.md` (write-reports.ts). */
export const RECORD_KINDS = ['round-trip', 'features', 'input', 'integration', 'long-docs'] as const;
export type RecordKind = (typeof RECORD_KINDS)[number];

/** `app/test-results/raw/`: one JSON Lines file per kind and worker, cleaned by Playwright at the start of a run. */
export const RAW_DIR = fileURLToPath(new URL('../../test-results/raw/', import.meta.url));

/** Append one result row to `test-results/raw/<kind>-<worker>.jsonl`. */
export function record(kind: RecordKind, row: object): void {
  mkdirSync(RAW_DIR, { recursive: true });
  const worker = process.env.TEST_WORKER_INDEX ?? '0';
  appendFileSync(join(RAW_DIR, `${kind}-${worker}.jsonl`), `${JSON.stringify(row)}\n`);
}
