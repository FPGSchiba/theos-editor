import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { citedSources, parseCell, parseSources, parseTable, probeSources, section } from './markdown.ts';
import type { Cell } from './markdown.ts';

/**
 * Document checks for the comparison of CE-06.1 (spec §4.3, §4.4; plan Task 4). They check the
 * form and the arithmetic, not the truth of a score; the evidence check (spec §4.10) covers that.
 */

const COMPARISON = fileURLToPath(new URL('../../../comparison.md', import.meta.url));

const CANDIDATES = ['CodeMirror 6', 'ProseMirror/Tiptap', 'Milkdown', 'Lexical', 'Monaco'] as const;

/** The criteria of spec §4.3 in their order, with the weight column as written. */
const CRITERIA = [
  { name: 'Byte-stable Markdown', weight: '3' },
  { name: 'Front matter untouched', weight: '2' },
  { name: 'Inline link chips', weight: '2' },
  { name: 'Callouts with their own look', weight: '2' },
  { name: 'Beat headings with attribute lines', weight: '2' },
  { name: 'Spell check and IME', weight: '2' },
  { name: 'Long documents', weight: '1' },
  { name: 'Svelte 5 integration', weight: '2' },
  { name: 'Licence', weight: '–' },
  { name: 'Maintenance', weight: '2' },
] as const;
const LICENCE = 'Licence';
const TOTAL = 'Weighted total';
const BYTES = 'Byte-stable Markdown';
const FRONT_MATTER = 'Front matter untouched';
/**
 * Equal totals: spec §4.4 prefers the higher byte-stability score; controller ruling 11 (P10
 * ledger) continues down the criteria in weight order, then table order. The first criterion
 * with different scores decides.
 */
const TIE_ORDER = [
  BYTES,
  FRONT_MATTER,
  'Inline link chips',
  'Callouts with their own look',
  'Beat headings with attribute lines',
  'Spell check and IME',
  'Svelte 5 integration',
  'Maintenance',
  'Long documents',
] as const;
/** The repo path of the measurement reports (spec §4.6–4.8); a measured cell cites at least one of them. */
const REPORTS = 'research/P10-prose-editor/report/';
/** Controller ruling 12 (P10 ledger): Jann's recorded decision on a tie that ruling 11 cannot break. */
const DECISION = /^Tie decided by Jann \((\d{4}-\d{2}-\d{2})\): (.+)$/;

const read = (): string => {
  if (!existsSync(COMPARISON)) throw new Error(`${COMPARISON} is missing`);
  return readFileSync(COMPARISON, 'utf8').replace(/\r\n/g, '\n');
};

/** The matrix as criterion → cells in candidate order, after checking its header. */
const matrixOf = (md: string): Map<string, string[]> => {
  const [header, ...rows] = parseTable(md, 'Matrix');
  expect(header).toEqual(['Criterion', 'Weight', ...CANDIDATES]);
  const byName = new Map<string, string[]>();
  for (const row of rows) byName.set(row[0] ?? '', row.slice(2));
  return byName;
};

/** The parsed cell of one criterion and candidate; fails the test if the cell is malformed. */
const cellOf = (matrix: Map<string, string[]>, criterion: string, candidate: number): Cell => {
  const raw = matrix.get(criterion)?.[candidate] ?? '';
  const cell = parseCell(raw);
  if (cell === undefined) throw new Error(`${criterion} / ${CANDIDATES[candidate]}: malformed cell "${raw}"`);
  return cell;
};

/** Weighted sum and knockout of one candidate, from the paper or the effective scores. */
const standing = (matrix: Map<string, string[]>, candidate: number, use: 'paper' | 'effective') => {
  const score = (criterion: string): string => {
    const cell = cellOf(matrix, criterion, candidate);
    return use === 'paper' ? cell.paper : (cell.measured ?? cell.paper);
  };
  let total = 0;
  for (const { name, weight } of CRITERIA) if (name !== LICENCE) total += Number(weight) * Number(score(name));
  const knockout = score(BYTES) === '0' || score(FRONT_MATTER) === '0' || score(LICENCE) === 'fail';
  const ties = TIE_ORDER.map((name) => Number(score(name)));
  return { name: CANDIDATES[candidate] as string, total, ties, knockout };
};
type Standing = ReturnType<typeof standing>;

/** Rank order: higher total first, then the tie order of spec §4.4 and controller ruling 11; 0 = undecided. */
const byRank = (a: Standing, b: Standing): number => {
  if (a.total !== b.total) return b.total - a.total;
  for (const [index] of TIE_ORDER.entries()) {
    const difference = (b.ties[index] ?? 0) - (a.ties[index] ?? 0);
    if (difference !== 0) return difference;
  }
  return 0;
};

/** The paragraphs (blank-line separated) of a text. */
const paragraphs = (text: string): string[] => text.split(/\n\s*\n/).map((p) => p.trim());

describe('comparison.md (CE-06.1)', () => {
  // AC: CE-06.1
  it('every criterion row exists once with its weight', () => {
    const [header, ...rows] = parseTable(read(), 'Matrix');
    expect(header).toEqual(['Criterion', 'Weight', ...CANDIDATES]);
    expect(rows.map((row) => row[0])).toEqual([...CRITERIA.map((c) => c.name), TOTAL]);
    for (const row of rows) expect(row, `row "${row[0]}"`).toHaveLength(2 + CANDIDATES.length);
    for (const [index, { name, weight }] of CRITERIA.entries()) expect(rows[index]?.[1], name).toBe(weight);
    expect(rows.at(-1)?.[1], TOTAL).toBe('');
  });

  // AC: CE-06.1
  it('every cell has a valid score and at least one source', () => {
    const matrix = matrixOf(read());
    let checked = 0;
    for (const { name } of CRITERIA) {
      for (const [index, candidate] of CANDIDATES.entries()) {
        const cell = cellOf(matrix, name, index);
        const allowed = name === LICENCE ? ['pass', 'fail'] : ['0', '1', '2', '3'];
        expect(allowed, `${name} / ${candidate}: paper score`).toContain(cell.paper);
        if (cell.measured !== undefined) expect(allowed, `${name} / ${candidate}: measured`).toContain(cell.measured);
        expect(cell.sources.length, `${name} / ${candidate}: sources`).toBeGreaterThan(0);
        checked += 1;
      }
    }
    expect(checked).toBe(50);
  });

  // AC: CE-06.1
  it('every cell cites at least one source that is not a researcher probe', () => {
    const md = read();
    const matrix = matrixOf(md);
    const probes = probeSources(md);
    for (const { name } of CRITERIA) {
      for (const [index, candidate] of CANDIDATES.entries()) {
        const cell = cellOf(matrix, name, index);
        const decisive = cell.sources.filter((id) => !probes.has(id));
        expect(decisive, `${name} / ${candidate} cites only probes: ${cell.sources.join(', ')}`).not.toHaveLength(0);
      }
    }
  });

  // AC: CE-06.1
  it('every cited source is in the source list with a date', () => {
    const md = read();
    const sources = parseSources(md);
    expect(sources.size).toBeGreaterThan(0);
    const body = md.slice(0, md.indexOf('\n## Sources'));
    const cited = citedSources(body);
    for (const id of cited) {
      const entry = sources.get(id);
      expect(entry, `${id} is cited but not listed`).toBeDefined();
      expect(Number.isNaN(Date.parse(entry?.date ?? '')), `${id} has no valid read date`).toBe(false);
    }
    const unused = [...sources.keys()].filter((id) => !cited.has(id));
    expect(unused, 'listed but never cited').toEqual([]);
  });

  // AC: CE-06.1
  it('weighted totals match the cells', () => {
    const matrix = matrixOf(read());
    const totals = matrix.get(TOTAL) ?? [];
    for (const [index, candidate] of CANDIDATES.entries()) {
      const { total, knockout } = standing(matrix, index, 'effective');
      expect(totals[index], candidate).toBe(knockout ? 'knockout' : String(total));
    }
  });

  // AC: CE-06.1
  it('the top two follow spec §4.4 and controller rulings 11 and 12 on paper scores', () => {
    const md = read();
    const matrix = matrixOf(md);
    const all = CANDIDATES.map((_, index) => standing(matrix, index, 'paper'));
    // Survivors first; if fewer than two survive, the best knocked-out candidates fill the gap.
    const ranked = [...all.filter((s) => !s.knockout).sort(byRank), ...all.filter((s) => s.knockout).sort(byRank)];
    const [first, second, third] = ranked;
    expect(section(md, 'Top two'), 'the "## Top two" section').toBeDefined();
    const lines = md.split('\n').filter((line) => line.startsWith('Top two: '));
    expect(lines).toHaveLength(1);
    const named = (lines[0] ?? '').slice('Top two: '.length).split(', ');

    // Every tie-order cell equal at the second place: ruling 11 cannot pick.
    const undecided = second !== undefined && third !== undefined && second.knockout === third.knockout
      && byRank(second, third) === 0;
    if (!undecided) {
      expect(new Set(named)).toEqual(new Set(ranked.slice(0, 2).map((s) => s.name)));
      return;
    }
    // Ruling 12: such a tie is settled by exactly one recorded decision of Jann, listed second.
    const tied = ranked.filter((s) => s.knockout === second.knockout && byRank(s, second) === 0).map((s) => s.name);
    // Only a line of the "## Top two" section counts, not a quote of it elsewhere in the file.
    const decisions = (section(md, 'Top two') ?? '').split('\n').map((line) => DECISION.exec(line))
      .filter((m): m is RegExpExecArray => m !== null);
    expect(decisions, `spec §4.4 and ruling 11 do not decide between ${tied.join(' and ')}, and no single `
      + '"Tie decided by Jann (<YYYY-MM-DD>): <candidate>" line settles it').toHaveLength(1);
    const [, date, pick] = decisions[0] ?? [];
    expect(Number.isNaN(Date.parse(date ?? '')), `the decision has no valid date: ${date}`).toBe(false);
    expect(tied, `the decision names "${pick}", which is not in the tie`).toContain(pick);
    expect(tied, 'ruling 12 settles the second slot; this tie includes the first place').not.toContain(first?.name);
    expect(named, 'the "Top two:" line lists the first place, then the decided candidate').toEqual([first?.name, pick]);
  });

  // AC: CE-06.1
  it('every candidate has a section that argues each of its scores with sources', () => {
    const md = read();
    const matrix = matrixOf(md);
    expect(md.split('\n').filter((line) => /^Stage: (paper|final)$/.test(line))).toHaveLength(1);
    for (const [index, candidate] of CANDIDATES.entries()) {
      const text = section(md, candidate);
      expect(text, `the "## ${candidate}" section`).toBeDefined();
      const parts = paragraphs(text ?? '');
      for (const { name } of CRITERIA) {
        const cell = cellOf(matrix, name, index);
        const score = cell.measured === undefined ? cell.paper : `${cell.paper}→${cell.measured}`;
        const lead = `**${name}: ${score}.**`;
        const argument = parts.find((p) => p.startsWith(lead));
        expect(argument, `${candidate}: no paragraph starting "${lead}"`).toBeDefined();
        expect(citedSources(argument ?? '').size, `${candidate} / ${name}: the argument cites no source`).toBeGreaterThan(0);
      }
    }
    expect(citedSources(section(md, 'Webviews') ?? '').size, 'the "## Webviews" section cites its facts').toBeGreaterThan(0);
  });

  // AC: CE-06.1
  it('at stage final the top two carry measured scores', () => {
    const md = read();
    const matrix = matrixOf(md);
    const sources = parseSources(md);
    expect(md.split('\n').filter((line) => line === 'Stage: final'), 'the "Stage: final" line').toHaveLength(1);
    const lines = md.split('\n').filter((line) => line.startsWith('Top two: '));
    expect(lines).toHaveLength(1);
    const topTwo = (lines[0] ?? '').slice('Top two: '.length).split(', ');
    expect(topTwo).toHaveLength(2);
    for (const [index, candidate] of CANDIDATES.entries()) {
      const top = topTwo.includes(candidate);
      for (const { name } of CRITERIA) {
        const cell = cellOf(matrix, name, index);
        const label = `${name} / ${candidate}`;
        // Spec §4.4: measured scores for the top two on criteria 1–8; maintenance keeps its paper score.
        if (!top || name === 'Maintenance') {
          expect(cell.measured, `${label}: no measured score`).toBeUndefined();
          continue;
        }
        if (name === LICENCE && cell.measured === undefined) continue;
        expect(cell.measured, `${label}: <paper>→<measured>`).toBeDefined();
        const reports = cell.sources.filter((id) => {
          const ref = sources.get(id)?.ref ?? '';
          return ref.startsWith(REPORTS) && existsSync(fileURLToPath(new URL(`../../../report/${ref.slice(REPORTS.length)}`, import.meta.url)));
        });
        expect(reports, `${label} cites no source whose ref is a file of ${REPORTS}: ${cell.sources.join(', ')}`).not.toHaveLength(0);
      }
    }
  });
});
