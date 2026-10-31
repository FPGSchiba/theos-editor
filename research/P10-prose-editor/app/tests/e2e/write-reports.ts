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

export interface ReportHeader {
  commit: string;
  os: string;
  browsers: { chromium: string; webkit: string };
  date: string;
}

/** The Markdown body of a kind's report; later suites register theirs here. */
export const sections: Partial<Record<RecordKind, (rows: Row[]) => string>> = {};

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
