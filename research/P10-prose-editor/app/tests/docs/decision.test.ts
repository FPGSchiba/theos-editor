import { existsSync, readFileSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import MarkdownIt from 'markdown-it';
import type { Token } from 'markdown-it';
import { describe, expect, it } from 'vitest';
import { variants } from '../../src/harness/variants.ts';
import { section, tableRows } from './markdown.ts';

/**
 * Document checks for the decision record of CE-06.3 (spec §4.9, §4.11; plan Task 19). They check
 * the form: every section, one recommendation out of the top two with its version, links that
 * resolve, and the row of the decision log. Whether the record is right is the question of the
 * evidence check (spec §4.10).
 */

const ROOT = fileURLToPath(new URL('../../../../../', import.meta.url));
const RECORD = resolve(ROOT, 'docs/decisions/0001-prose-editor.md');
const LOG = resolve(ROOT, 'docs/decisions/README.md');
const COMPARISON = resolve(ROOT, 'research/P10-prose-editor/comparison.md');
const RESEARCH_README = resolve(ROOT, 'research/P10-prose-editor/README.md');
const REPORTS = resolve(ROOT, 'research/P10-prose-editor/report');

/** The sections of spec §4.9, in its order; the record has these and no others. */
const SECTIONS = ['Status', 'Decision', 'Why', 'Consequences for P12', 'Alternative', 'If rejected'];

const read = (path: string): string => {
  if (!existsSync(path)) throw new Error(`${path} is missing`);
  return readFileSync(path, 'utf8').replace(/\r\n/g, '\n');
};

/** The `Top two:` line of the comparison: the two candidates the record chooses from. */
const topTwo = (): string[] => {
  const lines = read(COMPARISON).split('\n').filter((line) => line.startsWith('Top two: '));
  expect(lines, 'the "Top two:" line of comparison.md').toHaveLength(1);
  const names = (lines[0] ?? '').slice('Top two: '.length).split(', ');
  expect(names).toHaveLength(2);
  return names;
};

/** The names of `names` that the text mentions. */
const mentioned = (text: string, names: string[]): string[] => names.filter((name) => text.includes(name));

/** The one candidate of the top two that the record's `## Decision` section names. */
const recommended = (record: string): string => {
  const names = mentioned(section(record, 'Decision') ?? '', topTwo());
  expect(names, 'the Decision section names exactly one of the top two').toHaveLength(1);
  return names[0] ?? '';
};

/**
 * Every link target (`href`, and `src` of images) of a Markdown text, as markdown-it parses it.
 * `document` is the whole file a section comes from, so that its reference definitions
 * (`[name]: path`) resolve the section's reference links.
 */
const links = (md: string, document = md): string[] => {
  const markdown = new MarkdownIt();
  const env = {};
  markdown.parse(document, env);
  const targets: string[] = [];
  const visit = (tokens: Token[]): void => {
    for (const token of tokens) {
      const target = token.type === 'link_open' ? token.attrGet('href') : token.type === 'image' ? token.attrGet('src') : null;
      if (target !== null) targets.push(target);
      if (token.children !== null) visit(token.children);
    }
  };
  visit(markdown.parse(md, env));
  return targets;
};

/** A link with a scheme (`https:`, `mailto:`) or a protocol-relative one is not a repo path. */
const isRelative = (target: string): boolean => !/^[a-z][a-z0-9+.-]*:/i.test(target) && !target.startsWith('//');

/**
 * The relative links of a document that do not resolve to a file or folder of the repo. A
 * fragment (`#…`) is reported too: this check cannot resolve one, so the documents link files
 * and name the section in their text.
 */
const brokenLinks = (path: string): string[] => {
  const broken: string[] = [];
  for (const target of links(read(path)).filter(isRelative)) {
    if (target.includes('#')) {
      broken.push(`${target} (a fragment; link the file and name the section)`);
      continue;
    }
    if (!existsSync(resolve(dirname(path), decodeURIComponent(target)))) broken.push(target);
  }
  return broken;
};

describe('the decision record (CE-06.3)', () => {
  // AC: CE-06.3
  it('the record has every section', () => {
    const record = read(RECORD);
    const headings = record.split('\n').filter((line) => line.startsWith('## ')).map((line) => line.slice(3));
    expect(headings, 'the sections of spec §4.9, in order').toEqual(SECTIONS);
    for (const heading of SECTIONS) {
      expect((section(record, heading) ?? '').trim(), `the "## ${heading}" section is empty`).not.toBe('');
    }
    const status = (section(record, 'Status') ?? '').split('\n').find((line) => line.trim() !== '') ?? '';
    expect(status, 'the status line').toMatch(/^proposed\b/);
  });

  // AC: CE-06.3
  it('the record recommends exactly one of the top two', () => {
    const record = read(RECORD);
    const pick = recommended(record);
    const [other] = topTwo().filter((name) => name !== pick);
    expect(mentioned(section(record, 'Alternative') ?? '', [other ?? '']), `the Alternative section names ${other}`)
      .toHaveLength(1);
    // Spec §4.9: the decision names the version researched, the one the prototype measured.
    const researched = variants.filter((v) => v.baseline !== true && pick.startsWith(v.component)).map((v) => v.version);
    expect(researched.length, `a variant of src/harness/variants.ts for ${pick}`).toBeGreaterThan(0);
    for (const version of new Set(researched)) {
      expect(section(record, 'Decision') ?? '', `the Decision section names version ${version}`).toContain(version);
    }
  });

  // AC: CE-06.3
  it('every relative link in the record resolves', () => {
    expect(brokenLinks(RECORD), 'links of docs/decisions/0001-prose-editor.md').toEqual([]);
    // Spec §4.9: the evidence that decided is linked into research/P10-prose-editor/.
    const record = read(RECORD);
    const why = links(section(record, 'Why') ?? '', record).filter(isRelative)
      .map((target) => relative(REPORTS, resolve(dirname(RECORD), decodeURIComponent(target))))
      .filter((path) => path !== '' && !path.startsWith('..'));
    expect(why.length, 'the Why section links at least one report of research/P10-prose-editor/report/')
      .toBeGreaterThan(0);
  });

  // AC: CE-06.3
  it('every relative link in the research README resolves', () => {
    expect(brokenLinks(RESEARCH_README), 'links of research/P10-prose-editor/README.md').toEqual([]);
  });

  // AC: CE-06.3
  it('the decision log row names the record', () => {
    const [header, ...rows] = tableRows(read(LOG));
    expect(header, 'the decision log table').toEqual(['Date', 'Decision']);
    const open = rows.filter(([date, text]) => date === 'open' && /prose editor/i.test(text ?? ''));
    expect(open, 'an "open" row for the prose editor').toEqual([]);
    const naming = rows.filter(([, text]) => links(text ?? '').includes('0001-prose-editor.md'));
    expect(naming, 'rows linking 0001-prose-editor.md').toHaveLength(1);
    const [date, text] = naming[0] ?? [];
    expect(date, 'the date of the row').toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(Number.isNaN(Date.parse(date ?? '')), `the row's date ${date} is a date`).toBe(false);
    expect(text, 'the row names the component of the record').toContain(recommended(read(RECORD)));
  });
});
