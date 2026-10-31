import type { Finding } from './types.ts';

/** The words the inworld-vocabulary check flags in voice text (mock list, spec §4.5). */
export const developerWords: string[] = ['T-012', 'never-confirm', 'tier', 'ledger', 'canon'];

/**
 * Mock findings per file, keyed by the manifest path. The one finding the suites look for is
 * a `never-confirm` error on beat 2 of S-004, which reveals C-031 (G8: always blocking).
 */
const FINDINGS: Record<string, Finding[]> = {
  'records/story/scripts/S-004.md': [
    {
      rule: 'never-confirm',
      severity: 'error',
      message: 'never-confirm: beat 2 would confirm T-012 through C-031',
      at: { beat: 2 },
    },
  ],
};

/** The findings of one file; `fileId` is its manifest path, e.g. `records/story/scripts/S-004.md`. */
export function findingsFor(fileId: string): Finding[] {
  return (FINDINGS[fileId] ?? []).map((finding) => ({ ...finding, at: { ...finding.at } }));
}
