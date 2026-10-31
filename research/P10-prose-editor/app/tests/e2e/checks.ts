import { expect, type Page } from '@playwright/test';
import { readManifest, type CorpusFile } from '../../src/harness/manifest.ts';
import { record, strict } from './helpers.ts';
import type { CheckRow } from './write-reports.ts';

// What the feature, input and integration suites share: how a check turns into a row, and the
// few page queries every one of them needs. A check measures what an editor variant renders or
// does; `ok: false` is a result, not a test failure, so only P10_STRICT=1 fails a test.

/** The kinds of result rows that hold checks (`long-docs` has its own shape). */
export type CheckKind = 'features' | 'input' | 'integration';

export interface Subject {
  variant: string;
  engine: string;
  check: string;
}

/** What a check found. `detail` says why it failed, or what it saw when it passed. */
export interface Verdict {
  ok: boolean;
  detail?: string;
}

/** A verdict from the problems a check found: none is a pass, else the problems are the detail. */
export function judge(problems: string[], seen?: string): Verdict {
  if (problems.length > 0) return { ok: false, detail: problems.join('; ') };
  return seen === undefined ? { ok: true } : { ok: true, detail: seen };
}

const firstLine = (error: unknown): string => {
  const message = error instanceof Error ? error.message : String(error);
  return (message.split('\n')[0] ?? message).slice(0, 300);
};

/**
 * Run the check and record its row. An exception is a harness error and becomes an `error` row.
 * Under P10_STRICT=1 an error or `ok: false` fails the test (after the row is recorded); `gap`
 * rows are known gaps of the tooling, which no prototype can close, so they never fail it.
 */
export async function runCheck(
  kind: CheckKind,
  subject: Subject,
  run: () => Promise<Verdict>,
  options: { gap?: boolean } = {},
): Promise<void> {
  let row: CheckRow;
  try {
    const verdict = await run();
    row = verdict.detail === undefined ? { ...subject, ok: verdict.ok } : { ...subject, ok: verdict.ok, detail: verdict.detail };
  } catch (error) {
    row = { ...subject, ok: false, error: firstLine(error) };
  }
  record(kind, row);
  if (!strict() || options.gap === true) return;
  expect(row.error, `${subject.check} could not run`).toBeUndefined();
  expect(row.ok, `${subject.check} failed: ${row.detail ?? ''}`).toBe(true);
}

/** A file of the corpus by its manifest path. */
export function corpusFile(path: string): CorpusFile {
  const file = readManifest().files.find((candidate) => candidate.path === path);
  if (file === undefined) throw new Error(`checks: the manifest has no file ${path}`);
  return file;
}

/** Two animation frames: what an editor that updates its view in a frame has surely done. */
export function settle(page: Page): Promise<void> {
  return page.evaluate(
    () => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))),
  );
}

/** Poll `condition` until it holds or `timeoutMs` has passed; returns whether it held. */
export async function eventually(condition: () => Promise<boolean>, timeoutMs = 1_000): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    if (await condition()) return true;
    if (Date.now() >= deadline) return false;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
}

/** Focus the editor and put the caret at `offset` (into the LF form of the file). */
export async function caretAt(page: Page, offset: number): Promise<void> {
  await page.evaluate((at) => {
    const harness = window.__harness!;
    harness.focus();
    harness.caretTo(at);
  }, offset);
}

/** What the editor would save right now. */
export const savedText = (page: Page): Promise<string> => page.evaluate(() => window.__harness!.text());

/** How many elements of the editor match; the popover lives outside it, see `count(page, sel, false)`. */
export function count(page: Page, selector: string, insideEditor = true): Promise<number> {
  return page.evaluate(
    ([sel, scoped]) => (scoped ? document.getElementById('editor-host')! : document).querySelectorAll(sel).length,
    [selector, insideEditor] as const,
  );
}

/** The `textContent` of every element of the editor that matches, whitespace collapsed. */
export function texts(page: Page, selector: string): Promise<string[]> {
  return page.evaluate(
    (sel) =>
      [...document.getElementById('editor-host')!.querySelectorAll(sel)].map((node) =>
        (node.textContent ?? '').replace(/\s+/g, ' ').trim(),
      ),
    selector,
  );
}

/**
 * What a person can read in the editor: the rendered text, plus the value of a form field that
 * is on screen (a textarea shows its text raw, and so does a Source mode that is built on one;
 * a hidden one shows nothing).
 */
export function visibleText(page: Page): Promise<string> {
  return page.evaluate(() => {
    const host = document.getElementById('editor-host')!;
    const fields = [...host.querySelectorAll<HTMLTextAreaElement | HTMLInputElement>('textarea, input')]
      .filter((field) => field.checkVisibility({ opacityProperty: true, visibilityProperty: true }))
      .map((field) => field.value);
    return [host.innerText, ...fields].join('\n');
  });
}
