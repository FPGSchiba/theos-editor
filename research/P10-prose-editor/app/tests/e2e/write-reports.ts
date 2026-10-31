import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { arch, release, type } from 'node:os';
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

export interface ReportHeader {
  commit: string;
  os: string;
  browsers: { chromium: string; webkit: string };
  date: string;
}

/** The Markdown body of a kind's report; later suites register theirs here. */
export const sections: Partial<Record<RecordKind, (rows: Row[]) => string>> = {
  // The rows were written by round-trip.spec.ts, which types them as RoundTripRow.
  'round-trip': (rows) => roundTripSection(rows as unknown as RoundTripRow[]),
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
