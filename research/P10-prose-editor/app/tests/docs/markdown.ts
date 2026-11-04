/**
 * Readers for the research documents (comparison.md, the decision record): sections, tables, the
 * source list and the score cells of the comparison. They parse the fixed formats of the plan
 * (Task 4) and throw on a malformed source list, so a document check fails instead of skipping
 * an entry it cannot read.
 */

/** The text that marks a source as a researcher probe (supporting evidence only). */
export const PROBE_MARKER = 'researcher probe, not reproduced; supporting evidence only';

/**
 * The text of the `## heading` section, from the line after the heading up to the next `#` or
 * `##` heading (`###` subsections stay inside); undefined if the document has no such section.
 */
export function section(md: string, heading: string): string | undefined {
  const lines = md.replace(/\r\n/g, '\n').split('\n');
  const start = lines.findIndex((line) => line === `## ${heading}`);
  if (start === -1) return undefined;
  const rest = lines.slice(start + 1);
  const end = rest.findIndex((line) => /^##? /.test(line));
  return (end === -1 ? rest : rest.slice(0, end)).join('\n');
}

/**
 * The first table of the `## heading` section as rows of trimmed cells: the header row first,
 * then the data rows; the delimiter row (`|---|`) is left out. `\|` inside a cell is a literal
 * pipe. Throws when the section or its table is missing.
 */
export function parseTable(md: string, heading: string): string[][] {
  const body = section(md, heading);
  if (body === undefined) throw new Error(`the document has no "## ${heading}" section`);
  const rows = tableRows(body);
  if (rows.length === 0) throw new Error(`the "## ${heading}" section has no table`);
  return rows;
}

/**
 * The first table of a text as rows of trimmed cells, in the form `parseTable` reads; an empty
 * list when the text has no table.
 */
export function tableRows(text: string): string[][] {
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  const first = lines.findIndex((line) => line.startsWith('|'));
  if (first === -1) return [];
  const rows: string[][] = [];
  for (const line of lines.slice(first)) {
    if (!line.startsWith('|')) break;
    const inner = line.trim().replace(/^\|/, '').replace(/(?<!\\)\|$/, '');
    const cells = inner.split(/(?<!\\)\|/).map((cell) => cell.trim().replace(/\\\|/g, '|'));
    if (cells.every((cell) => /^:?-+:?$/.test(cell))) continue;
    rows.push(cells);
  }
  return rows;
}

const SOURCE_LINE = /^- (S\d+) · (.+) · read (\d{4}-\d{2}-\d{2})$/;

/** The list lines of the `## Sources` section (lines that start with `- `). */
function sourceLines(md: string): string[] {
  return (section(md, 'Sources') ?? '').split('\n').filter((line) => line.startsWith('- '));
}

/**
 * The `## Sources` list: `- S<n> · <url or repo path> · <title> · read <YYYY-MM-DD>`, keyed by
 * `S<n>`. `ref` is the URL or repo path, `date` the read date. Throws on a list line that does
 * not have this form and on an id listed twice.
 */
export function parseSources(md: string): Map<string, { ref: string; date: string }> {
  const sources = new Map<string, { ref: string; date: string }>();
  for (const line of sourceLines(md)) {
    const match = SOURCE_LINE.exec(line);
    const [id, middle, date] = [match?.[1], match?.[2], match?.[3]];
    const fields = middle?.split(' · ') ?? [];
    const ref = fields[0]?.trim() ?? '';
    if (id === undefined || date === undefined || fields.length < 2 || ref === '') {
      throw new Error(`malformed source line: ${line}`);
    }
    if (sources.has(id)) throw new Error(`${id} is listed twice`);
    sources.set(id, { ref, date });
  }
  return sources;
}

/** The ids of the sources whose entry carries the probe marker. */
export function probeSources(md: string): Set<string> {
  const ids = new Set<string>();
  for (const line of sourceLines(md)) {
    const id = SOURCE_LINE.exec(line)?.[1];
    if (id !== undefined && line.includes(PROBE_MARKER)) ids.add(id);
  }
  return ids;
}

/** A score cell of the comparison matrix. */
export interface Cell {
  /** The paper score: `0`–`3`, or `pass`/`fail` for the licence. */
  paper: string;
  /** The measured score of Task 18 (`<paper>→<measured>`), if the cell has one. */
  measured?: string;
  /** The cited source ids, in order. */
  sources: string[];
}

const CELL = /^(\d|pass|fail)(?:→(\d|pass|fail))? \[(S\d+(?:, S\d+)*)\]$/;

/** Parses `<score> [S…]` or `<paper>→<measured> [S…]`; undefined if the cell has another form. */
export function parseCell(cell: string): Cell | undefined {
  const match = CELL.exec(cell);
  const [paper, measured, list] = [match?.[1], match?.[2], match?.[3]];
  if (paper === undefined || list === undefined) return undefined;
  return measured === undefined ? { paper, sources: list.split(', ') } : { paper, measured, sources: list.split(', ') };
}

/** Every source id cited anywhere in the text, in brackets like `[S1]` or `[S1, S4]`. */
export function citedSources(text: string): Set<string> {
  const ids = new Set<string>();
  for (const group of text.matchAll(/\[(S\d+(?:, S\d+)*)\]/g)) {
    for (const id of group[1]?.split(', ') ?? []) ids.add(id);
  }
  return ids;
}
