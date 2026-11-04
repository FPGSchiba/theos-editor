/**
 * Writes `report/licences.md` (licence of every package in each variant's dependency closure) and
 * `report/adapter-size.md` (non-blank lines per adapter file, grouped by feature). Run from `app/`:
 * `node scripts/licences.ts`. The data comes from `pnpm licenses list --json` and
 * `pnpm list --json --depth Infinity`; nothing is parsed by hand beyond that JSON.
 */
import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { variants } from '../src/harness/variants.ts';

const APP_DIR = fileURLToPath(new URL('../', import.meta.url));
const REPORT_DIR = fileURLToPath(new URL('../../report/', import.meta.url));

/** Licences the project accepts without a second look (plan, Task 17). */
const ALLOWED = new Set(['MIT', 'ISC', 'BSD-2-Clause', 'BSD-3-Clause', 'Apache-2.0', '0BSD', 'BlueOak-1.0.0', 'CC0-1.0']);

interface ListNode {
  version: string;
  deduped?: boolean;
  dependencies?: Record<string, ListNode>;
}
interface LicenceEntry {
  name: string;
  versions: string[];
  license: string;
}

/** Runs pnpm through corepack (bare pnpm is not on every PATH) and parses its JSON output. */
function pnpmJson<T>(args: string[]): T {
  const out = execFileSync('corepack', ['pnpm', ...args], {
    cwd: APP_DIR,
    encoding: 'utf8',
    maxBuffer: 256 * 1024 * 1024,
    shell: process.platform === 'win32',
  });
  return JSON.parse(out) as T;
}

/**
 * `name@version` of every package reachable from `roots` (direct dependencies of the project).
 * pnpm prints a package it already printed elsewhere as `deduped`, without its dependencies, so
 * the dependencies of an id come from its one fully printed occurrence.
 */
function closure(roots: string[], tree: Record<string, ListNode>): string[] {
  const expanded = new Map<string, Record<string, ListNode>>();
  const index = (nodes: Record<string, ListNode>): void => {
    for (const [name, node] of Object.entries(nodes)) {
      if (node.deduped !== true) expanded.set(`${name}@${node.version}`, node.dependencies ?? {});
      index(node.dependencies ?? {});
    }
  };
  index(tree);

  const found = new Set<string>();
  const walk = (name: string, node: ListNode): void => {
    const id = `${name}@${node.version}`;
    if (found.has(id)) return;
    found.add(id);
    for (const [child, childNode] of Object.entries(expanded.get(id) ?? {})) walk(child, childNode);
  };
  for (const root of roots) {
    const node = tree[root];
    if (node === undefined) throw new Error(`${root} is not a direct dependency of the project`);
    walk(root, node);
  }
  return [...found].sort();
}

function licencesReport(): string {
  const [project] = pnpmJson<Array<{ dependencies?: Record<string, ListNode>; devDependencies?: Record<string, ListNode> }>>([
    'list',
    '--json',
    '--depth',
    'Infinity',
  ]);
  const tree = { ...project?.devDependencies, ...project?.dependencies };
  const byLicence = pnpmJson<Record<string, LicenceEntry[]>>(['licenses', 'list', '--json']);
  const licenceOf = new Map<string, string>();
  for (const [licence, entries] of Object.entries(byLicence)) {
    for (const entry of entries) for (const version of entry.versions) licenceOf.set(`${entry.name}@${version}`, licence);
  }

  const flagged = new Map<string, string>();
  const sections: string[] = [];
  for (const variant of variants) {
    const ids = closure(variant.packages, tree);
    const rows = ids.map((id) => {
      const licence = licenceOf.get(id) ?? 'unknown';
      if (!ALLOWED.has(licence)) flagged.set(id, licence);
      return `| ${id} | ${licence}${ALLOWED.has(licence) ? '' : ' (flagged)'} |`;
    });
    sections.push(
      `## ${variant.name}`,
      '',
      `Direct packages: ${variant.packages.length === 0 ? 'none (built into the browser)' : variant.packages.join(', ')}.`,
      '',
      `Closure: ${ids.length} packages.`,
      '',
      '| package | licence |',
      '|---|---|',
      ...rows,
      '',
    );
  }
  return [
    '# licences report',
    '',
    'The licence of every package in the dependency closure of each variant (`VariantInfo.packages` and everything',
    'they depend on, from `pnpm list --json --depth Infinity`), with the licences from `pnpm licenses list --json`.',
    `Allowed without a flag: ${[...ALLOWED].join(', ')}.`,
    '',
    '## Flags',
    '',
    ...(flagged.size === 0 ? ['No package outside the allowed licences.'] : [...flagged].map(([id, l]) => `- ${id}: ${l}`)),
    '',
    ...sections,
  ].join('\n');
}

/** The feature a file name stands for; the names say the concern. */
const FEATURE_OF: Record<string, string> = {
  links: 'links', linkview: 'links', linkkeys: 'links', popover: 'links',
  callouts: 'callouts', calloutview: 'callouts', calloutraw: 'callouts', calloutlook: 'callouts', devwords: 'callouts', notestate: 'callouts',
  beats: 'beats', beatrow: 'beats', beatview: 'beats', beatparam: 'beats', beatraw: 'beats',
  livepreview: 'live preview', rawlines: 'live preview',
  typing: 'input and keys', spellcheck: 'input and keys', keymap: 'input and keys',
  history: 'history', modes: 'modes', source: 'modes', external: 'external changes', frontmatter: 'front matter', edits: 'scripted edits',
  blocks: 'save mapping', changes: 'save mapping', edit: 'save mapping', fallback: 'save mapping', offsets: 'save mapping',
  sourcebody: 'save mapping', spanedit: 'save mapping', spans: 'save mapping', verify: 'save mapping',
  index: 'core', context: 'core', theme: 'core', schema: 'core', markdown: 'core', body: 'core', positions: 'core',
};

const nonBlank = (path: string): number => readFileSync(path, 'utf8').split('\n').filter((line) => line.trim() !== '').length;

function adapterSizeReport(): string {
  const root = join(APP_DIR, 'src', 'adapters');
  const out = [
    '# adapter size report',
    '',
    'Non-blank lines per file of every `src/adapters/<folder>/`, grouped by feature from the file name.',
    '`NOTES.md` and tests (`*.test.ts`) are excluded. `prosemirror-best-effort` is built on `prosemirror`, so the',
    'best-effort variant costs both folders; `prosemirror:stock` costs only `prosemirror`.',
    '',
  ];
  const folders = readdirSync(root, { withFileTypes: true }).filter((e) => e.isDirectory()).map((e) => e.name).sort();
  for (const folder of folders) {
    const files = readdirSync(join(root, folder)).filter((f) => f.endsWith('.ts') && !f.endsWith('.test.ts')).sort();
    const groups = new Map<string, Array<[string, number]>>();
    for (const file of files) {
      const feature = FEATURE_OF[basename(file, '.ts')] ?? 'other';
      groups.set(feature, [...(groups.get(feature) ?? []), [file, nonBlank(join(root, folder, file))]]);
    }
    const total = [...groups.values()].flat().reduce((sum, [, n]) => sum + n, 0);
    out.push(`## ${folder}`, '', `Total: ${total} non-blank lines in ${files.length} files.`, '', '| feature | lines | files |', '|---|---|---|');
    for (const [feature, list] of [...groups].sort(([a], [b]) => a.localeCompare(b))) {
      out.push(`| ${feature} | ${list.reduce((s, [, n]) => s + n, 0)} | ${list.map(([f, n]) => `${f} (${n})`).join(', ')} |`);
    }
    out.push('');
  }
  return out.join('\n');
}

writeFileSync(join(REPORT_DIR, 'licences.md'), licencesReport());
writeFileSync(join(REPORT_DIR, 'adapter-size.md'), adapterSizeReport());
