import { expect, type Page } from '@playwright/test';
import { readManifest, type CorpusFile } from '../../src/harness/manifest.ts';
import { chipKeys } from './chip-key.ts';
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

// ── Reading a whole record from an editor that draws only part of it ─────────

/**
 * The elements that scroll the editor: the focused element (or the host) and its ancestors that
 * have something to scroll, scrollable elements inside the host, and the page itself. It runs in
 * the page, so it is passed by its source (`inPage`); `long-docs.spec.ts` uses it too.
 */
export function findScrollers(): Element[] {
  const host = document.getElementById('editor-host')!;
  const active = document.activeElement;
  const scrollers = new Set<Element>();
  for (let node: Element | null = active !== null && host.contains(active) ? active : host; node !== null; node = node.parentElement) {
    if (node.scrollHeight > node.clientHeight + 1) scrollers.add(node);
  }
  for (const node of host.querySelectorAll('*')) {
    const overflow = getComputedStyle(node).overflowY;
    if ((overflow === 'auto' || overflow === 'scroll') && node.scrollHeight > node.clientHeight + 1) scrollers.add(node);
  }
  if (document.scrollingElement !== null) scrollers.add(document.scrollingElement);
  return [...scrollers];
}

/** The top of an element in document coordinates (its rect plus every scroll offset above it), rounded. */
export function absoluteTop(element: Element): number {
  let top = element.getBoundingClientRect().top;
  for (let node = element.parentElement; node !== null; node = node.parentElement) top += node.scrollTop;
  return Math.round(top);
}

/** A function of this file as source text, to be evaluated in the page (`(0, eval)(inPage(fn))`). */
export const inPage = (fn: (...args: never[]) => unknown): string => `(${fn.toString()})`;

const MAX_STEPS = 400;

/** Scroll every scroller of the editor to `step` half-windows from the top; true when all are at their end. */
function scrollEditorTo(page: Page, step: number): Promise<boolean> {
  return page.evaluate(
    ([finder, index]) => {
      const scrollers = (0, eval)(finder)() as Element[];
      const half = window.innerHeight / 2;
      for (const node of scrollers) node.scrollTop = index * half;
      return scrollers.every((node) => node.scrollTop + node.clientHeight >= node.scrollHeight - 2);
    },
    [inPage(findScrollers), step] as const,
  );
}

/**
 * Run `read` with the editor scrolled to the top and then down in half-window steps (two frames
 * at each), and return the union of what it found; the editor is scrolled back to the top at the
 * end. A viewport editor draws only what is near the window, so a check that reads constructs
 * that are not under the caret must read them where they are drawn. Items are told apart by
 * `key` (default: the item as JSON): the DOM elements of a viewport editor are recycled, so an
 * item has to carry a document-absolute position (see `collectRows`), not an element identity.
 * The caret is never moved and the viewport is never enlarged.
 */
export async function collectWhileScrolling<T>(
  page: Page,
  read: () => Promise<T[]>,
  key: (item: T) => string = (item) => JSON.stringify(item),
): Promise<T[]> {
  const found = new Map<string, T>();
  const take = async (): Promise<void> => {
    for (const item of await read()) if (!found.has(key(item))) found.set(key(item), item);
  };
  const scrollTo = (step: number): Promise<boolean> => scrollEditorTo(page, step);
  try {
    await scrollTo(0);
    await settle(page);
    await take();
    let reachedEnd = false;
    for (let step = 1; step <= MAX_STEPS && !reachedEnd; step++) {
      reachedEnd = await scrollTo(step);
      await settle(page);
      await take();
    }
    // A truncated read would be a wrong verdict; an error row says the check could not run.
    if (!reachedEnd) throw new Error(`collectWhileScrolling: the end was not reached in ${MAX_STEPS} half-window steps`);
  } finally {
    await scrollTo(0);
    await settle(page);
  }
  return [...found.values()];
}

/**
 * Scroll the editor until an element matching `selector` is attached, then scroll it into view
 * (centred). For a driver or a check that has to click or fill a rendered element that is not
 * under the caret: a viewport editor draws only what is near the window, so a person would scroll
 * to it first. It starts at the top and goes down in half-window steps (two frames at each), over
 * the whole range; when `waitMs` is given it repeats the pass until that time is up, for an
 * element that is mounted late. It throws when the end is reached without the element. The caret
 * is never moved (moving it would turn the construct raw) and the viewport is never enlarged.
 */
export async function revealElement(page: Page, selector: string, waitMs = 0): Promise<void> {
  const present = (): Promise<boolean> =>
    page.evaluate((sel) => document.getElementById('editor-host')?.querySelector(sel) != null, selector);
  const show = async (): Promise<boolean> => {
    await page.evaluate(
      (sel) => document.getElementById('editor-host')?.querySelector(sel)?.scrollIntoView({ block: 'center' }),
      selector,
    );
    await settle(page);
    return present();
  };
  const deadline = Date.now() + waitMs;
  for (;;) {
    // An element that is already drawn is only brought into view: scrolling to the top first could
    // push it out of a viewport editor's window and drop the UI state it holds.
    if ((await present()) && (await show())) return;
    await scrollEditorTo(page, 0);
    await settle(page);
    let reachedEnd = false;
    for (let step = 1; step <= MAX_STEPS; step++) {
      if ((await present()) && (await show())) return;
      if (reachedEnd) break;
      reachedEnd = await scrollEditorTo(page, step);
      await settle(page);
    }
    if (!reachedEnd) throw new Error(`revealElement: the end was not reached in ${MAX_STEPS} half-window steps`);
    if (Date.now() >= deadline) throw new Error(`revealElement: nothing matches ${selector}`);
  }
}

/**
 * Every element of the editor matching `selector`, described by `pick` (which runs in the page)
 * and given `at`, its document-absolute top. Collected over the whole scroll range. The `at` in
 * the default key is what tells two equal constructs apart and stops one construct that is read
 * in two steps from counting twice.
 */
export function collectRows<T extends object>(
  page: Page,
  selector: string,
  pick: (element: Element) => T,
  key?: (row: T & { at: number }) => string,
): Promise<Array<T & { at: number }>> {
  return collectWhileScrolling(
    page,
    () =>
      page.evaluate(
        ([sel, pickSource, topSource]) => {
          const pickIt = (0, eval)(pickSource) as (element: Element) => object;
          const topOf = (0, eval)(topSource) as (element: Element) => number;
          return [...document.getElementById('editor-host')!.querySelectorAll(sel)].map((element) => ({
            ...pickIt(element),
            at: topOf(element),
          }));
        },
        [selector, inPage(pick), inPage(absoluteTop)] as const,
      ) as Promise<Array<T & { at: number }>>,
    key,
  );
}

/** Like `texts`, over the whole scroll range: the whitespace-collapsed text of every match. */
export async function collectTexts(page: Page, selector: string): Promise<string[]> {
  const rows = await collectRows(page, selector, (node) => ({ text: (node.textContent ?? '').replace(/\s+/g, ' ').trim() }));
  return rows.map((row) => row.text);
}

/**
 * The content keys of every chip of the editor over the whole scroll range, one per chip (see
 * `chipKeys`): the link id, the text of its line around it, and its occurrence on the line. The
 * line is the nearest block ancestor; other chips of it read as their link id.
 */
export async function collectChipKeys(page: Page): Promise<string[]> {
  const rows = await collectWhileScrolling(
    page,
    () =>
      page.evaluate(
        ([keysSource]) => {
          const keysOf = (0, eval)(keysSource) as (parts: Array<string | { chip: string }>) => string[];
          const host = document.getElementById('editor-host')!;
          const lines = new Map<Element, Element[]>();
          for (const chip of host.querySelectorAll('.tu-link')) {
            let line: Element = chip.parentElement ?? host;
            while (line !== host && getComputedStyle(line).display.startsWith('inline')) line = line.parentElement ?? host;
            lines.set(line, [...(lines.get(line) ?? []), chip]);
          }
          const keys: string[] = [];
          for (const line of lines.keys()) {
            const parts: Array<string | { chip: string }> = [];
            const walk = (node: Node): void => {
              if (node.nodeType === Node.TEXT_NODE) parts.push((node.textContent ?? '').replace(/\s+/g, ' '));
              else if (node instanceof Element && node.classList.contains('tu-link')) parts.push({ chip: node.getAttribute('data-link-id') ?? '' });
              else node.childNodes.forEach(walk);
            };
            walk(line);
            keys.push(...keysOf(parts));
          }
          return keys.map((key) => ({ key }));
        },
        [inPage(chipKeys)] as const,
      ),
    (row) => row.key,
  );
  return rows.map((row) => row.key);
}
