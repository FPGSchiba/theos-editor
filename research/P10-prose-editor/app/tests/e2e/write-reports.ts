import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { arch, cpus as cpuList, release, totalmem, type } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium, webkit } from '@playwright/test';
import { RAW_DIR, RECORD_KINDS, type RecordKind } from './record.ts';

/** `research/P10-prose-editor/report/`, committed by the run that writes it (spec §4.6–4.8). */
const REPORT_DIR = fileURLToPath(new URL('../../../report/', import.meta.url));
const APP_DIR = fileURLToPath(new URL('../../', import.meta.url));

type Row = Record<string, unknown>;

/**
 * One row of `round-trip.json` (written by round-trip.spec.ts): the bytes an editor variant saved
 * for one file and one check, compared with the oracle's. `check` is `unedited` (driver `load`)
 * or the id of a manifest edit (driver `api` or `input`). `level` is `span` when the bytes are
 * identical, `block` when the mismatch stays inside the edited block, else `none`.
 */
export interface RoundTripRow {
  variant: string;
  engine: string;
  file: string;
  check: string;
  driver: 'load' | 'api' | 'input';
  identical: boolean;
  firstDiff?: { line: number; column: number };
  extraBytes: number;
  withinBlock?: boolean;
  level: 'span' | 'block' | 'none';
  /** What the adapter said about its own save (`lastSaveLevel`), when it says anything. */
  adapterLevel?: 'span' | 'block';
  hunk?: string;
  /** A harness error (exception, timeout, a feature the variant does not render); no comparison was made. */
  error?: string;
  /** `external` only: which comparison failed first, the one after the edit or the one after one undo. */
  stage?: 'edit' | 'undo';
}

/**
 * One row of `features.json`, `input.json` or `integration.json` (written by features.spec.ts,
 * input.spec.ts and integration.spec.ts): whether one variant passed one check in one engine.
 * `detail` says what failed (or, for a pass, what the check saw); `error` is a harness error
 * (exception, timeout), which means the check could not run. A `detail` that starts with `gap:`
 * names a gap of the tooling that no variant can close (WebKit has no composition input).
 */
export interface CheckRow {
  variant: string;
  engine: string;
  check: string;
  ok: boolean;
  detail?: string;
  error?: string;
}

/** Latency of one typing position: median and 95th percentile in ms over `n` keystrokes. */
export interface Latency {
  median: number;
  p95: number;
  n: number;
}

/**
 * One row of `long-docs.json` (written by long-docs.spec.ts): the numbers of one variant and
 * engine on one generated file. `size` is the target size in bytes (`generateLong`), `bytes` the
 * UTF-8 size of the text. Every measure is optional because a row with an `error` keeps what
 * was measured before the error. `heapBytes` is Chromium only.
 */
export interface LongDocRow {
  variant: string;
  engine: string;
  size: number;
  label: string;
  bytes: number;
  /** From `load()` to the second animation frame after it. */
  loadMs?: number;
  /** Keydown (`event.timeStamp`) to the next animation frame after the editor's DOM changed. */
  typing?: { start: Latency; middle: Latency; end: Latency };
  /** From jumping to the end of the document to the last line being drawn. */
  scrollMs?: number;
  scrollReachedEnd?: boolean;
  heapBytes?: number;
  error?: string;
}

export interface ReportHeader {
  commit: string;
  os: string;
  browsers: { chromium: string; webkit: string };
  date: string;
}

/** The Markdown body of a kind's report; later suites register theirs here. */
export const sections: Partial<Record<RecordKind, (rows: Row[]) => string>> = {
  // The rows were written by the spec of their kind, which types them as the interface above.
  'round-trip': (rows) => roundTripSection(rows as unknown as RoundTripRow[]),
  features: (rows) => checkSection(rows as unknown as CheckRow[], 'a rendered feature of the editor'),
  input: (rows) => checkSection(rows as unknown as CheckRow[], 'spell check, IME or dead-key input'),
  integration: (rows) => checkSection(rows as unknown as CheckRow[], 'how the editor fits the app'),
  'long-docs': (rows) => longDocsSection(rows as unknown as LongDocRow[]),
};

/** The default body: how many rows each variant and engine produced, and how many carry an error. */
function summary(rows: Row[]): string {
  const counts = new Map<string, { rows: number; errors: number }>();
  for (const row of rows) {
    const key = `${String(row.variant ?? '-')} | ${String(row.engine ?? '-')}`;
    const entry = counts.get(key) ?? { rows: 0, errors: 0 };
    entry.rows += 1;
    if (row.error !== undefined) entry.errors += 1;
    counts.set(key, entry);
  }
  const lines = ['| variant | engine | rows | errors |', '|---|---|---|---|'];
  for (const [key, entry] of [...counts].sort(([a], [b]) => a.localeCompare(b))) {
    const [variant, engine] = key.split(' | ');
    lines.push(`| ${variant} | ${engine} | ${entry.rows} | ${entry.errors} |`);
  }
  return `${lines.join('\n')}\n\nEvery row is in the JSON file next to this one.\n`;
}

/** A backtick fence longer than any backtick run inside `text`, so a hunk that shows a code fence cannot end it. */
function fenceFor(text: string): string {
  const longest = Math.max(0, ...(text.match(/`+/g) ?? []).map((run) => run.length));
  return '`'.repeat(Math.max(3, longest + 1));
}

const share = (matching: number, total: number): string => `${matching}/${total}`;

/** One entry per mismatch (or error): where it is, how big, which level, and the diff. */
function roundTripEntry(row: RoundTripRow): string {
  const lines = [`#### \`${row.file}\` · \`${row.check}\` · ${row.driver}`, ''];
  if (row.error !== undefined) {
    lines.push(`- error: ${row.error}`);
  } else {
    if (row.firstDiff !== undefined) lines.push(`- position: ${row.firstDiff.line}:${row.firstDiff.column}`);
    lines.push(`- extra bytes: ${row.extraBytes}`);
    lines.push(`- level: ${row.level}${row.adapterLevel === undefined ? '' : ` (the adapter says: ${row.adapterLevel})`}`);
  }
  if (row.stage !== undefined) lines.push(`- failed after: ${row.stage === 'edit' ? 'the edit' : 'one undo'}`);
  if (row.hunk !== undefined) {
    const hunk = row.hunk.replace(/\n+$/, '');
    const fence = fenceFor(hunk);
    lines.push('', `${fence}diff`, hunk, fence);
  }
  return lines.join('\n');
}

/**
 * The Markdown body of `round-trip.md`: per variant and engine how many unedited files came back
 * identical and how many edits matched the oracle (all, and by driver), then every mismatch.
 */
export function roundTripSection(rows: RoundTripRow[]): string {
  const groups = new Map<string, RoundTripRow[]>();
  for (const row of rows) {
    const key = `${row.variant}\u0000${row.engine}`;
    groups.set(key, [...(groups.get(key) ?? []), row]);
  }
  const ordered = [...groups].sort(([a], [b]) => a.localeCompare(b));

  const table = [
    '| variant | engine | unedited identical | edits matching | via api | via input | errors |',
    '|---|---|---|---|---|---|---|',
  ];
  const mismatches: string[] = [];
  for (const [key, group] of ordered) {
    const [variant = '', engine = ''] = key.split('\u0000');
    const unedited = group.filter((row) => row.check === 'unedited');
    const edits = group.filter((row) => row.check !== 'unedited');
    const count = (list: RoundTripRow[]): string => share(list.filter((row) => row.identical).length, list.length);
    const byDriver = (driver: RoundTripRow['driver']): RoundTripRow[] => edits.filter((row) => row.driver === driver);
    table.push(
      `| ${variant} | ${engine} | ${count(unedited)} | ${count(edits)} | ${count(byDriver('api'))} | ${count(byDriver('input'))} | ${group.filter((row) => row.error !== undefined).length} |`,
    );
    const wrong = group.filter((row) => !row.identical);
    if (wrong.length > 0) {
      mismatches.push(`### ${variant} · ${engine}`, '', ...wrong.flatMap((row) => [roundTripEntry(row), '']));
    }
  }

  return [
    '`level` is `span` when the saved bytes are identical to the oracle, `block` when the mismatch stays',
    'inside the edited block, and `none` otherwise. A row with an error ran no comparison.',
    '',
    '## Summary',
    '',
    ...table,
    '',
    '## Mismatches',
    '',
    ...(mismatches.length === 0 ? ['No mismatches.', ''] : mismatches),
    'Every row is in the JSON file next to this one.',
    '',
  ].join('\n');
}

const isGap = (row: CheckRow): boolean => !row.ok && row.error === undefined && (row.detail ?? '').startsWith('gap:');

/** The verdict of one row in a matrix cell: `ok`, `FAIL`, `error` (it could not run) or `gap`. */
function verdictOf(row: CheckRow): string {
  if (row.error !== undefined) return 'error';
  if (row.ok) return 'ok';
  return isGap(row) ? 'gap' : 'FAIL';
}

/**
 * The Markdown body of `features.md`, `input.md` and `integration.md`: per variant and engine how
 * many checks passed and failed (a failed one that could not run counts as an error, a tooling
 * gap as a gap), a matrix of every check, and one line per failed check with its detail.
 * `what` names the kind of check for the legend.
 */
export function checkSection(rows: CheckRow[], what: string): string {
  const groups = new Map<string, CheckRow[]>();
  for (const row of rows) {
    const key = `${row.variant}\u0000${row.engine}`;
    groups.set(key, [...(groups.get(key) ?? []), row]);
  }
  const ordered = [...groups].sort(([a], [b]) => a.localeCompare(b));

  const summary = ['| variant | engine | checks | ok | failed | errors | gaps |', '|---|---|---|---|---|---|---|'];
  const failures: string[] = [];
  for (const [key, group] of ordered) {
    const [variant = '', engine = ''] = key.split('\u0000');
    const failed = group.filter((row) => !row.ok);
    summary.push(
      `| ${variant} | ${engine} | ${group.length} | ${group.length - failed.length} | ${failed.length} | ${group.filter((row) => row.error !== undefined).length} | ${group.filter(isGap).length} |`,
    );
    if (failed.length > 0) {
      failures.push(
        `### ${variant} · ${engine}`,
        '',
        ...failed
          .sort((a, b) => a.check.localeCompare(b.check))
          .map((row) => `- \`${row.check}\`: ${row.error === undefined ? (row.detail ?? 'failed') : `error: ${row.error}`}`),
        '',
      );
    }
  }

  const columns = ordered.map(([key]) => key.split('\u0000').join(' · '));
  const checks = [...new Set(rows.map((row) => row.check))].sort((a, b) => a.localeCompare(b));
  const matrix = [`| check | ${columns.join(' | ')} |`, `|---|${columns.map(() => '---').join('|')}|`];
  for (const check of checks) {
    const cells = ordered.map(([, group]) => {
      const found = group.find((row) => row.check === check);
      return found === undefined ? '-' : verdictOf(found);
    });
    matrix.push(`| \`${check}\` | ${cells.join(' | ')} |`);
  }

  return [
    `Checks of ${what}. A check is \`ok\` or \`FAIL\`; \`error\` means it could not run (the error is listed),`,
    'and `gap` is a gap of the tooling that no variant can close. A failed check is a result, not a test failure.',
    '',
    '## Summary',
    '',
    ...summary,
    '',
    '## Checks',
    '',
    ...matrix,
    '',
    '## Failed checks',
    '',
    ...(failures.length === 0 ? ['No failed checks.', ''] : failures),
    'Every row is in the JSON file next to this one.',
    '',
  ].join('\n');
}

/** The machine the numbers of `long-docs` come from, for the report (spec §4.8). */
export function describeMachine(): string {
  const cpus = cpuList();
  const model = cpus[0]?.model.replace(/\s+/g, ' ').trim() ?? 'unknown CPU';
  return `${model}, ${cpus.length} logical cores, ${(totalmem() / 2 ** 30).toFixed(1)} GiB RAM`;
}

const ms = (value: number | undefined): string => (value === undefined ? '-' : value.toFixed(value < 10 ? 1 : 0));
const latency = (value: Latency | undefined): string =>
  value === undefined ? '-' : `${ms(value.median)} / ${ms(value.p95)}${value.n < 200 ? ` (n=${value.n})` : ''}`;
const sizeLabel = (row: LongDocRow): string =>
  `${row.label} (${row.bytes >= 1_000_000 ? `${(row.bytes / 1_000_000).toFixed(1)} MB` : `${Math.round(row.bytes / 1000)} KB`})`;

/**
 * The Markdown body of `long-docs.md`: one table row per variant, engine and file with the load
 * time, the typing latency (median / p95 in ms) at the start, middle and end, the scroll time and
 * the JS heap (Chromium only), then the errors.
 */
export function longDocsSection(rows: LongDocRow[], machine: string = describeMachine()): string {
  const ordered = [...rows].sort(
    (a, b) => a.variant.localeCompare(b.variant) || a.engine.localeCompare(b.engine) || a.size - b.size,
  );
  const table = [
    '| variant | engine | file | load ms | typing start (median / p95 ms) | typing middle | typing end | scroll ms | heap MB |',
    '|---|---|---|---|---|---|---|---|---|',
  ];
  for (const row of ordered) {
    const scroll = row.scrollMs === undefined ? '-' : `${row.scrollReachedEnd === false ? '>' : ''}${ms(row.scrollMs)}`;
    const heap = row.heapBytes === undefined ? '-' : (row.heapBytes / 1_000_000).toFixed(1);
    table.push(
      `| ${row.variant} | ${row.engine} | ${sizeLabel(row)} | ${ms(row.loadMs)} | ${latency(row.typing?.start)} | ${latency(row.typing?.middle)} | ${latency(row.typing?.end)} | ${scroll} | ${heap} |`,
    );
  }
  const errors = ordered.filter((row) => row.error !== undefined);

  return [
    `Machine: ${machine}.`,
    '',
    'Load is the time from `load()` to the second animation frame after it. Typing is the time from the',
    "keydown's `timeStamp` to the next animation frame after the editor's DOM changed (a form field's",
    '`input` event counts as a change), over 200 keystrokes typed at the start, middle and end of the',
    'prose, each sent to an idle page after a pause of 0 to 16 ms. It stops before that frame is laid out',
    'and painted, so it is a lower bound, and the headless browsers draw a frame on demand, so a light',
    'editor measures about 1 ms here, not the interval of a 60 Hz display. Scroll is the time from',
    'jumping to the end of the document until its last line is drawn (`>` when it never was within the',
    'limit). Heap is `performance.memory.usedJSHeapSize` after a garbage collection, Chromium only. The',
    'numbers are evidence, not a gate (spec §4.8); a headless run on a loaded machine is slower than the app.',
    '',
    '## Results',
    '',
    ...table,
    '',
    '## Errors',
    '',
    ...(errors.length === 0
      ? ['No errors.', '']
      : [...errors.map((row) => `- ${row.variant} · ${row.engine} · ${sizeLabel(row)}: ${row.error ?? ''}`), '']),
    'Every row is in the JSON file next to this one.',
    '',
  ].join('\n');
}

function readRows(kind: RecordKind): Row[] {
  if (!existsSync(RAW_DIR)) return [];
  const rows: Row[] = [];
  for (const name of readdirSync(RAW_DIR).filter((n) => n.startsWith(`${kind}-`) && n.endsWith('.jsonl')).sort()) {
    for (const line of readFileSync(join(RAW_DIR, name), 'utf8').split('\n')) {
      if (line.trim() !== '') rows.push(JSON.parse(line) as Row);
    }
  }
  // The workers finish in any order; the report must not depend on it.
  const key = (row: Row) => ['variant', 'engine', 'file', 'size', 'check', 'driver'].map((k) => String(row[k] ?? '')).join('\u0000');
  return rows.sort((a, b) => key(a).localeCompare(key(b)));
}

async function browserVersion(launcher: typeof chromium): Promise<string> {
  try {
    const browser = await launcher.launch();
    const version = browser.version();
    await browser.close();
    return version;
  } catch (error) {
    return `unavailable (${error instanceof Error ? error.message.split('\n')[0] : String(error)})`;
  }
}

async function header(): Promise<ReportHeader> {
  return {
    commit: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: APP_DIR, encoding: 'utf8' }).trim(),
    os: `${type()} ${release()} ${arch()}`,
    browsers: { chromium: await browserVersion(chromium), webkit: await browserVersion(webkit) },
    date: new Date().toISOString(),
  };
}

function markdown(kind: RecordKind, head: ReportHeader, rows: Row[]): string {
  const body = (sections[kind] ?? summary)(rows);
  return [
    `# ${kind} report`,
    '',
    `- commit: \`${head.commit}\``,
    `- os: ${head.os}`,
    `- chromium: ${head.browsers.chromium}`,
    `- webkit: ${head.browsers.webkit}`,
    `- date: ${head.date}`,
    '',
    body,
  ].join('\n');
}

/**
 * Playwright's global teardown, only with `P10_REPORT=1`: per kind, `report/<kind>.json` and
 * `report/<kind>.md` from the rows of this run. A kind without rows leaves its old report alone.
 */
export default async function writeReports(): Promise<void> {
  if (process.env.P10_REPORT !== '1') return;
  const head = await header();
  mkdirSync(REPORT_DIR, { recursive: true });
  for (const kind of RECORD_KINDS) {
    const rows = readRows(kind);
    if (rows.length === 0) {
      console.warn(`write-reports: no ${kind} rows in this run; report/${kind}.* left as it is`);
      continue;
    }
    writeFileSync(join(REPORT_DIR, `${kind}.json`), `${JSON.stringify({ ...head, kind, rows }, null, 2)}\n`);
    writeFileSync(join(REPORT_DIR, `${kind}.md`), markdown(kind, head, rows));
  }
}
