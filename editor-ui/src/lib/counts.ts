// The counts the title bar, the status bar and the record header show (P11 §5.8), as text.

import type { ProjectView } from './ipc/bindings';

/** `n` with its noun, `1 record` or `21 records`; `plural` when the plural is not the noun + `s`. */
export function count(n: number, noun: string, plural = `${noun}s`): string {
  return `${n} ${n === 1 ? noun : plural}`;
}

/** The signal of a pip, as its `is-*` class names it. */
export type Signal = 'reject' | 'warn' | 'approve';

/** The validator pip: errors first, then warnings, else clean. */
export function validator(view: Pick<ProjectView, 'errors' | 'warnings'>): {
  text: string;
  signal: Signal;
} {
  if (view.errors > 0) {
    return { text: `validator · ${count(view.errors, 'error')}`, signal: 'reject' };
  }
  if (view.warnings > 0) {
    return { text: `validator · ${view.warnings} warn`, signal: 'warn' };
  }
  return { text: 'validator · clean', signal: 'approve' };
}

/** The status bar's counts: `21 records · 5 errors · 2 warnings`. */
export function totals(view: Pick<ProjectView, 'records' | 'errors' | 'warnings'>): string {
  return [
    count(view.records, 'record'),
    count(view.errors, 'error'),
    count(view.warnings, 'warning'),
  ].join(' · ');
}
