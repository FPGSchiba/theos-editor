import { execFileSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { expect, test, type Page } from '@playwright/test';
import { splitFrontMatter } from '../../src/harness/frontmatter.ts';
import { corpusFile, findScrollers, inPage } from './checks.ts';
import { loadFile, record, strict, variantsUnderTest } from './helpers.ts';
import type { Latency, LongDocRow } from './write-reports.ts';

// Open, type and scroll the three long documents of spec §4.8 (CE-06.1 criterion 7): about 5 KB,
// 250 KB and 2.5 MB, generated from one fixed block (`generateLong`, no files needed). Per
// variant, engine and size: the time from load to the second animation frame, the latency of 200
// keystrokes at the start, middle and end, the time to scroll to the end, and, in Chromium, the
// JS heap. The numbers are evidence, not a gate: a row only fails a test under P10_STRICT=1, and
// then only when a stage could not run. For a measurement that means something run this suite
// alone with one worker; other suites in parallel compete for the CPU.

const SIZES = [
  { label: 'typical', size: 5_000 },
  { label: 'large', size: 250_000 },
  { label: 'stress', size: 2_500_000 },
] as const;

/** Keystrokes per position, and the letters they type. */
const KEYSTROKES = 200;
const LETTERS = 'abcdefghijklmnopqrstuvwxyz';
/** A file of its own id: the mock findings and index only know the corpus files. */
const LONG_FILE_ID = 'records/story/scripts/S-999.md';
/** The words the last line of every generated file ends with; scrolling is done when they are drawn. */
const LAST_LINE = 'a saying on the shrine stair';
/** How long the editor may take to draw the end of the document after a jump to it. */
const SCROLL_LIMIT_MS = 10_000;
/** A keystroke that has not led to a DOM change and a frame after this long is given up on. */
const SAMPLE_LIMIT_MS = 2_000;

// Generating the 2.5 MB text takes a moment; each size is built once per worker.
const GENERATOR = pathToFileURL(fileURLToPath(new URL('../../../corpus/generate-long.ts', import.meta.url))).href;
const generated = new Map<number, string>();

/**
 * The text of `generateLong(size)`. Playwright loads TypeScript outside this package as
 * CommonJS, which cannot import the generator (an ES module), so plain node runs it, as its
 * header says; the text comes back on stdout and no file is written.
 */
function longText(size: number): string {
  let text = generated.get(size);
  if (text === undefined) {
    const code = `import { generateLong } from ${JSON.stringify(GENERATOR)}; process.stdout.write(generateLong(${size}));`;
    text = execFileSync(process.execPath, ['--input-type=module', '--eval', code], {
      encoding: 'utf8',
      maxBuffer: 16 * 2 ** 20,
    });
    generated.set(size, text);
  }
  return text;
}

/** The three places the typing happens, in the order they are typed (see `positions`). */
const PLACES = ['end', 'middle', 'start'] as const;

/**
 * Where the typing happens, as offsets into the text: inside the intro paragraph (start), the
 * paragraph of a beat near the middle of the file (middle), and the paragraph of the last beat
 * (end), each in plain prose. The offsets are into the original text, so the places are typed
 * from the end of the file backwards (`PLACES`): what is typed at one place never moves a place
 * that is still to come. Each offset is checked to lie in a prose line (not a heading, an
 * attribute line, a callout or the front matter).
 */
function positions(text: string): Record<(typeof PLACES)[number], number> {
  const prose = 'The stair is crowded';
  const middle = text.indexOf(prose, Math.floor(text.length / 2));
  const end = text.lastIndexOf(prose);
  const start = text.indexOf('Mother Ilse welcomes');
  if (start === -1 || middle === -1 || end === -1) throw new Error('long-docs: the generated text has no prose to type in');
  const found = { start: start + 6, middle: middle + 12, end: end + 12 };
  const bodyStart = splitFrontMatter(text).frontMatter.length;
  for (const place of PLACES) {
    const offset = found[place];
    const line = text.slice(text.lastIndexOf('\n', offset - 1) + 1, text.indexOf('\n', offset));
    if (offset < bodyStart || /^(>|#|[a-z]+::)/.test(line)) {
      throw new Error(`long-docs: the ${place} position is not in a prose line: ${JSON.stringify(line.slice(0, 40))}`);
    }
  }
  return found;
}

function summarise(samples: number[]): Latency {
  const sorted = [...samples].sort((a, b) => a - b);
  const n = sorted.length;
  if (n === 0) throw new Error('long-docs: no keystroke led to a DOM change and a frame');
  const middle = n % 2 === 1 ? sorted[(n - 1) / 2]! : (sorted[n / 2 - 1]! + sorted[n / 2]!) / 2;
  const p95 = sorted[Math.min(n - 1, Math.ceil(0.95 * n) - 1)]!;
  const tenth = (value: number): number => Math.round(value * 10) / 10;
  return { median: tenth(middle), p95: tenth(p95), n };
}

// ── In-page instruments ─────────────────────────────────────────────────────

/** The samples of one position: per keystroke, in ms since its keydown, in the page's own clock. */
interface Samples {
  /** Until the main thread has styled, laid out and painted the frame after the change. */
  paint: number[];
  /** Until the animation frame callback of that frame, which runs before style, layout and paint. */
  frame: number[];
}

/** The probe that lives in the page. */
interface Probe {
  reset(): void;
  waitFor(count: number, limitMs: number): Promise<number>;
  take(): Samples;
}

/**
 * Install the probe. A keydown stores `event.timeStamp`; when the editor's DOM changes (a
 * MutationObserver) or a form field reports an `input`, the next animation frame ends the
 * measurement of every keydown that was waiting. That callback runs before the frame's style,
 * layout and paint, so it only gives the `frame` sample; it posts a message on a MessageChannel,
 * a task that runs after the frame's main-thread work, and the `paint` sample is taken there.
 */
function installProbe(page: Page): Promise<void> {
  return page.evaluate(() => {
    const host = document.getElementById('editor-host')!;
    let samples: Samples = { paint: [], frame: [] };
    let waiting: number[] = [];
    // The frames whose message has not arrived yet, oldest first (messages arrive in order).
    const posted: Array<{ starts: number[]; frame: number }> = [];
    const channel = new MessageChannel();
    channel.port1.onmessage = () => {
      const paint = performance.now();
      const done = posted.shift();
      if (done === undefined) return;
      for (const start of done.starts) {
        samples.paint.push(paint - start);
        samples.frame.push(done.frame - start);
      }
    };
    const changed = (): void => {
      if (waiting.length === 0) return;
      const starts = waiting;
      waiting = [];
      requestAnimationFrame(() => {
        posted.push({ starts, frame: performance.now() });
        channel.port2.postMessage(0);
      });
    };
    document.addEventListener(
      'keydown',
      (event) => {
        // Some engines stamp events with the epoch, not the page's clock.
        waiting.push(event.timeStamp > 1e11 ? event.timeStamp - performance.timeOrigin : event.timeStamp);
      },
      true,
    );
    new MutationObserver(changed).observe(host, { subtree: true, childList: true, characterData: true });
    host.addEventListener('input', changed, true);
    const probe: Probe = {
      reset() {
        samples = { paint: [], frame: [] };
        waiting = [];
        posted.length = 0;
      },
      // Resolves with the number of finished samples once there are `count`, or after the limit.
      waitFor(count, limitMs) {
        return new Promise((resolve) => {
          const started = performance.now();
          const tick = (): void => {
            if (samples.paint.length >= count || performance.now() - started > limitMs) resolve(samples.paint.length);
            else requestAnimationFrame(tick);
          };
          tick();
        });
      },
      take: () => ({ paint: [...samples.paint], frame: [...samples.frame] }),
    };
    Object.assign(window, { p10Probe: probe });
  });
}

const probe = {
  reset: (page: Page) => page.evaluate(() => (window as unknown as { p10Probe: Probe }).p10Probe.reset()),
  waitFor: (page: Page, count: number) =>
    page.evaluate(([n, limit]) => (window as unknown as { p10Probe: Probe }).p10Probe.waitFor(n, limit), [count, SAMPLE_LIMIT_MS] as const),
  take: (page: Page) => page.evaluate(() => (window as unknown as { p10Probe: Probe }).p10Probe.take()),
};

/** The letters `typeAt` types: `KEYSTROKES` of them, cycling through `LETTERS`. */
const TYPED = Array.from({ length: KEYSTROKES }, (_, key) => LETTERS[key % LETTERS.length]).join('');

/**
 * Type `KEYSTROKES` letters at `offset`, one at a time, and summarise the latency of each
 * (`paint`: until the frame after the change is styled, laid out and painted; `frame`: until its
 * animation frame callback). The letters must end up at `offset`, else the position was not the
 * one the report names and the sample is no use.
 */
async function typeAt(page: Page, offset: number): Promise<{ paint: Latency; frame: Latency }> {
  await page.evaluate((at) => {
    const harness = window.__harness!;
    harness.focus();
    harness.caretTo(at);
  }, offset);
  await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
  await probe.reset(page);
  for (let key = 0; key < KEYSTROKES; key++) {
    // Each keystroke waits for its frame before the next one is sent, so without a pause every
    // keydown would land right after a frame and show the same frame interval. A fixed pattern of
    // 0 to 16 ms spreads them over the frame, like a person typing, and keeps runs comparable.
    await page.waitForTimeout((key * 7) % 17);
    await page.keyboard.press(LETTERS[key % LETTERS.length]!);
    const seen = await probe.waitFor(page, key + 1);
    // An editor that never changes its DOM would cost the limit per keystroke; stop early.
    if (key === 4 && seen === 0) throw new Error('long-docs: five keystrokes led to no DOM change and no frame');
  }
  await probe.waitFor(page, KEYSTROKES);
  const landed = await page.evaluate(([at, typed]) => window.__harness!.text().startsWith(typed, at), [offset, TYPED] as const);
  if (!landed) throw new Error(`long-docs: the typed letters are not at offset ${offset}`);
  const samples = await probe.take(page);
  return { paint: summarise(samples.paint), frame: summarise(samples.frame) };
}

/**
 * Jump to the end of the document and wait until its last line is drawn. The scrollers are found by
 * `findScrollers` (checks.ts).
 */
function scrollToEnd(page: Page): Promise<{ ms: number; reached: boolean }> {
  return page.evaluate(
    async ([marker, limit, finder]) => {
      const host = document.getElementById('editor-host')!;
      window.__harness!.focus();
      const scrollers = new Set<Element>((0, eval)(finder)() as Element[]);
      const atBottom = (node: Element): boolean => node.scrollTop + node.clientHeight >= node.scrollHeight - 2;
      const frame = (): Promise<void> => new Promise((resolve) => requestAnimationFrame(() => resolve()));

      const started = performance.now();
      for (;;) {
        // The height may still grow while the editor measures what it has drawn.
        for (const node of scrollers) node.scrollTop = node.scrollHeight;
        await frame();
        const drawn = host.querySelector('textarea') !== null || (host.textContent ?? '').includes(marker);
        if (drawn && [...scrollers].every(atBottom)) return { ms: performance.now() - started, reached: true };
        if (performance.now() - started > limit) return { ms: performance.now() - started, reached: false };
      }
    },
    [LAST_LINE, SCROLL_LIMIT_MS, inPage(findScrollers)] as const,
  );
}

/** Used JS heap after a garbage collection (Chromium; the collection goes through CDP). */
async function heapBytes(page: Page): Promise<number | undefined> {
  const client = await page.context().newCDPSession(page);
  try {
    await client.send('HeapProfiler.collectGarbage');
  } catch {
    // The reading is still taken, just without the collection.
  } finally {
    await client.detach().catch(() => undefined);
  }
  return page.evaluate(() => (performance as unknown as { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize);
}

// ── The suite ───────────────────────────────────────────────────────────────

// One measurement at a time per engine; the files differ by two orders of magnitude.
test.describe.configure({ mode: 'serial' });

for (const variant of variantsUnderTest()) {
  for (const { label, size } of SIZES) {
    test(`@long ${label} · ${variant.name}`, async ({ page, browserName }) => {
      test.setTimeout(15 * 60_000);
      const text = longText(size);
      const row: LongDocRow = { variant: variant.name, engine: browserName, size, label, bytes: Buffer.byteLength(text) };
      try {
        // A small file first, so the page and the editor code are warm: this measures opening a
        // file in a running app, not starting one.
        await loadFile(page, variant.name, corpusFile('records/world/hearth/f04-choir-hall.md'));

        row.loadMs = await page.evaluate(
          async ([name, content, fileId]) => {
            const started = performance.now();
            await window.__harness!.load(name, content, fileId);
            await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
            return performance.now() - started;
          },
          [variant.name, text, LONG_FILE_ID] as const,
        );
        row.loadMs = Math.round(row.loadMs * 10) / 10;

        const scrolled = await scrollToEnd(page);
        row.scrollMs = Math.round(scrolled.ms * 10) / 10;
        row.scrollReachedEnd = scrolled.reached;

        await installProbe(page);
        const where = positions(text);
        // From the end backwards, so that typing at one place does not move the next one.
        const typed = {} as Record<(typeof PLACES)[number], { paint: Latency; frame: Latency }>;
        for (const place of PLACES) typed[place] = await typeAt(page, where[place]);
        row.typing = { start: typed.start.paint, middle: typed.middle.paint, end: typed.end.paint };
        row.typingFrame = { start: typed.start.frame, middle: typed.middle.frame, end: typed.end.frame };

        if (browserName === 'chromium') {
          const heap = await heapBytes(page);
          if (heap !== undefined) row.heapBytes = heap;
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        row.error = (message.split('\n')[0] ?? message).slice(0, 300);
      }
      record('long-docs', row);
      if (strict()) expect(row.error, `${label} could not be measured`).toBeUndefined();
    });
  }
}
